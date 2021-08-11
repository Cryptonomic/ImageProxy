use hyper::{Body, Response};
use serde::Serialize;
use uuid::Uuid;

use super::responses::*;
use super::VERSION;
use crate::config::Configuration;

#[derive(Serialize)]
pub struct RpcError {
    pub code: u8,
    pub reason: String,
    pub request_id: Uuid,
}

#[derive(Serialize)]
pub enum Errors {
    InvalidRpcVersionError,
    InvalidRpcMethodError,
    JsonDecodeError,
    InternalError,
    FetchFailed,
    NotFound,
    ModerationFailed,
    UnsupportedImageType,
    UnsupportedUriScheme,
    InvalidUri,
    InvalidOrBlockedHost,
}

impl Errors {
    fn to_rpc_error(&self, request_id: &Uuid) -> RpcError {
        let (code, reason) = match *self {
            Errors::InvalidRpcVersionError => (100, "Invalid RPC version".to_string()),
            Errors::InvalidRpcMethodError => (101, "Invalid RPC method".to_string()),
            Errors::JsonDecodeError => (102, "Invalid JSON supplied".to_string()),
            Errors::InternalError => (103, "Internal Error".to_string()),
            Errors::FetchFailed => (104, "Fetch Failed".to_string()),
            Errors::NotFound => (105, "Image not found".to_string()),
            Errors::ModerationFailed => (106, "Image moderation failed".to_string()),
            Errors::UnsupportedImageType => (107, "Image type unsupported".to_string()),
            Errors::UnsupportedUriScheme => (108, "Uri scheme unsupported".to_string()),
            Errors::InvalidUri => (109, "Invalid Uri".to_string()),
            Errors::InvalidOrBlockedHost => {
                (110, "Invalid or blocked destination host".to_string())
            }
        };

        RpcError {
            code,
            reason,
            request_id: *request_id,
        }
    }

    pub fn to_response(&self, request_id: &Uuid, config: &Configuration) -> Response<Body> {
        let error = self.to_rpc_error(request_id);

        let body = serde_json::to_string_pretty(&ErrorResponse {
            jsonrpc: VERSION.to_string(),
            rpc_status: RpcStatus::Err,
            error,
        })
        .unwrap_or_default();
        Response::builder()
            .status(hyper::StatusCode::OK)
            .header(hyper::header::CONTENT_TYPE, "application/json")
            .header(
                hyper::header::ACCESS_CONTROL_ALLOW_ORIGIN,
                &config.cors.origin,
            )
            .body(Body::from(body))
            .unwrap_or_default()
    }
}
