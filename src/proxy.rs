extern crate bb8_postgres;
extern crate tokio_postgres;

use crate::cache::{get_cache, Cache};
use crate::config::{Cors, SecurityConfig};
use crate::db::{DatabaseFactory, DatabaseProvider};
use crate::dns::StandardDnsResolver;
use crate::document::Document;

use crate::http::filters::private_network::PrivateNetworkFilter;
use crate::http::filters::UriFilter;
use crate::http::{HttpClientFactory, HttpClientWrapper};
use crate::metrics;
use crate::metrics::REGISTRY;
use crate::moderation::{ModerationProvider, ModerationService};
use crate::rpc::responses::{
    DescribeResponse, FetchResponse, ReportDescribeResponse, ReportResponse, RpcStatus,
};
use crate::rpc::*;
use crate::{built_info, rpc::error::Errors};
use crate::{
    config::Configuration,
    rpc::{
        requests::{DescribeRequest, FetchRequest, MethodHeader, ReportRequest, RpcMethods},
        responses::Info,
    },
};

use chrono::Utc;

use hyper::header::HeaderValue;
use hyper::{Body, Method, Request, Response, StatusCode};
use log::{debug, error};
use prometheus::Encoder;
use rust_embed::RustEmbed;
use serde::de;
use serde_json;
use std::{borrow::Borrow, sync::Arc};
use uuid::Uuid;
type GenericError = Box<dyn std::error::Error + Send + Sync>;

#[deprecated(
    since = "1.4.0",
    note = "UI Dashboard will be removed starting version 2.0"
)]
#[derive(RustEmbed)]
#[folder = "dashboard-ui/build"]
struct Asset;

pub struct Context {
    pub database: Box<dyn DatabaseProvider + Send + Sync>,
    pub moderation_provider: Box<dyn ModerationProvider + Send + Sync>,
    pub http_client_provider: HttpClientWrapper,
    pub cache: Option<Box<dyn Cache<String, Document> + Send + Sync>>,
}

impl Context {
    pub async fn new(config: Arc<Configuration>) -> Result<Context, GenericError> {
        let database = DatabaseFactory::get_provider(&config.database).await?;
        let moderation_provider = ModerationService::get_provider(&config)?;
        let dns_resolver = StandardDnsResolver {};
        //TODO: Add more filters here
        let uri_filters: Vec<Box<dyn UriFilter + Send + Sync>> =
            vec![Box::new(PrivateNetworkFilter::new(Box::new(dns_resolver)))];
        let http_client = HttpClientFactory::get_provider(
            config.ipfs.clone(),
            config.max_document_size,
            uri_filters,
            config.timeout,
        );
        Ok(Context {
            database,
            moderation_provider,
            http_client_provider: http_client,
            cache: get_cache(&config.cache_config),
        })
    }
}

pub fn authenticate(security_config: &SecurityConfig, req: &Request<Body>, req_id: &Uuid) -> bool {
    match req.headers().get("apikey") {
        Some(h) => match String::from_utf8(h.as_bytes().to_vec()) {
            Ok(key) => {
                if let Some(api_key) = security_config.api_keys.iter().find(|k| k.key.eq(&key)) {
                    metrics::API_KEY_USAGE
                        .with_label_values(&[api_key.name.as_str()])
                        .inc();
                    debug!("Authorized key_name={}, req_id={}", &api_key.name, req_id);
                    true
                } else {
                    debug!("Authorization failed for req_id={}", req_id);
                    false
                }
            }
            Err(e) => {
                error!("Unable to convert api key header to string, reason={}", e);
                false
            }
        },
        None => false,
    }
}

fn with_cors(
    mut response: Response<Body>,
    cors_config: &Cors,
) -> Result<Response<Body>, GenericError> {
    // TODO: Cors header configs should be validated at startup
    let cors_origin: HeaderValue = cors_config.origin.to_owned().parse()?;
    response
        .headers_mut()
        .insert(hyper::header::ACCESS_CONTROL_ALLOW_ORIGIN, cors_origin);
    Ok(response)
}

fn empty_response(code: StatusCode) -> Result<Response<Body>, GenericError> {
    Ok(Response::builder()
        .status(code)
        .body(Body::default())
        .unwrap_or_default())
}

