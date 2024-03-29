use http_body_util::Full;
use hyper::body::Bytes;
use hyper::Response;
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

#[derive(std::fmt::Debug, PartialEq, Eq, Serialize)]
pub enum Errors {
    InvalidRpcVersionError,
    InvalidRpcMethodError,
    RpcPayloadTooBigError,
    JsonDecodeError,
    InternalError,
    FetchFailed,
    NotFound,
    ModerationFailed,
    UnsupportedImageType,
    UnsupportedUriScheme,
    InvalidUri,
    InvalidOrBlockedHost,
    TimedOut,
    ImageResizeError,
}

impl Errors {
    pub fn to_rpc_error(&self, request_id: &Uuid) -> RpcError {
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
            Errors::TimedOut => (
                111,
                "Connection/Request/Response from the destination timed out".to_string(),
            ),
            Errors::ImageResizeError => (112, "Image Resize Error".to_string()),
            Errors::RpcPayloadTooBigError => (113, "RPC Payload too big".to_string()),
        };

        RpcError {
            code,
            reason,
            request_id: *request_id,
        }
    }

    pub fn to_response(&self, request_id: &Uuid) -> Response<Full<Bytes>> {
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
            .body(Full::new(Bytes::from(body)))
            .unwrap_or_default()
    }
}
