use std::borrow::Borrow;
use std::time::Duration;

use std::error::Error as StdError;
use std::io::ErrorKind;

use hyper::client::HttpConnector;
use hyper::http::uri::Scheme;
use hyper::Client;
use hyper::{body::to_bytes, Uri};
use hyper_timeout::TimeoutConnector;
use hyper_tls::HttpsConnector;
use log::error;
use log::info;
use log::{debug, warn};
use uuid::Uuid;

use crate::config::Host;
use crate::document::Document;
use crate::metrics;
use crate::rpc::error::Errors;

use hyper::client::connect::dns::GaiResolver;
use hyper::Body;

use self::filters::UriFilter;

pub mod filters;

pub struct HttpClient {
    client: Client<TimeoutConnector<HttpsConnector<HttpConnector<GaiResolver>>>, Body>,
    _max_document_size: Option<u64>,
    ipfs_config: Host,
    uri_filters: Vec<Box<dyn UriFilter + Send + Sync>>,
}

impl HttpClient {
    pub fn new(
        ipfs_config: Host,
        max_document_size: Option<u64>,
        uri_filters: Vec<Box<dyn UriFilter + Send + Sync>>,
        timeout: u64,
    ) -> HttpClient {
        let https = HttpsConnector::new();
        let mut connector = TimeoutConnector::new(https);

        connector.set_connect_timeout(Some(Duration::from_secs(timeout)));
        connector.set_read_timeout(Some(Duration::from_secs(timeout)));
        connector.set_write_timeout(Some(Duration::from_secs(timeout)));

        let client = Client::builder().build::<_, hyper::Body>(connector);
        assert!(
            !uri_filters.is_empty(),
            "No URI filters provided. This is insecure, check code. Exiting..."
        );
        HttpClient {
            client,
            _max_document_size: max_document_size,
            ipfs_config,
            uri_filters,
        }
    }

    fn to_uri(ipfs_config: &Host, url: &str) -> Result<Uri, Errors> {
        let uri = url.parse::<Uri>().map_err(|e| {
            error!("Error parsing url={}, reason={}", url, e);
            Errors::InvalidUri
        })?;

        match uri.scheme() {
            Some(s) if s.eq(&Scheme::HTTP) | s.eq(&Scheme::HTTPS) => Ok(uri),
            Some(s) if s.to_string().eq_ignore_ascii_case("ipfs") => {
                match url
                    .strip_prefix("ipfs://")
                    .or_else(|| url.strip_prefix("IPFS://"))
                {
                    Some(ipfs_path) => {
                        let ipfs_path_prefix = ipfs_config
                            .path
                            .strip_prefix('/')
                            .unwrap_or(&ipfs_config.path);
                        let gateway_url = format!(
                            "{}://{}:{}/{}/{}",
                            ipfs_config.protocol,
                            ipfs_config.host,
                            ipfs_config.port,
                            ipfs_path_prefix,
                            ipfs_path
                        );
                        debug!("Ipfs gateway path: {}", gateway_url);
                        gateway_url.parse::<Uri>().map_err(|e| {
                            error!("Error parsing url={}, reason={}", url, e);
                            Errors::InvalidUri
                        })
                    }
                    None => Err(Errors::InvalidUri),
                }
            }
            _ => Err(Errors::UnsupportedUriScheme),
        }
    }

    pub async fn fetch(&self, req_id: &Uuid, url: &str) -> Result<Document, Errors> {
        info!("Fetching document for id:{}, url:{}", req_id, url);
        let uri = HttpClient::to_uri(&self.ipfs_config, &url.to_string())?;

        let filter_results = self
            .uri_filters
            .iter()
            .map(|f| f.filter(uri.borrow()))
            .reduce(|a, b| a & b);

        match filter_results {
            Some(true) => match self.client.get(uri).await {
                Ok(response) => match response.status() {
                    hyper::StatusCode::OK => {
                        let headers = response.headers().clone();
                        let content_length = headers
                            .get(hyper::header::CONTENT_LENGTH)
                            .map(|h| {
                                String::from_utf8(h.as_bytes().to_vec())
                                    .map(|s| s.parse::<u64>().ok())
                                    .ok()
                            })
                            .flatten()
                            .flatten()
                            .unwrap_or(0);
                        let content_type = headers
                            .get(hyper::header::CONTENT_TYPE)
                            .map(|h| String::from_utf8(h.as_bytes().to_vec()).ok())
                            .flatten()
                            .unwrap_or_default();
                        let bytes = to_bytes(response.into_body()).await.map_err(|e| {
                            error!("Error retrieving document body, reason={}", e);
                            Errors::FetchFailed
                        })?;

                        info!(
                            "Document fetched for id={}, content_length={:?}, content_type={:?}",
                            req_id, content_length, content_type
                        );
                        metrics::DOCUMENT.with_label_values(&["fetched"]).inc();
                        metrics::TRAFFIC
                            .with_label_values(&["fetched"])
                            .inc_by(bytes.len() as u64);
                        metrics::DOCUMENT_SIZE
                            .with_label_values(&["size_bytes"])
                            .observe(bytes.len() as f64);

                        Ok(Document {
                            id: *req_id,
                            content_type,
                            content_length,
                            bytes,
                            url: url.to_string(),
                        })
                    }
                    hyper::StatusCode::NOT_FOUND => {
                        metrics::DOCUMENT.with_label_values(&["fetch_error"]).inc();
                        error!("Document not found on remote, id={}", req_id);
                        Err(Errors::NotFound)
                    }

                    e => {
                        metrics::DOCUMENT.with_label_values(&["fetch_error"]).inc();
                        error!(
                            "Unable to fetch document, id={}, response_code={}, url={}",
                            req_id, e, url
                        );
                        Err(Errors::FetchFailed)
                    }
                },
                Err(e) => {
                    metrics::DOCUMENT.with_label_values(&["fetch_error"]).inc();
                    let err = e
                        .source()
                        .unwrap()
                        .downcast_ref::<std::io::Error>()
                        .unwrap();
                   
                    if let ErrorKind::TimedOut = err.kind() {
                        error!("Unable to fetch document, connection/response/request to the server timed out, id={}", req_id);
                        Err(Errors::TimedOut)
                    } else {
                        error!(
                            "Unable to fetch document, id={}, reason={}, url={}",
                            req_id, e, url
                        );
                        Err(Errors::FetchFailed)
                    }
                }
            },
            Some(false) => {
                warn!("Invalid destination host for id:{}", req_id);
                metrics::URI_FILTER_BLOCKED.inc();
                Err(Errors::InvalidOrBlockedHost)
            }
            None => {
                warn!("Invalid destination host for id:{}", req_id);
                metrics::URI_FILTER_BLOCKED.inc();
                Err(Errors::InvalidOrBlockedHost)
            }
        }
    }
}
