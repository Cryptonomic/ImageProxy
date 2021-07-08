use hyper::Uri;
use serde::Deserialize;

pub mod hostname_filter;
pub mod ip_filter;
pub mod private_network_filter;

#[derive(Deserialize, Clone)]
pub enum FilterAction {
    Allow,
    Deny,
}

pub trait UriFilter {
    fn filter(&self, uri: Uri) -> bool;
}
