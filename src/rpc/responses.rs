use hyper::{Body, Response};
use log::error;
use serde::Serialize;
use uuid::Uuid;

use crate::moderation::{ModerationCategories, ModerationService};

use super::VERSION;

#[derive(Serialize)]
pub enum StatusCodes {
    Ok,
    InvalidRpcVersionError,
    InvalidRpcMethodError,
    InvalidUri,
    UnsupportedUriScheme,
    JsonDecodeError,
    DocumentBlocked,
    DocumentFetchFailed,
    DocumentNotFound,
    ModerationFailed,
    UnsupportedImageType,
    InternalError,
    BlockedHost
}

#[derive(Default, Serialize)]
pub struct Info {
    pub package_version: &'static str,
    pub git_version: &'static str,
}

#[derive(Serialize)]
pub struct ModerationResult {
    pub categories: Vec<ModerationCategories>,
}

#[derive(Serialize)]
pub struct FetchResponse {
    pub jsonrpc: String,
    pub code: StatusCodes,
    pub result: ModerationResult,
}

#[derive(Serialize)]
pub enum DocumentStatus {
    Blocked,
    Allowed,
    NeverSeen,
}

#[derive(Serialize)]
pub struct DescribeResult {
    pub url: String,
    pub status: DocumentStatus,
    pub categories: Vec<ModerationCategories>,
    pub provider: ModerationService,
}

#[derive(Serialize)]
pub struct DescribeResponse {
    pub jsonrpc: String,
    pub code: StatusCodes,
    pub result: Vec<DescribeResult>,
}

#[derive(Serialize)]
pub struct ReportResult {
    pub url: String,
    pub id: Uuid,
}

#[derive(Serialize)]
pub struct ReportResponse {
    pub jsonrpc: String,
    pub code: StatusCodes,
    pub result: ReportResult,
}

#[derive(Serialize)]
pub struct ReportDescribeResult {
    pub url: String,
    pub categories: Vec<ModerationCategories>,
    pub id: String,
    pub updated_at: String,
}
#[derive(Serialize)]
pub struct ReportDescribeResponse {
    pub jsonrpc: String,
    pub code: StatusCodes,
    pub result: Vec<ReportDescribeResult>,
}

// Rpc Error
#[derive(Serialize)]
pub struct RpcError {
    pub jsonrpc: String,
    pub code: StatusCodes,
}

impl FetchResponse {
    pub fn to_response(code: StatusCodes, categories: Vec<ModerationCategories>) -> Response<Body> {
        let result = FetchResponse {
            jsonrpc: String::from(VERSION),
            code,
            result: ModerationResult {
                categories: categories.clone(),
            },
        };

        match serde_json::to_string_pretty(&result) {
            Ok(body) => Response::builder()
                .status(hyper::StatusCode::OK)
                .header(hyper::header::CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .unwrap_or_default(),
            Err(e) => {
                error!("Error serializing fetch response, reason={}", e);
                RpcError::to_response(StatusCodes::InternalError)
            }
        }
    }
}

impl DescribeResponse {
    pub fn to_response(code: StatusCodes, describe_results: Vec<DescribeResult>) -> Response<Body> {
        let result = DescribeResponse {
            jsonrpc: String::from(VERSION),
            code,
            result: describe_results,
        };

        match serde_json::to_string_pretty(&result) {
            Ok(body) => Response::builder()
                .status(hyper::StatusCode::OK)
                .header(hyper::header::CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .unwrap_or_default(),
            Err(e) => {
                error!("Error serializing fetch response, reason={}", e);
                RpcError::to_response(StatusCodes::InternalError)
            }
        }
    }
}

impl ReportResponse {
    pub fn to_response(code: StatusCodes, url: &str, id: &Uuid) -> Response<Body> {
        let result = ReportResponse {
            jsonrpc: String::from(VERSION),
            code,
            result: ReportResult {
                url: String::from(url),
                id: id.clone(),
            },
        };

        match serde_json::to_string_pretty(&result) {
            Ok(body) => Response::builder()
                .status(hyper::StatusCode::OK)
                .header(hyper::header::CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .unwrap_or_default(),
            Err(e) => {
                error!("Error serializing fetch response, reason={}", e);
                RpcError::to_response(StatusCodes::InternalError)
            }
        }
    }
}

impl ReportDescribeResponse {
    pub fn to_response(code: StatusCodes, results: Vec<ReportDescribeResult>) -> Response<Body> {
        let result = ReportDescribeResponse {
            jsonrpc: String::from(VERSION),
            code,
            result: results,
        };

        match serde_json::to_string_pretty(&result) {
            Ok(body) => Response::builder()
                .status(hyper::StatusCode::OK)
                .header(hyper::header::CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .unwrap_or_default(),
            Err(e) => {
                error!("Error serializing fetch response, reason={}", e);
                RpcError::to_response(StatusCodes::InternalError)
            }
        }
    }
}

impl RpcError {
    pub fn to_response(code: StatusCodes) -> Response<Body> {
        let result = RpcError {
            jsonrpc: String::from(VERSION),
            code,
        };
        let body = serde_json::to_string_pretty(&result)
            .unwrap_or_default()
            .clone();
        Response::builder()
            .status(hyper::StatusCode::OK)
            .header(hyper::header::CONTENT_TYPE, "application/json")
            .body(Body::from(body))
            .unwrap_or_default()
    }
}
