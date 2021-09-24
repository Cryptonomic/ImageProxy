extern crate bb8_postgres;
extern crate tokio_postgres;

use crate::cache::{get_cache, Cache};
use crate::dns::StandardDnsResolver;
use crate::document::Document;
use crate::http::HttpClient;

use crate::http::filters::private_network::PrivateNetworkFilter;
use crate::http::filters::UriFilter;
use crate::metrics;
use crate::metrics::REGISTRY;
use crate::rpc::*;
use crate::{built_info, rpc::error::Errors};
use crate::{
    config::Configuration,
    rpc::{
        requests::{DescribeRequest, FetchRequest, MethodHeader, ReportRequest, RpcMethods},
        responses::Info,
    },
};
use crate::{
    db::Database,
    moderation::{ModerationProvider, ModerationService},
};

use chrono::Utc;

use hyper::{Body, Method, Request, Response, StatusCode};
use log::error;
use prometheus::Encoder;
use rust_embed::RustEmbed;
use serde::de;
use serde_json;
use std::{borrow::Borrow, sync::Arc};
use uuid::Uuid;
type GenericError = Box<dyn std::error::Error + Send + Sync>;

#[derive(RustEmbed)]
#[folder = "dashboard-ui/build"]
struct Asset;

pub struct Context {
    pub config: Configuration,
    pub database: Database,
    pub moderation_provider: Box<dyn ModerationProvider + Send + Sync>,
    pub http_client: HttpClient,
    pub cache: Option<Box<dyn Cache<String, Document> + Send + Sync>>,
}

impl Context {
    pub async fn new(config: Configuration) -> Result<Context, GenericError> {
        let database = Database::new(&config.database).await?;
        let moderation_provider = ModerationService::get_provider(&config)?;
        let dns_resolver = StandardDnsResolver {};
        //TODO: Add more filters here
        let uri_filters: Vec<Box<dyn UriFilter + Send + Sync>> =
            vec![Box::new(PrivateNetworkFilter::new(Box::new(dns_resolver)))];
        let http_client = HttpClient::new(
            config.ipfs.clone(),
            config.max_document_size,
            uri_filters,
            config.timeout
        );
        Ok(Context{
            config: config.clone(),
            database,
            moderation_provider,
            http_client,
            cache: get_cache(&config.cache_config),
        })
    }
}

pub fn authenticate(api_keys: &[String], req: &Request<Body>) -> bool {
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

pub async fn route(ctx: Arc<Context>, req: Request<Body>) -> Result<Response<Body>, GenericError> {
    metrics::HITS.inc();
    metrics::ACTIVE_CLIENTS.inc();
    let response_time_start = Utc::now().timestamp_millis();
    let proxy_config = ctx.config.clone();
    let response = match (req.method(), req.uri().path()) {
        (&Method::POST, "/") => {
            if authenticate(&ctx.config.api_keys.clone(), req.borrow()) {
                let req_id = Uuid::new_v4();
                rpc(ctx, req, req_id).await.or_else(|e| {
                    metrics::ERRORS.inc();
                    Ok(e.to_response(&req_id, &proxy_config))
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
        (&Method::GET, "/info") => info(&ctx.config).await,
        (&Method::GET, "/metrics") if ctx.config.metrics_enabled => metrics(ctx).await,
        (&Method::GET, path) if ctx.config.dashboard_enabled => {
            let file = Asset::get(&path[1..]);
            match file {
                Some(f) => Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header(
                        hyper::header::ACCESS_CONTROL_ALLOW_ORIGIN,
                        &ctx.config.cors.origin.to_owned(),
                    )
                    .body(Body::from(f.data.into_owned()))
                    .unwrap()),
                None => Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .header(
                        hyper::header::ACCESS_CONTROL_ALLOW_ORIGIN,
                        &ctx.config.cors.origin.to_owned(),
                    )
                    .body(Body::default())
                    .unwrap_or_default()),
            }
        }
        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header(
                hyper::header::ACCESS_CONTROL_ALLOW_ORIGIN,
                &ctx.config.cors.origin.to_owned(),
            )
            .body(Body::default())
            .unwrap_or_default()),
    };

    metrics::API_RESPONSE_TIME
        .with_label_values(&["overall"])
        .observe((Utc::now().timestamp_millis() - response_time_start) as f64);
    metrics::ACTIVE_CLIENTS.dec();

    response.or_else(|e| {
        metrics::ERRORS.inc();
        error!("Unknown error, reason:{}", e);
        Ok(Errors::InternalError.to_response(&Uuid::new_v4(), &proxy_config))
    })
}

async fn info(config: &Configuration) -> Result<Response<Body>, GenericError> {
    let info = Info {
        package_version: built_info::PKG_VERSION,
        git_version: built_info::GIT_VERSION.unwrap_or("unknown"),
    };
    let result = serde_json::to_string(&info).unwrap_or_default();
    Ok(Response::builder()
        .status(hyper::StatusCode::OK)
        .header(hyper::header::CONTENT_TYPE, "application/json")
        .header(
            hyper::header::ACCESS_CONTROL_ALLOW_ORIGIN,
            &config.cors.origin,
        )
        .body(Body::from(result))
        .unwrap_or_default())
}

async fn metrics(ctx: Arc<Context>) -> Result<Response<Body>, GenericError> {
    let encoder = prometheus::TextEncoder::new();
    let mut buffer = Vec::new();

    if let Some(cache) = &ctx.cache {
        cache.gather_metrics(&metrics::CACHE_METRICS);
    }

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
        .header(
            hyper::header::ACCESS_CONTROL_ALLOW_ORIGIN,
            &ctx.config.cors.origin,
        )
        .body(Body::from(output.unwrap_or_default()))
        .unwrap_or_else(|_| {
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(String::default().into())
                .unwrap_or_default()
        }))
}

fn decode<T: de::DeserializeOwned>(body: &[u8]) -> Result<T, Errors> {
    serde_json::from_slice::<T>(body).map_err(|e| {
        error!("Json decode error, reason:{}", e);
        Errors::JsonDecodeError
    })
}

async fn rpc(
    ctx: Arc<Context>,
    req: Request<Body>,
    req_id: Uuid,
) -> Result<Response<Body>, Errors> {
    match hyper::body::to_bytes(req.into_body()).await {
        Ok(body) => match decode::<MethodHeader>(&body) {
            Ok(header) if header.jsonrpc.eq_ignore_ascii_case(VERSION) => {
                let method = header.method;
                metrics::API_REQUESTS
                    .with_label_values(&[method.to_string().as_str()])
                    .inc();
                match method {
                    RpcMethods::img_proxy_fetch => {
                        let params = decode::<FetchRequest>(&body)?;
                        fetch(ctx, &req_id, &params.params).await
                    }
                    RpcMethods::img_proxy_describe => {
                        let params = decode::<DescribeRequest>(&body)?;
                        describe(ctx, &req_id, &params.params).await
                    }
                    RpcMethods::img_proxy_report => {
                        let params = decode::<ReportRequest>(&body)?;
                        report(ctx, &req_id, &params.params).await
                    }
                    RpcMethods::img_proxy_describe_report => describe_report(ctx, &req_id).await,
                }
            }
            Ok(_) => Err(Errors::InvalidRpcVersionError),
            Err(e) => Err(e),
        },
        Err(e) => {
            error!("Unable to obtain request body, reason:{}", e);
            Err(Errors::InternalError)
        }
    }
}
