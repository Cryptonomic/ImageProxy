use log::{error, warn};
use serde::Deserialize;
use std::hash::Hash;
use std::sync::Arc;
use std::u64;

use prometheus::IntGaugeVec;

use self::memory_cache::MemoryBoundedLruCache;

pub mod memory_cache;

/// Cache for storing any key / value pair.
pub trait Cache<K, V>
where
    K: 'static + Hash + Eq + Clone,
{
    fn put(&self, key: K, value: Arc<V>);
    fn get(&self, key: &K) -> Option<Arc<V>>;
    fn remove(&self, key: &K);
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
    fn clear(&self);
    fn gather_metrics(&self, metrics: &IntGaugeVec);
}

/// Implemented by cache values which need to report runtime memory size
pub trait ByteSizeable {
    fn size_in_bytes(&self) -> u64;
}

/// Types of cache implementation available
#[derive(Deserialize, Clone, Debug)]
pub enum CacheType {
    MemoryBoundedLruCache,
    DiskCache,
    None,
}

/// Config struct for InMemoryBoundLruCache
#[derive(Deserialize, Clone)]
pub struct InMemorySizeBoundedLruCacheConfig {
    pub max_cache_size_mb: u64,
}

/// Config struct for Disk Cache
#[derive(Deserialize, Clone)]
pub struct DiskCacheConfig {
    pub cache_path: Option<String>,
}

/// Config struct for overall cache configuration
#[derive(Deserialize, Clone)]
pub struct CacheConfig {
    pub cache_type: CacheType,
    pub memory_cache_config: InMemorySizeBoundedLruCacheConfig,
    pub disk_cache_config: DiskCacheConfig,
}

/// Factory method for cache
pub fn get_cache<K, V>(config: &CacheConfig) -> Option<Arc<Box<dyn Cache<K, V> + Send + Sync>>>
where
    K: 'static + Hash + Eq + Clone + Send + Sync,
    V: 'static + ByteSizeable + Send + Sync,
{
    match config.cache_type {
        CacheType::MemoryBoundedLruCache => {
            let cache = MemoryBoundedLruCache::new(
                config.memory_cache_config.max_cache_size_mb * 1024 * 1024,
            );
            Some(Arc::new(Box::new(cache)))
        }
        CacheType::DiskCache => {
            error!("Cache type:{:?} not supported yet", config.cache_type);
            None
        }
        CacheType::None => {
            warn!("Caching is disabled via configuration");
            None
        }
    }
}
