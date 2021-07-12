use hyper::{Body, Response};
use serde::Serialize;
use uuid::Uuid;

use super::responses::*;
use super::VERSION;

#[derive(Serialize)]
pub struct RpcError {
    pub code: u8,
    pub reason: String,
    pub request_id: Uuid,
}

#[derive(Serialize)]
pub enum ImgProxyError {
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
}

impl ImgProxyError {
    fn to_rpc_error(&self, request_id: Uuid) -> RpcError {
        let (code, reason) = match *self {
            ImgProxyError::InvalidRpcVersionError => (100, "Invalid RPC version".to_string()),
            ImgProxyError::InvalidRpcMethodError => (101, "Invalid RPC method".to_string()),
            ImgProxyError::JsonDecodeError => (102, "Invalid JSON supplied".to_string()),
            ImgProxyError::InternalError => (103, "Internal Error".to_string()),
            ImgProxyError::FetchFailed => (104, "Fetch Failed".to_string()),
            ImgProxyError::NotFound => (105, "Image not found".to_string()),
            ImgProxyError::ModerationFailed => (106, "Image moderation failed".to_string()),
            ImgProxyError::UnsupportedImageType => (107, "Image type unsupported".to_string()),
            ImgProxyError::UnsupportedUriScheme => (108, "Uri scheme unsupported".to_string()),
            ImgProxyError::InvalidUri => (109, "Invalid Uri".to_string()),
        };

        RpcError {
            code,
            reason,
            request_id,
        }
    }

    pub fn to_response(&self, request_id: Uuid) -> Response<Body> {
        let error = self.to_rpc_error(request_id);

        let body = serde_json::to_string_pretty(&ErrorResponse {
            jsonrpc: VERSION.to_string(),
            rpc_status: RpcStatus::Err,
            error: error,
        })
        .unwrap_or_default()
        .clone();
        Response::builder()
            .status(hyper::StatusCode::OK)
            .header(hyper::header::CONTENT_TYPE, "application/json")
            .body(Body::from(body))
            .unwrap_or_default()
    }
}
