use hyper::client::HttpConnector;
use hyper::Client;
use hyper::{body::to_bytes, Uri};
use hyper_tls::HttpsConnector;
use log::debug;
use log::error;
use log::info;
use uuid::Uuid;

use crate::config::Host;
use crate::dns::DnsResolver;
use crate::document::Document;
use crate::metrics;
use crate::rpc::responses::StatusCodes;

use self::filters::UriFilter;

pub mod filters;

pub struct HttpClient {
    client: Client<HttpsConnector<HttpConnector>>,
    dns_resolver: Box<dyn DnsResolver + Send + Sync>,
    max_document_size: Option<u64>,
    ipfs_config: Host,
    uri_filters: Vec<Box<dyn UriFilter + Send + Sync>>,
}

impl HttpClient {
    pub fn new(
        ipfs_config: Host,
        dns_resolver: Box<dyn DnsResolver + Send + Sync>,
        max_document_size: Option<u64>,
        uri_filters: Vec<Box<dyn UriFilter + Send + Sync>>,
    ) -> HttpClient {
        let https = HttpsConnector::new();
        let client = Client::builder().build(https);
        assert!(
            uri_filters.len() > 0,
            "No URI filters provided. This is insecure, check code. Exiting..."
        );
        HttpClient {
            client,
            dns_resolver,
            max_document_size,
            ipfs_config,
            uri_filters,
        }
    }

    fn to_uri(ipfs_config: &Host, url: &String) -> Result<Uri, StatusCodes> {
        match url {
            u if u.starts_with("http://") | u.starts_with("https://") => {
                u.parse::<Uri>().map_err(|e| {
                    error!("Error parsing url={}, reason={}", u, e);
                    StatusCodes::InvalidUri
                })
            }
            u if u.starts_with("ipfs://") => {
                let ipfs_path = url.strip_prefix("ipfs://").unwrap_or_default();
                let gateway_url = format!(
                    "{}://{}:{}{}/{}",
                    ipfs_config.protocol,
                    ipfs_config.host,
                    ipfs_config.port,
                    ipfs_config.path,
                    ipfs_path
                );
                debug!("Ipfs gateway path: {}", gateway_url);
                gateway_url.parse::<Uri>().map_err(|e| {
                    error!("Error parsing url={}, reason={}", u, e);
                    StatusCodes::InvalidUri
                })
            }
            _ => Err(StatusCodes::UnsupportedUriScheme),
        }
    }

    pub async fn fetch(&self, req_id: &Uuid, url: &String) -> Result<Document, StatusCodes> {
        info!("Fetching document for id:{}, url:{}", req_id, url);
        let uri = HttpClient::to_uri(&self.ipfs_config, url)?;
        match self.client.get(uri).await {
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
                        .unwrap_or("".to_string());
                    let bytes = to_bytes(response.into_body()).await.map_err(|e| {
                        error!("Error retrieving document body, reason={}", e);
                        StatusCodes::DocumentFetchFailed
                    })?;

                    info!(
                        "Document fetched for id={}, content_length={:?}, content_type={:?}",
                        req_id, content_length, content_type
                    );
                    metrics::DOCUMENTS_FETCHED.inc();
                    metrics::BYTES_FETCHED.inc_by(content_length as i64);
                    metrics::DOCUMENT_SIZE
                        .with_label_values(&["size_bytes"])
                        .observe(content_length as f64);

                    Ok(Document {
                        id: req_id.clone(),
                        content_type: content_type,
                        content_length: content_length,
                        bytes: bytes,
                    })
                }
                hyper::StatusCode::NOT_FOUND => {
                    metrics::DOCUMENTS_FETCHED_ERROR.inc();
                    error!("Document not found on remote, id={}", req_id);
                    Err(StatusCodes::DocumentNotFound)
                }
                e => {
                    metrics::DOCUMENTS_FETCHED_ERROR.inc();
                    error!(
                        "Unable to fetch document, id={}, response_code={}",
                        req_id, e
                    );
                    Err(StatusCodes::DocumentFetchFailed)
                }
            },
            Err(e) => {
                metrics::DOCUMENTS_FETCHED_ERROR.inc();
                error!("Unable to fetch document, id={}, reason={}", req_id, e);
                Err(StatusCodes::DocumentFetchFailed)
            }
        }
    }
}
