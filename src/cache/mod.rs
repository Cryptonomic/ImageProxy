use log::{error, warn};
use serde::Deserialize;
use std::sync::Arc;
use std::u64;

use prometheus::IntGaugeVec;

use crate::document::Document;

use self::moka::{InMemoryCache, InMemoryCacheConfig};

pub mod moka;

// K: 'static + Hash + Eq + Clone + Send + Sync,
// V: 'static + Send + Sync,
pub type Key = String;
pub type Value = Arc<Document>;

/// Cache for storing
pub trait Cache {
    fn put(&self, key: &Key, value: &Value) -> bool;
    fn get(&self, key: &Key) -> Option<Value>;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
    fn clear(&self);
    fn gather_metrics(&self, metrics: &IntGaugeVec);
}

/// Types of cache implementation available
#[derive(Deserialize, Clone, Debug)]
pub enum CacheType {
    InMemoryCache,
    HybridCache,
    RedisCache,
    DiskCache,
    S3Cache,
    None,
}

/// Config struct for overall cache configuration
#[derive(Deserialize, Clone)]
pub struct CacheConfig {
    pub cache_type: CacheType,
    pub in_memory_cache_config: Option<InMemoryCacheConfig>,
}

/// Factory method for cache
pub fn get_cache(config: &CacheConfig) -> Option<Box<dyn Cache + Send + Sync>> {
    match &config.cache_type {
        CacheType::InMemoryCache => {
            if let Some(in_memory_cache_config) = &config.in_memory_cache_config {
                Some(Box::new(InMemoryCache::new(in_memory_cache_config)))
            } else {
                error!("Configuration missing for InMemoryCache. Caching is disabled");
                None
            }
        }
        _ => {
            warn!("Caching is disabled via configuration");
            None
        }
    }
}
