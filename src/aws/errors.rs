use aws_sdk_s3::error::PutObjectError;
use aws_sdk_s3::SdkError;

use aws_sdk_rekognition::error::{GetContentModerationError, StartContentModerationError};
use aws_sdk_rekognition::model::VideoJobStatus;

use crate::rpc::error::Errors as RpcError;
use std::fmt;

use tokio::sync::AcquireError;

pub enum S3Error {
    Other(String),
}

impl From<String> for AwsError {
    fn from(src: String) -> AwsError {
        AwsError::Other(src)
    }
}

impl From<&str> for AwsError {
    fn from(src: &str) -> AwsError {
        src.to_string().into()
    }
}

impl From<AwsError> for RpcError {
    fn from(src: AwsError) -> RpcError {
        match src {
            AwsError::LocalError(_) | AwsError::NoResponseRecieved(_) | AwsError::Other(_) => {
                RpcError::InternalError
            }
            AwsError::RemoteError(_) | AwsError::CorruptResponse(_) => RpcError::ModerationFailed,
        }
    }
}

// we will never encounter this error , since we never close the semaphore
impl From<AcquireError> for AwsError {
    fn from(_src: AcquireError) -> AwsError {
        "a semaphore was closed".into()
    }
}

impl From<VideoJobStatus> for AwsError {
    fn from(src: VideoJobStatus) -> AwsError {
        let service = AwsService::Rekognition;
        let proxy_msg = "attempting to start moderation".to_string();

        match src {
            VideoJobStatus::Failed => AwsError::RemoteError(AwsErrorDetails {
                service,
                proxy_msg,
                error: "job failed".into(),
            }),
            VideoJobStatus::Unknown(s) => AwsError::RemoteError(AwsErrorDetails {
                service,
                proxy_msg,
                error: s.into(),
            }),
            _ => "CODE ERROR: no error encountered  aws".into(),
        }
    }
}

impl From<SdkError<PutObjectError>> for AwsError {
    fn from(src: SdkError<PutObjectError>) -> AwsError {
        use AwsError::*;
        let service = AwsService::S3;
        let proxy_msg = "attempting to send data to s3 bucket".to_string();
        match src {
            SdkError::ConstructionFailure(error) => LocalError(AwsErrorDetails {
                service,
                proxy_msg,
                error,
            }),
            SdkError::DispatchFailure(error) => NoResponseRecieved(AwsErrorDetails {
                service,
                proxy_msg,
                error: Box::new(error),
            }),
            SdkError::ServiceError { err, .. } => RemoteError(AwsErrorDetails {
                service,
                proxy_msg,
                error: Box::new(err),
            }),

            SdkError::ResponseError { err, .. } => CorruptResponse(AwsErrorDetails {
                service,
                proxy_msg,
                error: err,
            }),
        }
    }
}

impl From<SdkError<StartContentModerationError>> for AwsError {
    fn from(src: SdkError<StartContentModerationError>) -> AwsError {
        use AwsError::*;
        let service = AwsService::Rekognition;
        let proxy_msg = "attempting to start moderation".to_string();
        match src {
            SdkError::ConstructionFailure(error) => LocalError(AwsErrorDetails {
                service,
                proxy_msg,
                error,
            }),
            SdkError::DispatchFailure(error) => NoResponseRecieved(AwsErrorDetails {
                service,
                proxy_msg,
                error: Box::new(error),
            }),
            SdkError::ServiceError { err, .. } => RemoteError(AwsErrorDetails {
                service,
                proxy_msg,
                error: Box::new(err),
            }),

            SdkError::ResponseError { err, .. } => CorruptResponse(AwsErrorDetails {
                service,
                proxy_msg,
                error: err,
            }),
        }
    }
}

impl From<SdkError<GetContentModerationError>> for AwsError {
    fn from(src: SdkError<GetContentModerationError>) -> AwsError {
        use AwsError::*;
        let service = AwsService::Rekognition;
        let proxy_msg = "attempting to start moderation".to_string();
        match src {
            SdkError::ConstructionFailure(error) => LocalError(AwsErrorDetails {
                service,
                proxy_msg,
                error,
            }),
            SdkError::DispatchFailure(error) => NoResponseRecieved(AwsErrorDetails {
                service,
                proxy_msg,
                error: Box::new(error),
            }),
            SdkError::ServiceError { err, .. } => RemoteError(AwsErrorDetails {
                service,
                proxy_msg,
                error: Box::new(err),
            }),

            SdkError::ResponseError { err, .. } => CorruptResponse(AwsErrorDetails {
                service,
                proxy_msg,
                error: err,
            }),
        }
    }
}

#[derive(Debug)]
enum AwsService {
    S3,
    Rekognition,
}

impl fmt::Display for AwsService {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AwsService::S3 => "S3".fmt(f),
            AwsService::Rekognition => "Rekognition".fmt(f),
        }
    }
}

#[derive(Debug)]
pub struct AwsErrorDetails {
    service: AwsService,
    proxy_msg: String,
    error: Box<dyn std::error::Error + Send + Sync>,
}

#[derive(Debug)]
pub enum AwsError {
    LocalError(AwsErrorDetails),
    NoResponseRecieved(AwsErrorDetails),
    RemoteError(AwsErrorDetails),
    CorruptResponse(AwsErrorDetails),
    Other(String),
}

unsafe impl Send for AwsError {}
unsafe impl Sync for AwsError {}

impl std::error::Error for AwsError {}

impl fmt::Display for AwsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use AwsError::*;
        match self {
            LocalError(d) => write!(
                f,
                "ImageProxy encountered an error at origin. {}, proxy message: {}, service_error: {} ",
                d.proxy_msg,
                d.service,
                d.error,
            ),

            NoResponseRecieved(d) => write!(
                f,
                "ImageProxy did't retrieve a response from the service. {}, proxy message: {}, service_error: {} ",
                d.proxy_msg,
                d.service,
                d.error,
            ),

            RemoteError(d) => write!(
                f,
                "ImageProxy encountered a remote error with service {} .proxy message: {}, service_error: {} ",
                d.service,
                d.proxy_msg,
                d.error,
            ),

            CorruptResponse(d) => write!(
                f,
                "ImageProxy encountered a corrupt response from the remote service. {},.proxy message: {}, service_error: {} ",
                d.service,
                d.proxy_msg,
                d.error,
            ),

           _ => unimplemented!(),
        }
    }
}
impl fmt::Display for AwsErrorDetails {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "  service: {} proxy_msg: {}\n error: {}\n ",
            self.service, self.proxy_msg, self.error
        )
    }
}
