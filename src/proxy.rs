extern crate bb8_postgres;
extern crate tokio_postgres;

use crate::dns::StandardDnsResolver;
use crate::http::HttpClient;

use crate::http::filters::private_network_filter::PrivateNetworkFilter;
use crate::metrics;
use crate::metrics::REGISTRY;
use crate::rpc::*;
use crate::{built_info, rpc::responses::StatusCodes};
use crate::{
    config::Configuration,
    rpc::{
        requests::{DescribeRequest, FetchRequest, MethodHeader, ReportRequest, RpcMethods},
        responses::{Info, RpcError},
    },
};
use crate::{
    db::Database,
    moderation::{ModerationProvider, ModerationService},
};

use chrono::{DateTime, Utc};

use hyper::{Body, Method, Request, Response, StatusCode};
use log::error;
use procfs::process::Process;
use prometheus::Encoder;
use serde::de;
use serde_json;
use std::{borrow::Borrow, sync::Arc};
use uuid::Uuid;

type GenericError = Box<dyn std::error::Error + Send + Sync>;

pub struct Proxy {
    pub config: Configuration,
    pub database: Database,
    pub start_time: DateTime<Utc>,
    pub moderation_provider: Box<dyn ModerationProvider + Send + Sync>,
    pub http_client: HttpClient,
}

impl Proxy {
    pub async fn new(config: &Configuration) -> Result<Proxy, GenericError> {
        let database = Database::new(config).await?;
        let moderation_provider = ModerationService::get_provider(config)?;
        let dns_resolver = StandardDnsResolver {};
        //TODO: Add more filters here
        let uri_filters = vec![PrivateNetworkFilter::new(
            false,
            vec![],
            Box::new(dns_resolver.clone()),
        )];
        let http_client = HttpClient::new(
            config.ipfs.clone(),
            Box::new(dns_resolver),
            config.max_document_size,
            uri_filters,
        );
        Ok(Proxy {
            config: config.clone(),
            database: database,
            start_time: Utc::now(),
            moderation_provider: moderation_provider,
            http_client: http_client,
        })
    }
}

pub fn authenticate(api_keys: &Vec<String>, req: &Request<Body>) -> bool {
    match req.headers().get("apikey") {
        Some(h) => match String::from_utf8(h.as_bytes().to_vec()) {
            Ok(key) => api_keys.contains(&key),
            Err(e) => {
                error!("Unable to convert api key header to string, reason={}", e);
                false
            }
        },
        None => false,
    }
}

pub async fn route(proxy: Arc<Proxy>, req: Request<Body>) -> Result<Response<Body>, GenericError> {
    metrics::HITS.inc();
    metrics::ACTIVE_CLIENTS.inc();
    let response_time_start = Utc::now();

    let response = match (req.method(), req.uri().path()) {
        (&Method::POST, "/") => {
            if authenticate(&proxy.config.api_keys.clone(), req.borrow()) {
                rpc(proxy, req).await.or_else(|e| {
                    metrics::ERRORS.inc();
                    Ok(RpcError::to_response(e))
                })
            } else {
                Ok(Response::builder()
                    .status(StatusCode::FORBIDDEN)
                    .body(Body::default())
                    .unwrap_or_default())
            }
        }
        (&Method::GET, "/") => Ok(Response::builder()
            .status(StatusCode::OK)
            .body(Body::default())
            .unwrap_or_default()),
        (&Method::GET, "/info") => info().await,
        (&Method::GET, "/metrics") if proxy.config.metrics_enabled => {
            metrics(&proxy.start_time).await
        }
        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::default())
            .unwrap_or_default()),
    };

    metrics::API_RESPONSE_TIME
        .with_label_values(&["overall"])
        .observe((Utc::now().time() - response_time_start.time()).num_milliseconds() as f64);
    metrics::ACTIVE_CLIENTS.dec();

    response.or_else(|e| {
        metrics::ERRORS.inc();
        error!("Unknown error, reason:{}", e);
        Ok(RpcError::to_response(StatusCodes::InternalError))
    })
}

async fn info() -> Result<Response<Body>, GenericError> {
    let info = Info {
        package_version: built_info::PKG_VERSION,
        git_version: built_info::GIT_VERSION.unwrap_or("unknown"),
    };
    let result = serde_json::to_string(&info).unwrap_or_default().to_owned();
    Ok(Response::builder()
        .status(hyper::StatusCode::OK)
        .header(hyper::header::CONTENT_TYPE, "application/json")
        .body(Body::from(result))
        .unwrap_or_default())
}

async fn metrics(service_start_time: &DateTime<Utc>) -> Result<Response<Body>, GenericError> {
    let encoder = prometheus::TextEncoder::new();
    let mut buffer = Vec::new();
    let uptime = Utc::now().time() - service_start_time.time();
    metrics::UPTIME.reset();
    metrics::UPTIME.inc_by(uptime.num_seconds());

    Process::myself()
        .ok()
        .map(|p| p.status().ok())
        .flatten()
        .map(|status| {
            status.vmsize.map(|s| metrics::MEM_VIRT.set(s as i64));
            status.vmrss.map(|s| metrics::MEM_RSS.set(s as i64));
        });

    let encode_result = encoder.encode(&REGISTRY.gather(), &mut buffer);
    if encode_result.is_err() {
        return Ok(Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(String::default().into())
            .unwrap_or_default());
    }

    let output = String::from_utf8(buffer.clone());
    buffer.clear();
    Ok(Response::new(Body::from(
        output.unwrap_or(String::default()),
    )))
}

fn decode<T: de::DeserializeOwned>(body: &[u8]) -> Result<T, StatusCodes> {
    match serde_json::from_slice::<T>(&body) {
        Ok(o) => Ok(o),
        Err(e) => {
            error!("Json decode error, reason:{}", e);
            Err(StatusCodes::JsonDecodeError)
        }
    }
}

async fn rpc(proxy: Arc<Proxy>, req: Request<Body>) -> Result<Response<Body>, StatusCodes> {
    metrics::API_REQUESTS.inc();
    match hyper::body::to_bytes(req.into_body()).await {
        Ok(body) => match decode::<MethodHeader>(&body) {
            Ok(header) if header.jsonrpc.eq_ignore_ascii_case(VERSION) => {
                let req_id = Uuid::new_v4();
                match header.method {
                    RpcMethods::img_proxy_fetch => {
                        let params = decode::<FetchRequest>(&body)?;
                        Methods::fetch(proxy, &req_id, &params.params).await
                    }
                    RpcMethods::img_proxy_describe => {
                        let params = decode::<DescribeRequest>(&body)?;
                        Methods::describe(proxy, &req_id, &params.params).await
                    }
                    RpcMethods::img_proxy_report => {
                        let params = decode::<ReportRequest>(&body)?;
                        Methods::report(proxy, &req_id, &params.params).await
                    }
                    RpcMethods::img_proxy_describe_report => {
                        Methods::describe_report(proxy, &req_id).await
                    }
                }
            }
            Ok(_) => Err(StatusCodes::InvalidRpcVersionError),
            Err(e) => Err(e),
        },
        Err(e) => {
            error!("Unable to obtain request body, reason:{}", e);
            Err(StatusCodes::InternalError)
        }
    }
}
