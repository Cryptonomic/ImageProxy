use log::{error, warn};
use serde::Deserialize;
use std::hash::Hash;
use std::sync::Arc;
use std::u64;

use prometheus::IntGaugeVec;

use crate::document::Document;

pub mod moka;

// K: 'static + Hash + Eq + Clone + Send + Sync,
// V: 'static + Send + Sync,
pub type Key = str;
pub type Value = Arc<Document>;

/// Cache for storing
pub trait Cache {
    fn put(&self, key: &Key, value: &Value) -> bool;
    fn get(&self, key: &Key) -> Option<Value>;
    fn remove(&self, key: &Key) -> Option<Value>;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
    fn clear(&self);
    fn gather_metrics(&self, metrics: &IntGaugeVec);
}

/// Types of cache implementation available
#[derive(Deserialize, Clone, Debug)]
pub enum CacheType {
    InMemoryCache,
    RedisCache,
    DiskCache,
    S3Cache,
    None,
}

/// Config struct for overall cache configuration
#[derive(Deserialize, Clone)]
pub struct CacheConfig {
    pub cache_type: CacheType,
}

/// Factory method for cache
pub fn get_cache(config: &CacheConfig) -> Option<Box<dyn Cache + Send + Sync>> {
    None
}
