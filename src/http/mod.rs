use async_trait::async_trait;
use hyper::http::uri::Scheme;
use hyper::Uri;
use log::{debug, error, info, warn};
use uuid::Uuid;

use crate::config::{Host, IpfsGatewayConfig};
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
    ipfs_config: IpfsGatewayConfig,
    uri_filters: Vec<Box<dyn UriFilter + Send + Sync>>,
}

#[derive(PartialEq, Debug)]
enum UriScheme {
    Http,
    Https,
    Ipfs,
}

impl std::fmt::Display for UriScheme {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

#[derive(Debug)]
struct ParsedUri {
    uri: Uri,
    scheme: UriScheme,
}

impl HttpClientWrapper {
    pub fn new(
        client: Box<dyn HttpClientProvider + Send + Sync>,
        ipfs_config: IpfsGatewayConfig,
        uri_filters: Vec<Box<dyn UriFilter + Send + Sync>>,
    ) -> Self {
        assert!(
            !uri_filters.is_empty(),
            "No URI filters configured. This cannot be correct, check code."
        );
        HttpClientWrapper {
            client,
            ipfs_config,
            uri_filters,
        }
    }

    fn parse_uri(url: &str) -> Result<ParsedUri, Errors> {
        let uri = url.parse::<Uri>().map_err(|e| {
            error!("Error parsing url={}, reason={}", url, e);
            Errors::InvalidUri
        })?;
        match uri.scheme() {
            Some(s) if s.eq(&Scheme::HTTP) => Ok(ParsedUri {
                uri,
                scheme: UriScheme::Http,
            }),
            Some(s) if s.eq(&Scheme::HTTPS) => Ok(ParsedUri {
                uri,
                scheme: UriScheme::Https,
            }),
            Some(s) if s.to_string().eq_ignore_ascii_case("ipfs") => Ok(ParsedUri {
                uri,
                scheme: UriScheme::Ipfs,
            }),
            _ => Err(Errors::UnsupportedUriScheme),
        }
    }