pub async fn route(
    ctx: Arc<Context>,
    config: Arc<Configuration>,
    req: Request<Body>,
) -> Result<Response<Body>, GenericError> {
    metrics::HITS.inc();
    let response_time_start = Utc::now().timestamp_millis();
    let cors_config = config.cors.clone();
    let req_id = Uuid::new_v4();

    let result = match (req.method(), req.uri().path()) {
        (&Method::POST, "/") => {
            if authenticate(&config.security, req.borrow(), &req_id) {
                rpc(ctx, req, req_id).await.or_else(|e| {
                    metrics::ERRORS.inc();
                    let rpc_error = e.to_rpc_error(&req_id);
                    metrics::ERRORS_RPC
                        .with_label_values(&[rpc_error.code.to_string().as_str()])
                        .inc();
                    Ok(e.to_response(&req_id))
                })
            } else {
                empty_response(StatusCode::FORBIDDEN)
            }
        }
        (&Method::GET, "/") => empty_response(StatusCode::OK),
        (&Method::GET, "/info") => info().await,
        (&Method::GET, "/metrics") if config.metrics_enabled => metrics(ctx).await,
        (&Method::GET, path) if config.dashboard_enabled => {
            let file = Asset::get(&path[1..]);
            match file {
                Some(f) => Ok(Response::builder()
                    .status(StatusCode::OK)
                    .body(Body::from(f.data.into_owned()))
                    .unwrap()),
                None => empty_response(StatusCode::NOT_FOUND),
            }
        }
        _ => empty_response(StatusCode::OK),
    };

    metrics::API_RESPONSE_TIME
        .with_label_values(&["overall"])
        .observe((Utc::now().timestamp_millis() - response_time_start) as f64);

    result
        .or_else(|e| {
            metrics::ERRORS.inc();
            error!("Unknown error, reason:{}", e);
            Ok(Errors::InternalError.to_response(&Uuid::new_v4()))
        })
        .and_then(|r| with_cors(r, &cors_config))
}

async fn info() -> Result<Response<Body>, GenericError> {
    let info = Info {
        package_version: built_info::PKG_VERSION,
        git_version: built_info::GIT_VERSION.unwrap_or("unknown"),
    };
    let result = serde_json::to_string(&info).unwrap_or_default();
    Ok(Response::builder()
        .status(hyper::StatusCode::OK)
        .header(hyper::header::CONTENT_TYPE, "application/json")
        .body(Body::from(result))
        .unwrap_or_default())
}

async fn metrics(ctx: Arc<Context>) -> Result<Response<Body>, GenericError> {
    let encoder = prometheus::TextEncoder::new();
    let mut buffer = Vec::new();

    if let Some(cache) = &ctx.cache {
        cache.gather_metrics(&metrics::CACHE_METRICS);
    }

    match encoder.encode(&REGISTRY.gather(), &mut buffer) {
        Ok(_) => match String::from_utf8(buffer) {
            Ok(output) => Response::builder()
                .body(Body::from(output))
                .or_else(|_| empty_response(StatusCode::INTERNAL_SERVER_ERROR)),
            Err(e) => {
                error!("Unable to covert metrics to string, reason={}", e);
                empty_response(StatusCode::INTERNAL_SERVER_ERROR)
            }
        },
        Err(e) => {
            error!("Unable to encode metrics, reason={}", e);
            empty_response(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
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
                        let result = fetch(ctx, &req_id, &params.params).await?;
                        Ok(FetchResponse::to_response(
                            &params.params.response_type,
                            result.document,
                            result.moderation_status,
                            result.categories,
                            &req_id,
                        ))
                    }
                    RpcMethods::img_proxy_describe => {
                        let params = decode::<DescribeRequest>(&body)?;
                        let result = describe(ctx, &req_id, &params.params).await?;
                        Ok(DescribeResponse::to_response(
                            RpcStatus::Ok,
                            result,
                            &req_id,
                        ))
                    }
                    RpcMethods::img_proxy_report => {
                        let params = decode::<ReportRequest>(&body)?;
                        let _ = report(ctx, &req_id, &params.params).await?;
                        Ok(ReportResponse::to_response(
                            RpcStatus::Ok,
                            &params.params.url,
                            &req_id,
                        ))
                    }
                    RpcMethods::img_proxy_describe_report => {
                        let results = describe_report(ctx, &req_id).await?;
                        Ok(ReportDescribeResponse::to_response(
                            RpcStatus::Ok,
                            results,
                            &req_id,
                        ))
                    }
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

#[cfg(test)]
mod tests {
    use crate::config::ApiKey;

    use super::*;

    fn build_request(key: &str) -> Request<Body> {
        Request::builder()
            .header("apikey", key)
            .body(Body::from(""))
            .unwrap()
    }

    #[test]
    fn test_authenticate_fn() {
        let api_keys = vec![
            ApiKey {
                name: "test_key1".to_string(),
                key: "1234".to_string(),
            },
            ApiKey {
                name: "test_key2".to_string(),
                key: "abcd".to_string(),
            },
        ];
        let security_config = SecurityConfig { api_keys };
        let req_id = Uuid::new_v4();

        // Key in api_key list
        let req = build_request("1234");
        assert!(authenticate(&security_config, &req, &req_id));

        // Key in api_key list
        let req = build_request("abcd");
        assert!(authenticate(&security_config, &req, &req_id));

        // Key not in api_key list
        let req = build_request("0000");
        assert!(!authenticate(&security_config, &req, &req_id));

        // No header specified
        let req = Request::builder().body(Body::from("")).unwrap();
        assert!(!authenticate(&security_config, &req, &req_id));
    }
}
