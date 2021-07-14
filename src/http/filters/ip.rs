use hyper::Uri;
use ipnet::IpNet;
use serde::Deserialize;

use super::FilterAction;
use super::UriFilter;

#[derive(Deserialize, Clone)]
pub struct IpFilterConfig {
    pub default_action: FilterAction,
    pub rules: Vec<IpFilterItem>,
}

#[derive(Deserialize, Clone)]
pub struct IpFilterItem {
    pub destination: IpNet,
    pub action: FilterAction,
}

pub struct IpFilter {
    config: IpFilterConfig,
}

impl IpFilter {
    pub fn new(config: IpFilterConfig) -> IpFilter {
        IpFilter { config: config }
    }
}

impl UriFilter for IpFilter {
    fn filter(&self, uri: &Uri) -> bool {
        
        true
    }
}
