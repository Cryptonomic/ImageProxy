use std::net::IpAddr;

use dns_lookup::lookup_host;

/// A trait that allows one to use different DNS implementations
pub trait DnsResolver {
    fn resolve(&self, host: &str) -> Result<Vec<IpAddr>, std::io::Error>;
}

/// A DNS resolver based on the crate dns_lookup
#[derive(Clone)]
pub struct StandardDnsResolver {}

impl DnsResolver for StandardDnsResolver {
    fn resolve(&self, host: &str) -> Result<Vec<IpAddr>, std::io::Error> {
        lookup_host(host)
    }
}

/// A dummy resolver for use with tests
pub struct DummyDnsResolver {
    pub resolved_address: Vec<IpAddr>,
}

impl DnsResolver for DummyDnsResolver {
    fn resolve(&self, _host: &str) -> Result<Vec<IpAddr>, std::io::Error> {
        Ok(self.resolved_address.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dummy_dns_resolver() {
        let ip: IpAddr = "127.0.0.1".parse().unwrap();
        let ip2: IpAddr = "10.0.0.2".parse().unwrap();
        let ip_vec = vec![ip.clone(), ip2.clone()];
        let resolver = DummyDnsResolver {
            resolved_address: ip_vec,
        };
        let hostname = String::from("some_host_name");
        let resolved_results = resolver.resolve(&hostname).unwrap();
        assert!(resolved_results.contains(&ip));
        assert!(resolved_results.contains(&ip2));
        assert_eq!(resolved_results.len(), 2);
    }
}
