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
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

use hyper_util::rt::TokioIo;
#[cfg(not(target_env = "msvc"))]
use tikv_jemallocator::Jemalloc;

use hyper::server::conn::http1;
use hyper::service::service_fn;
use log::{error, info};
use tokio::net::TcpListener;
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

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

#[derive(Clone)]
/// An Executor that uses the tokio runtime.
pub struct TokioExecutor;

impl<F> hyper::rt::Executor<F> for TokioExecutor
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    fn execute(&self, fut: F) {
        tokio::task::spawn(fut);
    }
}

pub async fn run(config: Configuration) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = SocketAddr::new(IpAddr::V4(config.bind_address), config.port);
    let listener = TcpListener::bind(addr).await?;
    let config = Arc::new(config);
    let context = Arc::new(Context::new(config.clone()).await?);

    info!("Proxy online. Listening on http://{}", addr);
    loop {
        let (stream, _remote_addr) = listener.accept().await?;
        let io = TokioIo::new(stream);
        let ctx = context.clone();
        let cfg = config.clone();

        let service = service_fn(move |req| route(ctx.clone(), cfg.clone(), req));

        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new().serve_connection(io, service).await {
                error!("Error serving connection {:?}", err);
            }
        });
    }
}

fn main() {
    print_banner();
    logging_init();

    info!("NFT Image Proxy");
    info!(
        "Version:{}, Git:{}",
        built_info::PKG_VERSION,
        built_info::GIT_VERSION.unwrap_or_default()
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
