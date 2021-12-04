use std::sync::Arc;

use hyper::{Body, Response};
use log::error;
use serde::Serialize;
use uuid::Uuid;

use super::{
    error::{Errors, RpcError},
    requests::ResponseType,
};
use crate::{
    document::Document,
    metrics,
    moderation::{ModerationCategories, ModerationService},
};

use super::VERSION;

#[derive(Serialize)]
pub enum RpcStatus {
    Ok,
    Err,
}

#[derive(Serialize, PartialEq)]
pub enum ModerationStatus {
    Allowed,
    Blocked,
}

impl From<bool> for ModerationStatus {
    fn from(s: bool) -> Self {
        if s {
            ModerationStatus::Blocked
        } else {
            ModerationStatus::Allowed
        }
    }
}

#[derive(Default, Serialize)]
pub struct Info {
    pub package_version: &'static str,
    pub git_version: &'static str,
}

#[derive(Serialize)]
pub struct ModerationResult {
    pub moderation_status: ModerationStatus,
    pub categories: Vec<ModerationCategories>,
    pub data: String,
    #[serde(skip_serializing)]
    pub document: Option<Arc<Document>>,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub jsonrpc: String,
    pub rpc_status: RpcStatus,
    pub error: RpcError,
}

#[derive(Serialize)]
pub struct FetchResponse {
    pub jsonrpc: String,
    pub rpc_status: RpcStatus,
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
    pub rpc_status: RpcStatus,
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
    pub rpc_status: RpcStatus,
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
    pub rpc_status: RpcStatus,
    pub result: Vec<ReportDescribeResult>,
}

#[derive(Serialize)]
pub struct ServerError {
    pub jsonrpc: String,
    pub rpc_status: RpcStatus,
}

impl FetchResponse {
    pub fn to_response(
        response_type: &ResponseType,
        document: Option<Arc<Document>>,
        moderation_status: ModerationStatus,
        categories: Vec<ModerationCategories>,
        req_id: &Uuid,
    ) -> Response<Body> {
        match response_type {
            ResponseType::Raw => {
                if let Some(doc) = document {
                    metrics::TRAFFIC
                        .with_label_values(&["served"])
                        .inc_by(doc.bytes.len() as u64);
                    Response::builder()
                        .status(200)
                        .header(hyper::header::CONTENT_TYPE, doc.content_type.clone())
                        .header(hyper::header::CONTENT_LENGTH, doc.bytes.len())
                        .body(Body::from(doc.bytes.clone()))
                        .unwrap_or_default()
                } else {
                    FetchResponse::to_response(
                        &ResponseType::Json,
                        None,
                        moderation_status,
                        categories,
                        req_id,
                    )
                }
            }
            ResponseType::Json => {
                let result = FetchResponse {
                    jsonrpc: String::from(VERSION),
                    rpc_status: RpcStatus::Ok,
                    result: ModerationResult {
                        moderation_status,
                        categories,
                        data: document.map(|doc| doc.to_url()).unwrap_or_default(),
                        document: None,
                    },
                };
                metrics::TRAFFIC
                    .with_label_values(&["served"])
                    .inc_by(result.result.data.len() as u64);

                match serde_json::to_string_pretty(&result) {
                    Ok(body) => Response::builder()
                        .status(hyper::StatusCode::OK)
                        .header(hyper::header::CONTENT_TYPE, "application/json")
                        .body(Body::from(body))
                        .unwrap_or_default(),
                    Err(e) => {
                        error!("Error serializing fetch response, reason={}", e);
                        Errors::InternalError.to_response(req_id)
                    }
                }
            }
        }
    }
}

impl DescribeResponse {
    pub fn to_response(
        rpc_status: RpcStatus,
        describe_results: Vec<DescribeResult>,
        req_id: &Uuid,
    ) -> Response<Body> {
        let result = DescribeResponse {
            jsonrpc: String::from(VERSION),
            rpc_status,
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
                Errors::InternalError.to_response(req_id)
            }
        }
    }
}

impl ReportResponse {
    pub fn to_response(rpc_status: RpcStatus, url: &str, req_id: &Uuid) -> Response<Body> {
        let result = ReportResponse {
            jsonrpc: String::from(VERSION),
            rpc_status,
            result: ReportResult {
                url: String::from(url),
                id: *req_id,
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
                Errors::InternalError.to_response(req_id)
            }
        }
    }
}

impl ReportDescribeResponse {
    pub fn to_response(
        rpc_status: RpcStatus,
        results: Vec<ReportDescribeResult>,
        req_id: &Uuid,
    ) -> Response<Body> {
        let result = ReportDescribeResponse {
            jsonrpc: String::from(VERSION),
            rpc_status,
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
                Errors::InternalError.to_response(req_id)
            }
        }
    }
}
