extern crate prometheus;

use lazy_static::lazy_static;
use prometheus::{HistogramOpts, HistogramVec, IntCounter, IntGauge, Registry};

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();
    pub static ref ERRORS: IntCounter = IntCounter::new("errors", "Total errors").unwrap();
    pub static ref HITS: IntCounter = IntCounter::new("hits", "Total hits").unwrap();
    pub static ref MODERATION_REQUESTS: IntCounter =
        IntCounter::new("mod_requests", "Moderation Requests").unwrap();
    pub static ref API_REQUESTS: IntCounter =
        IntCounter::new("api_requests", "Api Requests").unwrap();
    pub static ref API_REQUESTS_FETCH: IntCounter =
        IntCounter::new("api_fetch", "Api Fetch Requests").unwrap();
    pub static ref API_REQUESTS_DESCRIBE: IntCounter =
        IntCounter::new("api_describe", "Api Describe Requests").unwrap();
    pub static ref API_REQUESTS_REPORT: IntCounter =
        IntCounter::new("api_report", "Api Report Requests").unwrap();
    pub static ref API_RESPONSE_TIME: HistogramVec = HistogramVec::new(
        HistogramOpts::new("api_response_time", "Api Response Time"),
        &["method"]
    )
    .unwrap();
    pub static ref ACTIVE_CLIENTS: IntGauge =
        IntGauge::new("active_clients", "Active clients").unwrap();
    pub static ref DOCUMENTS_BLOCKED: IntCounter =
        IntCounter::new("docs_blocked", "Documents Blocked").unwrap();
    pub static ref DOCUMENTS_FORCED: IntCounter =
        IntCounter::new("docs_forced", "Documents force fetched").unwrap();
    pub static ref DOCUMENTS_FETCHED_ERROR: IntCounter =
        IntCounter::new("docs_fetched_errors", "Documents fetched error").unwrap();
    pub static ref DOCUMENTS_FETCHED: IntCounter =
        IntCounter::new("docs_fetched", "Documents fetched").unwrap();
    pub static ref UPTIME: IntCounter = IntCounter::new("uptime", "Service Uptime").unwrap();
    pub static ref BYTES_FETCHED: IntCounter =
        IntCounter::new("bytes_fetched", "Bytes Fetched").unwrap();
    pub static ref BYTES_SENT: IntCounter = IntCounter::new("bytes_sent", "Bytes Sent").unwrap();
    pub static ref BYTES_SENT_MODERATION: IntCounter =
        IntCounter::new("bytes_sent_mod", "Bytes sent for moderation").unwrap();
    pub static ref CACHE_HITS: IntCounter =
        IntCounter::new("cache_hit", "Moderation Cache Hit").unwrap();
    pub static ref CACHE_MISS: IntCounter =
        IntCounter::new("cache_miss", "Moderation Cache Miss").unwrap();
    pub static ref DOCUMENT_SIZE: HistogramVec = HistogramVec::new(
        HistogramOpts::new("doc_size", "Document Size"),
        &["size_bytes"]
    )
    .unwrap();
    pub static ref MEM_VIRT: IntGauge =
        IntGauge::new("mem_virt", "Total virtual memory size kb").unwrap();
    pub static ref MEM_RSS: IntGauge =
        IntGauge::new("mem_rss", "Total resident memory size kb").unwrap();
}

pub fn init_registry() {
    REGISTRY.register(Box::new(HITS.clone())).unwrap();
    REGISTRY
        .register(Box::new(MODERATION_REQUESTS.clone()))
        .unwrap();
    REGISTRY.register(Box::new(API_REQUESTS.clone())).unwrap();
    REGISTRY
        .register(Box::new(API_REQUESTS_FETCH.clone()))
        .unwrap();
    REGISTRY
        .register(Box::new(API_REQUESTS_DESCRIBE.clone()))
        .unwrap();
    REGISTRY
        .register(Box::new(API_REQUESTS_REPORT.clone()))
        .unwrap();
    REGISTRY
        .register(Box::new(API_RESPONSE_TIME.clone()))
        .unwrap();
    REGISTRY.register(Box::new(UPTIME.clone())).unwrap();
    REGISTRY.register(Box::new(BYTES_FETCHED.clone())).unwrap();
    REGISTRY.register(Box::new(BYTES_SENT.clone())).unwrap();
    REGISTRY
        .register(Box::new(BYTES_SENT_MODERATION.clone()))
        .unwrap();
    REGISTRY.register(Box::new(CACHE_HITS.clone())).unwrap();
    REGISTRY.register(Box::new(CACHE_MISS.clone())).unwrap();
    REGISTRY.register(Box::new(DOCUMENT_SIZE.clone())).unwrap();
    REGISTRY
        .register(Box::new(DOCUMENTS_BLOCKED.clone()))
        .unwrap();
    REGISTRY
        .register(Box::new(DOCUMENTS_FORCED.clone()))
        .unwrap();
    REGISTRY
        .register(Box::new(DOCUMENTS_FETCHED.clone()))
        .unwrap();
    REGISTRY
        .register(Box::new(DOCUMENTS_FETCHED_ERROR.clone()))
        .unwrap();
    REGISTRY.register(Box::new(MEM_VIRT.clone())).unwrap();
    REGISTRY.register(Box::new(MEM_RSS.clone())).unwrap();
    REGISTRY.register(Box::new(ERRORS.clone())).unwrap();
}
