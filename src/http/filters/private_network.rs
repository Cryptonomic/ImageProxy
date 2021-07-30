use hyper::Uri;
use log::{debug, error, warn};

use crate::dns::DnsResolver;

use super::UriFilter;

pub struct PrivateNetworkFilter {
    dns_resolver: Box<dyn DnsResolver + Send + Sync>,
}

impl PrivateNetworkFilter {
    pub fn new(dns_resolver: Box<dyn DnsResolver + Send + Sync>) -> PrivateNetworkFilter {
        PrivateNetworkFilter { dns_resolver }
    }
}

impl UriFilter for PrivateNetworkFilter {
    fn filter(&self, uri: &Uri) -> bool {
        match uri.host() {
            Some(host) => match self.dns_resolver.resolve(host) {
                Ok(ips) => {
                    debug!("Dns resolution, host:{}, ips:{:?}", host, ips);
                    ips.iter()
                        .fold(!ips.is_empty(), |acc, ip| acc & ip.is_global())
                }
                Err(e) => {
                    error!("DNS resolution error for host={}, reason={}", host, e);
                    false
                }
            },
            None => {
                warn!("No host specified in request");
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net::IpAddr;

    use crate::dns::{DummyDnsResolver, StandardDnsResolver};

    use super::*;

    #[test]
    fn test_block_private_1() {
        let private_ip: IpAddr = "10.0.0.2".parse().unwrap();
        let dns_resolver = DummyDnsResolver {
            resolved_address: vec![private_ip],
        };

        let filter = PrivateNetworkFilter::new(Box::new(dns_resolver));
        let private_uri = "http://localhost:8080/image.png".parse().unwrap();
        assert!(!filter.filter(&private_uri));
    }

    #[test]
    fn test_block_private_2() {
        let dns_resolver = StandardDnsResolver {};
        let filter = PrivateNetworkFilter::new(Box::new(dns_resolver));
        let private_uri = "http://localhost:8080/image.png".parse().unwrap();
        assert!(!filter.filter(&private_uri));
    }

    #[test]
    fn test_allow_global() {
        let global_ip1: IpAddr = "8.8.8.8".parse().unwrap();
        let resolver2 = DummyDnsResolver {
            resolved_address: vec![global_ip1],
        };
        let filter = PrivateNetworkFilter::new(Box::new(resolver2));
        let global_uri = "https://www.google.com/image.png".parse().unwrap();
        assert!(filter.filter(&global_uri));
    }

    #[test]
    fn test_block_private_global_mix() {
        let private_ip1: IpAddr = "172.16.10.14".parse().unwrap();
        let global_ip1: IpAddr = "8.8.8.8".parse().unwrap();
        let dns_resolver = DummyDnsResolver {
            resolved_address: vec![global_ip1, private_ip1],
        };
        let filter = PrivateNetworkFilter::new(Box::new(dns_resolver));
        let global_uri = "https://www.google.com/image.png".parse().unwrap();
        assert!(!filter.filter(&global_uri));
    }

    #[test]
    fn test_block_link_local() {
        let dns_resolver = StandardDnsResolver {};
        let filter = PrivateNetworkFilter::new(Box::new(dns_resolver));
        let global_uri = "https://169.254.10.254/image.png".parse().unwrap();
        assert!(!filter.filter(&global_uri));
    }

    #[test]
    fn test_block_broadcast() {
        let dns_resolver = StandardDnsResolver {};
        let filter = PrivateNetworkFilter::new(Box::new(dns_resolver));
        let global_uri = "https://255.255.255.255/image.png".parse().unwrap();
        assert!(!filter.filter(&global_uri));
    }
}
