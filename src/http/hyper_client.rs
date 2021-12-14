use std::error::Error;
use std::io::ErrorKind;
use std::time::Duration;

use async_trait::async_trait;
use hyper::client::HttpConnector;
use hyper::Client;
use hyper::{body::to_bytes, Uri};
use hyper_timeout::TimeoutConnector;
use hyper_tls::HttpsConnector;
use log::error;

use hyper::client::connect::dns::GaiResolver;
use hyper::Body;
use uuid::Uuid;

use super::{HttpClientProvider, StatusCode, CODE_CONNECTION_ERROR, CODE_IO_ERROR, CODE_TIMEOUT};
use crate::document::Document;

pub struct HyperHttpClient {
    client: Client<TimeoutConnector<HttpsConnector<HttpConnector<GaiResolver>>>, Body>,
    _max_document_size: Option<u64>,
}

impl HyperHttpClient {
    pub fn new(max_document_size: Option<u64>, timeout: u64) -> Self {
        let https = HttpsConnector::new();
        let mut connector = TimeoutConnector::new(https);

        connector.set_connect_timeout(Some(Duration::from_secs(timeout)));
        connector.set_read_timeout(Some(Duration::from_secs(timeout)));
        connector.set_write_timeout(Some(Duration::from_secs(timeout)));

        let client = Client::builder().build::<_, hyper::Body>(connector);
        HyperHttpClient {
            client,
            _max_document_size: max_document_size,
        }
    }
}

#[async_trait]
impl HttpClientProvider for HyperHttpClient {
    async fn fetch(&self, req_id: &Uuid, uri: &Uri) -> Result<Document, StatusCode> {
        let response = self.client.get(uri.clone()).await.map_err(|error| {
            error!(
                "Unable to fetch document, id={}, reason={}, url={}",
                req_id, error, uri
            );
            if let Some(err_ref) = error.source() {
                if let Some(err) = err_ref.downcast_ref::<std::io::Error>() {
                    if let ErrorKind::TimedOut = err.kind() {
                        return CODE_TIMEOUT;
                    }
                }
            }
            CODE_CONNECTION_ERROR
        })?;
        match response.status() {
            hyper::StatusCode::OK => {
                let headers = response.headers().clone();
                let bytes = to_bytes(response.into_body()).await.map_err(|error| {
                    error!(
                        "Unable to fetch document, id={}, reason={}, url={}",
                        req_id, error, uri
                    );
                    CODE_IO_ERROR
                })?;
                let content_length = headers
                    .get(hyper::header::CONTENT_LENGTH)
                    .and_then(|h| {
                        String::from_utf8(h.as_bytes().to_vec())
                            .map(|s| s.parse::<u64>().ok())
                            .ok()
                    })
                    .flatten()
                    .unwrap_or(bytes.len() as u64);
                let content_type = headers
                    .get(hyper::header::CONTENT_TYPE)
                    .and_then(|h| String::from_utf8(h.as_bytes().to_vec()).ok())
                    .unwrap_or_default();
                Ok(Document {
                    id: *req_id,
                    content_type,
                    content_length,
                    bytes,
                    url: uri.to_string(),
                })
            }
            status_code => Err(status_code.as_u16()),
        }
    }
}
