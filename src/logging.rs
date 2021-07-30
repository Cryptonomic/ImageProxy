use log::Record;
use log4rs::{
    config::{Deserialize, Deserializers},
    filter::{Filter, Response},
};

#[derive(serde::Deserialize)]
pub struct ProxyLoggingFilterConfig {}

#[derive(Debug)]
pub struct ProxyLoggingFilter {}

/// This filter accepts all log events from the top level logger / module.
/// Everything else is rejected.
impl Filter for ProxyLoggingFilter {
    fn filter(&self, record: &Record) -> Response {
        match record.module_path() {
            Some(module_path) if module_path.eq("nft_image_proxy") => Response::Accept,
            _ => Response::Reject,
        }
    }
}

pub struct ProxyFilterDeserializer;

impl Deserialize for ProxyFilterDeserializer {
    type Trait = dyn Filter;

    type Config = ProxyLoggingFilterConfig;

    fn deserialize(
        &self,
        _: ProxyLoggingFilterConfig,
        _: &Deserializers,
    ) -> anyhow::Result<Box<dyn Filter>> {
        Ok(Box::new(ProxyLoggingFilter {}))
    }
}

/// Initializes logging for the application
pub fn logging_init() {
    let mut custom_log_deserializer = log4rs::config::Deserializers::new();
    custom_log_deserializer.insert("proxy_filter", ProxyFilterDeserializer);
    log4rs::init_file("log4rs.yml", custom_log_deserializer).unwrap();
}
