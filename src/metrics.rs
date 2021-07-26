extern crate prometheus;

use lazy_static::lazy_static;
use prometheus::{
    HistogramOpts, HistogramVec, IntCounter, IntCounterVec, IntGauge, IntGaugeVec, Opts, Registry,
};

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();
    pub static ref ACTIVE_CLIENTS: IntGauge =
        IntGauge::new("active_clients", "Active clients").unwrap();
    pub static ref ERRORS: IntCounter = IntCounter::new("errors", "Total errors").unwrap();
    pub static ref HITS: IntCounter = IntCounter::new("hits", "Total hits").unwrap();
    pub static ref API_REQUESTS: IntCounterVec = IntCounterVec::new(
        Opts::new("api_requests", "Api request by method"),
        &["rpc_method"]
    )
    .unwrap();
    pub static ref API_RESPONSE_TIME: HistogramVec = HistogramVec::new(
        HistogramOpts::new("api_response_time", "Api Response Time"),
        &["method"]
    )
    .unwrap();
    pub static ref DOCUMENT: IntCounterVec = IntCounterVec::new(
        Opts::new("document", "Document stats by status"),
        &["status"]
    )
    .unwrap();
    pub static ref TRAFFIC: IntCounterVec =
        IntCounterVec::new(Opts::new("traffic", "Traffic stats in bytes"), &["metric"]).unwrap();
    pub static ref START_TIME: IntGauge =
        IntGauge::new("start_time", "Service start time").unwrap();
    pub static ref MODERATION: IntCounterVec =
        IntCounterVec::new(Opts::new("moderation", "Moderation stats"), &["metric"]).unwrap();
    pub static ref DOCUMENT_SIZE: HistogramVec = HistogramVec::new(
        HistogramOpts::new("doc_size", "Document Size"),
        &["size_bytes"]
    )
    .unwrap();
    pub static ref MEMORY: IntGaugeVec = IntGaugeVec::new(
        Opts::new("memory", "Process memory statistics"),
        &["metric"]
    )
    .unwrap();
    pub static ref URI_FILTER_BLOCKED: IntCounter = IntCounter::new(
        "uri_filter_block",
        "Number of times the filter blocked a host"
    )
    .unwrap();
}

pub fn init_registry() {
    REGISTRY.register(Box::new(HITS.clone())).unwrap();
    REGISTRY.register(Box::new(MODERATION.clone())).unwrap();
    REGISTRY.register(Box::new(API_REQUESTS.clone())).unwrap();
    REGISTRY
        .register(Box::new(API_RESPONSE_TIME.clone()))
        .unwrap();
    REGISTRY.register(Box::new(START_TIME.clone())).unwrap();
    REGISTRY.register(Box::new(DOCUMENT_SIZE.clone())).unwrap();
    REGISTRY.register(Box::new(DOCUMENT.clone())).unwrap();
    REGISTRY.register(Box::new(ERRORS.clone())).unwrap();
    REGISTRY
        .register(Box::new(URI_FILTER_BLOCKED.clone()))
        .unwrap();
}