    fn construct_ipfs_uri(url: &str, ipfs_config: &Host) -> Result<Uri, Errors> {
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
                let ipfs_uri = gateway_url.parse::<Uri>().map_err(|e| {
                    error!("Error parsing url={}, reason={}", url, e);
                    Errors::InvalidUri
                });
                metrics::URI_DESTINATION_HOST
                    .with_label_values(&[ipfs_config.host.as_str()])
                    .inc();
                ipfs_uri
            }
            None => Err(Errors::InvalidUri),
        }
    }

    pub async fn fetch(&self, req_id: &Uuid, url: &str) -> Result<Document, Errors> {
        info!("Fetching document for id:{}, url:{}", req_id, url);
        let parsed_uri = HttpClientWrapper::parse_uri(url)?;

        // Metrics
        metrics::URI_DESTINATION_PROTOCOL
            .with_label_values(&[parsed_uri.scheme.to_string().to_ascii_lowercase().as_str()])
            .inc();

        let uri = if parsed_uri.scheme == UriScheme::Http || parsed_uri.scheme == UriScheme::Https {
            if let Some(hostname) = parsed_uri.uri.host() {
                metrics::URI_DESTINATION_HOST
                    .with_label_values(&[hostname])
                    .inc();
            }
            parsed_uri.uri
        } else {
            // IPFS
            HttpClientWrapper::construct_ipfs_uri(url, &self.ipfs_config.primary)?
        };

        let result = self.fetch2(req_id, &uri).await;
        if result.is_err()
            && parsed_uri.scheme == UriScheme::Ipfs
            && self.ipfs_config.fallback.is_some()
        {
            let fallback_ipfs_config = self.ipfs_config.fallback.as_ref().unwrap();
            let uri = HttpClientWrapper::construct_ipfs_uri(url, fallback_ipfs_config)?;
            self.fetch2(req_id, &uri).await
        } else {
            result
        }
    }

    async fn fetch2(&self, req_id: &Uuid, uri: &Uri) -> Result<Document, Errors> {
        let filter_results = self
            .uri_filters
            .iter()
            .map(|f| f.filter(uri))
            .reduce(|a, b| a & b);

        match filter_results {
            Some(true) => match self.client.fetch(req_id, uri).await {
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
                        req_id, code, uri
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
        ipfs_config: IpfsGatewayConfig,
        max_document_size: Option<u64>,
        uri_filters: Vec<Box<dyn UriFilter + Send + Sync>>,
        timeout: u64,
        useragent: Option<String>,
    ) -> HttpClientWrapper {
        assert!(
            !uri_filters.is_empty(),
            "No URI filters provided. This is insecure, check code. Exiting..."
        );

        HttpClientWrapper::new(
            Box::new(HyperHttpClient::new(max_document_size, timeout, useragent)),
            ipfs_config,
            uri_filters,
        )
    }
}

#[cfg(test)]
pub mod tests {
    use std::{collections::HashMap, net::IpAddr, sync::Mutex};

    use super::*;
    use crate::dns::{DummyDnsResolver, StandardDnsResolver};
    use filters::private_network::PrivateNetworkFilter;
    use hyper::body::Bytes;

    pub struct DummyHttpClient {
        store: Mutex<HashMap<String, Document>>,
    }

    impl Default for DummyHttpClient {
        fn default() -> Self {
            Self::new()
        }
    }

    impl DummyHttpClient {
        pub fn new() -> Self {
            DummyHttpClient {
                store: Mutex::new(HashMap::new()),
            }
        }

        pub fn set(&mut self, url: &str, document: Document) {
            let mut store = self.store.lock().unwrap();
            store.insert(url.to_string(), document);
        }
    }

    #[async_trait]
    impl HttpClientProvider for DummyHttpClient {
        async fn fetch(&self, _: &Uuid, url: &Uri) -> Result<Document, StatusCode> {
            let store = self.store.lock().unwrap();
            let url = url.to_string();
            match store.get(&url) {
                Some(document) => Ok(document.clone()),
                None => Err(404),
            }
        }
    }

    fn construct_document(url: &str) -> Document {
        let buffer = "Hello There";
        Document {
            id: Uuid::new_v4(),
            content_type: "image/png".to_string(),
            content_length: buffer.len() as u64,
            bytes: Bytes::from(buffer),
            url: url.to_string(),
        }
    }

    /// Tests the fetch function to correctly see if uri filters
    /// are working. Specifically tests the localhost blocking filter    
    ///
    #[tokio::test]
    async fn test_fetch_block_localhost() {
        let url = "http://localhost/abcd";
        let dns_resolver = StandardDnsResolver {};
        let uri_filters: Vec<Box<dyn UriFilter + Send + Sync>> =
            vec![Box::new(PrivateNetworkFilter::new(Box::new(dns_resolver)))];
        let http_client = DummyHttpClient::new();

        let ipfs_config = IpfsGatewayConfig {
            primary: Host {
                protocol: "http".to_string(),
                host: "127.0.0.1".to_string(),
                port: 1337,
                path: "/ipfs".to_string(),
            },
            fallback: None,
        };

        let provider = HttpClientWrapper::new(Box::new(http_client), ipfs_config, uri_filters);
        // Test the result
        let result = provider.fetch(&Uuid::new_v4(), url).await;
        assert!(result.is_err());
        assert_eq!(result.err().unwrap(), Errors::InvalidOrBlockedHost);
    }

    /// Tests the fetch function correctly returns no document
    /// when the primary gateway has thrown an error and no fallback
    /// gateway is configured.
    ///
    /// A mock http client simulates returning a 404 for the url
    /// pointing to the primary gateway.
    ///
    #[tokio::test]
    async fn test_fetch_ipfs_no_fallback() {
        let ipfs_url = "ipfs://abcdef";
        let ip: IpAddr = "8.8.8.8".parse().unwrap();
        let ip_vec = vec![ip];
        let dns_resolver = DummyDnsResolver {
            resolved_address: ip_vec,
        };
        let uri_filters: Vec<Box<dyn UriFilter + Send + Sync>> =
            vec![Box::new(PrivateNetworkFilter::new(Box::new(dns_resolver)))];
        let http_client = DummyHttpClient::new();

        let ipfs_config = IpfsGatewayConfig {
            primary: Host {
                protocol: "http".to_string(),
                host: "127.0.0.1".to_string(),
                port: 1337,
                path: "/ipfs".to_string(),
            },
            fallback: None,
        };

        let provider = HttpClientWrapper::new(Box::new(http_client), ipfs_config, uri_filters);
        // Test the result
        let result = provider.fetch(&Uuid::new_v4(), ipfs_url).await;
        assert!(result.is_err());
        assert_eq!(result.err().unwrap(), Errors::FetchFailed);
    }

    /// Tests the fetch function correctly uses the fallback ipfs
    /// gateway when the primary gateway returns an error.
    ///
    /// A mock http client simulates returning a 404 for the url
    /// pointing to the primary gateway and a document for the
    /// fallback gateway.
    ///
    #[tokio::test]
    async fn test_fetch_ipfs_with_fallback() {
        let ipfs_url = "ipfs://abcdef";
        let ip: IpAddr = "8.8.8.8".parse().unwrap();
        let ip_vec = vec![ip];
        let dns_resolver = DummyDnsResolver {
            resolved_address: ip_vec,
        };
        let uri_filters: Vec<Box<dyn UriFilter + Send + Sync>> =
            vec![Box::new(PrivateNetworkFilter::new(Box::new(dns_resolver)))];
        let mut http_client = DummyHttpClient::new();

        let ipfs_config = IpfsGatewayConfig {
            primary: Host {
                protocol: "http".to_string(),
                host: "127.0.0.1".to_string(),
                port: 1337,
                path: "/ipfs".to_string(),
            },
            fallback: Some(Host {
                protocol: "https".to_string(),
                host: "localhost.com".to_string(),
                port: 443,
                path: "/ipfs".to_string(),
            }),
        };
        // Set the mock client to return results for the fallback url
        let mock_url = "https://localhost.com:443/ipfs/abcdef";
        http_client.set(mock_url, construct_document(mock_url));

        let provider = HttpClientWrapper::new(Box::new(http_client), ipfs_config, uri_filters);

        // Test the result
        let result = provider.fetch(&Uuid::new_v4(), ipfs_url).await;
        assert!(result.is_ok());
        // Assert that the document was fetched from the fallback gateway
        assert_eq!(result.unwrap().url, mock_url.to_string());
    }

    #[test]
    fn test_parse_uri() {
        let url1 = "https://localhost:3422/image.png";
        let url2 = "ipfs://OQIUi33i3u980uoasiduoi3202uas1ahj44";
        let url3 = "ftp://somedomain.com/data.png";
        let url4 = "http://localhost/image.png";
        let url5 = "ipfs:/CID";
        let url6 = "/CID/image.png";

        let result = HttpClientWrapper::parse_uri(url1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().scheme, UriScheme::Https);

        let result = HttpClientWrapper::parse_uri(url2);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().scheme, UriScheme::Ipfs);

        let result = HttpClientWrapper::parse_uri(url3);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), Errors::UnsupportedUriScheme);

        let result = HttpClientWrapper::parse_uri(url4);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().scheme, UriScheme::Http);

        let result = HttpClientWrapper::parse_uri(url5);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), Errors::InvalidUri);

        let result = HttpClientWrapper::parse_uri(url6);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), Errors::UnsupportedUriScheme);
    }

    #[test]
    fn test_construct_ipfs_uri() {
        let ipfs_config = Host {
            protocol: "https".to_string(),
            host: "localhost".to_string(),
            port: 1337,
            path: "/ipfs".to_string(),
        };

        let url1 = "ipfs://abcdefgh";
        let url2 = "http://abcdefgh";

        let result = HttpClientWrapper::construct_ipfs_uri(url1, &ipfs_config);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            Uri::from_static("https://localhost:1337/ipfs/abcdefgh")
        );

        let result = HttpClientWrapper::construct_ipfs_uri(url2, &ipfs_config);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), Errors::InvalidUri);
    }
}
