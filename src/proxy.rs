extern crate bb8_postgres;
extern crate tokio_postgres;

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
use std::fs::File;
use std::io::prelude::*;

use chrono::{DateTime, Utc};

use hyper::{Body, Method, Request, Response, StatusCode};
use log::{error, warn};
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
}

impl Proxy {
    pub async fn new(config: &Configuration) -> Result<Proxy, GenericError> {
        let database = Database::new(config).await?;
        let moderation_provider = ModerationService::get_provider(config)?;
        Ok(Proxy {
            config: config.clone(),
            database: database,
            start_time: Utc::now(),
            moderation_provider: moderation_provider,
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
        (&Method::OPTIONS, _) => Ok(Response::builder()
            .header(hyper::header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
            .header(
                hyper::header::ACCESS_CONTROL_ALLOW_METHODS,
                "POST, GET, OPTIONS",
            )
            .header(hyper::header::ACCESS_CONTROL_ALLOW_CREDENTIALS, "true")
            .header(hyper::header::ACCESS_CONTROL_MAX_AGE, 86400)
            .header(hyper::header::ACCESS_CONTROL_ALLOW_HEADERS, "apikey")
            .status(StatusCode::OK)
            .body(Body::default())
            .unwrap_or_default()),
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
        (&Method::GET, "/admin") => get_file("index.html").await,
        (&Method::GET, "/metrics") if proxy.config.metrics_enabled => {
            metrics(&proxy.start_time).await
        }
        (&Method::GET, path) => get_file(path).await,
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
    let response = Response::builder()
        .header(hyper::header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .body(Body::from(result))
        .unwrap();
    Ok(response)
}

async fn get_file(path: &str) -> Result<Response<Body>, GenericError> {
    match File::open(format!("./ui/{}", path)) {
        Ok(mut f) => {
            let mut source = Vec::new();
            match f.read_to_end(&mut source) {
                Ok(_) => Ok(Response::builder()
                    .header(hyper::header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                    .body(Body::from(source))
                    .unwrap_or_default()),
                Err(e) => {
                    error!("Unable to read {}", path);
                    metrics::ERRORS.inc();
                    Err(Box::new(e))
                }
            }
        }
        Err(_) => {
            warn!("Unable to find {}", path);
            Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::default())
                .unwrap_or_default())
        }
    }
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
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(hyper::header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .body(Body::from(output.unwrap_or(String::default())))
        .unwrap_or_default())
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
