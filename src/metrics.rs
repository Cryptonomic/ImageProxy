extern crate prometheus;

use lazy_static::lazy_static;
#[cfg(not(target_os = "macos"))]
use prometheus::process_collector::ProcessCollector;
use prometheus::{
    HistogramOpts, HistogramVec, IntCounter, IntCounterVec, IntGauge, IntGaugeVec, Opts, Registry,
};

lazy_static! {
    static ref API_RESPONSE_TIME_BUCKETS: Vec<f64> =
        vec![5.0, 10.0, 20.0, 30.0, 40.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 2000.0, 5000.0,];
    static ref DOCUMENT_SIZE_BUCKETS: Vec<f64> = vec![
        25.0 * 1024_f64,
        50.0 * 1024_f64,
        100.0 * 1024_f64,
        250.0 * 1024_f64,
        500.0 * 1024_f64,
        1000.0 * 1024_f64,
        2000.0 * 1024_f64,
        5000.0 * 1024_f64,
        10000.0 * 1024_f64,
        25000.0 * 1024_f64,
        50000.0 * 1024_f64,
    ];
    pub static ref REGISTRY: Registry = Registry::new();
    pub static ref ERRORS: IntCounter = IntCounter::new("errors", "Total errors").unwrap();
    pub static ref HITS: IntCounter = IntCounter::new("hits", "Total hits").unwrap();
    pub static ref CACHE_METRICS: IntGaugeVec = IntGaugeVec::new(
        Opts::new("cache_metrics", "Cache metics by cache type"),
        &["type", "metric"]
    )
    .unwrap();
    pub static ref API_REQUESTS: IntCounterVec = IntCounterVec::new(
        Opts::new("api_requests", "Api request by method"),
        &["rpc_method"]
    )
    .unwrap();
    pub static ref API_RESPONSE_TIME: HistogramVec = HistogramVec::new(
        HistogramOpts::new("api_response_time", "Api Response Time in milliseconds")
            .buckets(API_RESPONSE_TIME_BUCKETS.clone()),
        &["method"]
    )
    .unwrap();
    pub static ref HTTP_CLIENT_CODES: IntCounterVec = IntCounterVec::new(
        Opts::new("http_client_codes", "HTTP Client status codes"),
        &["status_code"]
    )
    .unwrap();
    pub static ref DOCUMENT: IntCounterVec = IntCounterVec::new(
        Opts::new("document", "Document stats by status"),
        &["status"]
    )
    .unwrap();
    pub static ref TRAFFIC: IntCounterVec =
        IntCounterVec::new(Opts::new("traffic", "Traffic stats in bytes"), &["metric"]).unwrap();
    pub static ref MODERATION: IntCounterVec =
        IntCounterVec::new(Opts::new("moderation", "Moderation stats"), &["metric"]).unwrap();
    pub static ref DOCUMENT_SIZE: HistogramVec = HistogramVec::new(
        HistogramOpts::new("doc_size", "Document Size").buckets(DOCUMENT_SIZE_BUCKETS.clone()),
        &["size_bytes"]
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
    REGISTRY.register(Box::new(DOCUMENT_SIZE.clone())).unwrap();
    REGISTRY.register(Box::new(DOCUMENT.clone())).unwrap();
    REGISTRY.register(Box::new(ERRORS.clone())).unwrap();
    REGISTRY.register(Box::new(CACHE_METRICS.clone())).unwrap();
    REGISTRY.register(Box::new(TRAFFIC.clone())).unwrap();
    REGISTRY
        .register(Box::new(URI_FILTER_BLOCKED.clone()))
        .unwrap();

    #[cfg(not(target_os = "macos"))]
    let pc = ProcessCollector::for_self();
    #[cfg(not(target_os = "macos"))]
    REGISTRY.register(Box::new(pc)).unwrap();
}
