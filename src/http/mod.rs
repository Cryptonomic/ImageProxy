use async_trait::async_trait;
use hyper::http::uri::Scheme;
use hyper::Uri;
use log::{debug, error, info, warn};
use uuid::Uuid;

use crate::config::Host;
use crate::document::Document;
use crate::http::hyper_client::HyperHttpClient;
use crate::metrics;
use crate::rpc::error::Errors;

use self::filters::UriFilter;

pub mod filters;
pub mod hyper_client;

type StatusCode = u16;

const CODE_CONNECTION_ERROR: StatusCode = 900_u16;
const CODE_TIMEOUT: StatusCode = 901_u16;
const CODE_IO_ERROR: StatusCode = 901_u16;

#[async_trait]
pub trait HttpClientProvider {
    // TODO: Not happy with this signature, need something better
    async fn fetch(&self, req_id: &Uuid, url: &Uri) -> Result<Document, StatusCode>;
}

pub struct HttpClientWrapper {
    client: Box<dyn HttpClientProvider + Send + Sync>,
    ipfs_config: Host,
    uri_filters: Vec<Box<dyn UriFilter + Send + Sync>>,
}

impl HttpClientWrapper {
    fn to_uri(&self, url: &str) -> Result<Uri, Errors> {
        let uri = url.parse::<Uri>().map_err(|e| {
            error!("Error parsing url={}, reason={}", url, e);
            Errors::InvalidUri
        })?;

        match uri.scheme() {
            Some(s) if s.eq(&Scheme::HTTP) | s.eq(&Scheme::HTTPS) => {
                metrics::URI_DESTINATION_PROTOCOL
                    .with_label_values(&[s.to_string().to_ascii_lowercase().as_str()])
                    .inc();
                if let Some(hostname) = uri.host() {
                    metrics::URI_DESTINATION_HOST
                        .with_label_values(&[hostname])
                        .inc();
                }
                Ok(uri)
            }
            Some(s) if s.to_string().eq_ignore_ascii_case("ipfs") => {
                metrics::URI_DESTINATION_PROTOCOL
                    .with_label_values(&["ipfs"])
                    .inc();
                match url
                    .strip_prefix("ipfs://")
                    .or_else(|| url.strip_prefix("IPFS://"))
                {
                    Some(ipfs_path) => {
                        let ipfs_path_prefix = self
                            .ipfs_config
                            .path
                            .strip_prefix('/')
                            .unwrap_or(&self.ipfs_config.path);
                        let gateway_url = format!(
                            "{}://{}:{}/{}/{}",
                            self.ipfs_config.protocol,
                            self.ipfs_config.host,
                            self.ipfs_config.port,
                            ipfs_path_prefix,
                            ipfs_path
                        );
                        debug!("Ipfs gateway path: {}", gateway_url);
                        let ipfs_uri = gateway_url.parse::<Uri>().map_err(|e| {
                            error!("Error parsing url={}, reason={}", url, e);
                            Errors::InvalidUri
                        });
                        metrics::URI_DESTINATION_HOST
                            .with_label_values(&[self.ipfs_config.host.as_str()])
                            .inc();
                        ipfs_uri
                    }
                    None => Err(Errors::InvalidUri),
                }
            }
            _ => Err(Errors::UnsupportedUriScheme),
        }
    }

    pub async fn fetch(&self, req_id: &Uuid, url: &str) -> Result<Document, Errors> {
        info!("Fetching document for id:{}, url:{}", req_id, url);
        let uri = self.to_uri(&url.to_string())?;

        let filter_results = self
            .uri_filters
            .iter()
            .map(|f| f.filter(&uri))
            .reduce(|a, b| a & b);

        match filter_results {
            Some(true) => match self.client.fetch(req_id, &uri).await {
                Ok(document) => {
                    info!(
                        "Document fetched for id={}, content_length={:?}, content_type={:?}",
                        req_id, document.content_length, document.content_type
                    );
                    metrics::DOCUMENT.with_label_values(&["fetched"]).inc();
                    metrics::DOCUMENT_TYPE
                        .with_label_values(&[document.content_type.as_str()])
                        .inc();
                    metrics::TRAFFIC
                        .with_label_values(&["fetched"])
                        .inc_by(document.bytes.len() as u64);
                    metrics::DOCUMENT_SIZE
                        .with_label_values(&["size_bytes"])
                        .observe(document.bytes.len() as f64);
                    metrics::HTTP_CLIENT_CODES.with_label_values(&["200"]).inc();
                    Ok(document)
                }

                Err(code) => {
                    metrics::DOCUMENT.with_label_values(&["fetch_error"]).inc();
                    metrics::HTTP_CLIENT_CODES
                        .with_label_values(&[code.to_string().as_str()])
                        .inc();
                    error!(
                        "Unable to fetch document, id={}, response_code={}, url={}",
                        req_id, code, url
                    );
                    Err(Errors::FetchFailed)
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

pub struct HttpClientFactory;

impl HttpClientFactory {
    pub fn get_provider(
        ipfs_config: Host,
        max_document_size: Option<u64>,
        uri_filters: Vec<Box<dyn UriFilter + Send + Sync>>,
        timeout: u64,
        useragent: Option<String>,
    ) -> HttpClientWrapper {
        assert!(
            !uri_filters.is_empty(),
            "No URI filters provided. This is insecure, check code. Exiting..."
        );
        HttpClientWrapper {
            client: Box::new(HyperHttpClient::new(max_document_size, timeout, useragent)),
            ipfs_config,
            uri_filters,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dns::StandardDnsResolver;
    use filters::private_network::PrivateNetworkFilter;

    #[test]
    fn test_to_uri_fn() {
        let ipfs_config = Host {
            protocol: "https".to_string(),
            host: "localhost".to_string(),
            port: 1337,
            path: "/ipfs".to_string(),
        };
        let uri_filters: Vec<Box<dyn UriFilter + Send + Sync>> = vec![Box::new(
            PrivateNetworkFilter::new(Box::new(StandardDnsResolver {})),
        )];
        let wrapper = HttpClientFactory::get_provider(ipfs_config, None, uri_filters, 10_u64, None);

        // Valid https url
        let result = wrapper.to_uri("https://localhost:3422/image.png");
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().to_string(),
            "https://localhost:3422/image.png"
        );

        // Valid ipfs url
        let result = wrapper.to_uri("ipfs://CID");
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().to_string(),
            "https://localhost:1337/ipfs/CID"
        );

        // Invalid url
        let result = wrapper.to_uri("ipfs:/CID");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), Errors::InvalidUri);

        // Relative urls are not valid
        let result = wrapper.to_uri("/CID/image.png");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), Errors::UnsupportedUriScheme);

        // Unsupported url scheme
        let result = wrapper.to_uri("sftp://localhost/image.png");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), Errors::UnsupportedUriScheme);
    }
}
