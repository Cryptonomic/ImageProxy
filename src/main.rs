#![feature(ip)]
extern crate tokio;

pub mod aws;
pub mod cache;
pub mod config;
pub mod db;
pub mod dns;
pub mod document;
pub mod http;
pub mod logging;
pub mod metrics;
pub mod moderation;
pub mod proxy;
pub mod rpc;
pub mod utils;

use std::{
    convert::Infallible,
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

use hyper::{
    service::{make_service_fn, service_fn},
    Server,
};
use log::info;
use tokio::runtime::Builder as TokioBuilder;

use crate::{
    config::Configuration,
    logging::logging_init,
    proxy::{route, Context},
};
use crate::{metrics::init_registry, utils::print_banner};

pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

pub async fn run(config: Configuration) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = SocketAddr::new(IpAddr::V4(config.bind_address), config.port);
    let proxy = Arc::new(Context::new(config).await?);
    let service = make_service_fn(move |_| {
        let proxy = proxy.clone();
        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                let proxy = proxy.to_owned();
                route(proxy, req)
            }))
        }
    });

    let server = Server::bind(&addr).serve(service);
    info!("Proxy online. Listening on http://{}", addr);
    server.await?;
    Ok(())
}

fn main() {
    print_banner();
    logging_init();

    info!("NFT Image Proxy");
    info!(
        "Version:{}, Git:{}",
        built_info::PKG_VERSION,
        built_info::GIT_VERSION.unwrap()
    );

    info!("Loading configuration file");
    let config = Configuration::load().unwrap();

    info!("Initializing runtime");
    let runtime = TokioBuilder::new_multi_thread()
        .worker_threads(usize::from(config.workers))
        .enable_all()
        .thread_name("image_proxy")
        .build()
        .unwrap();

    info!("Starting metrics collection");
    init_registry();

    info!("Starting proxy server");
    runtime.block_on(run(config)).unwrap();
}
