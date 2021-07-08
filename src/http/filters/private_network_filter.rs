use hyper::Uri;
use serde::Deserialize;
use std::net::Ipv4Addr;

use crate::dns::DnsResolver;

use super::UriFilter;

#[derive(Deserialize, Clone)]
pub struct PrivateNetworkFilterConfig {
    pub allow_private_connections: bool,
    pub allowed_ips: Vec<IpPortPair>,
}

#[derive(Deserialize, Clone)]
pub struct IpPortPair {
    pub destination: Ipv4Addr,
    pub port: u16,
}

pub struct PrivateNetworkFilter {
    dns_resolver: Box<dyn DnsResolver + Send + Sync>,
    allow_private_connections: bool,
    allowed_ips: Vec<IpPortPair>,
}

impl PrivateNetworkFilter {
    pub fn new(
        allow_private_connections: bool,
        allowed_ips: Vec<IpPortPair>,
        dns_resolver: Box<dyn DnsResolver + Send + Sync>,
    ) -> Box<dyn UriFilter + Send + Sync> {
        let filter = PrivateNetworkFilter {
            allow_private_connections,
            allowed_ips,
            dns_resolver,
        };
        Box::new(filter)
    }
}

impl UriFilter for PrivateNetworkFilter {
    fn filter(&self, uri: Uri) -> bool {
        //TODO
        true
    }
}
