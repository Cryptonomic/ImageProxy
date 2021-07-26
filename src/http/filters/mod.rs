use hyper::Uri;
use serde::Deserialize;

pub mod private_network;

#[derive(Deserialize, Clone)]
pub enum FilterAction {
    Allow,
    Deny,
}

pub trait UriFilter {
    fn filter(&self, uri: &Uri) -> bool;
}
