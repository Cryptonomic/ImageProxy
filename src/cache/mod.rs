use log::{error, warn};
use serde::Deserialize;
use std::hash::Hash;
use std::sync::Arc;
use std::u64;

use prometheus::IntGaugeVec;

use crate::config::Host;

use self::memory_cache::MemoryBoundedLruCache;
use self::redis_cache::RedisCache;

pub mod memory_cache;
pub mod redis_cache;

/// Cache for storing any key / value pair.
pub trait Cache<K, V>
where
    K: 'static + Hash + Eq + Clone,
{
    fn put(&self, key: K, value: Arc<V>) -> bool;
    fn get(&self, key: &K) -> Option<Arc<V>>;
    fn remove(&self, key: &K) -> Option<Arc<V>>;
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
    Redis,
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

/// Config struct for RedisCache
#[derive(Deserialize, Clone)]
pub struct RedisCacheConfig {
    pub server: Host,
    pub username: Option<String>,
    pub password: Option<String>,
}

/// Config struct for overall cache configuration
#[derive(Deserialize, Clone)]
pub struct CacheConfig {
    pub cache_type: CacheType,
    pub memory_cache_config: Option<InMemorySizeBoundedLruCacheConfig>,
    pub disk_cache_config: Option<DiskCacheConfig>,
    pub redis_cache_config: Option<RedisCacheConfig>,
}

// TODO Refactor this to return Result
/// Factory method for cache
pub fn get_cache<K, V>(config: &CacheConfig) -> Option<Box<dyn Cache<K, V> + Send + Sync>>
where
    K: 'static + Hash + Eq + Clone + Send + Sync,
    V: 'static + ByteSizeable + Send + Sync,
{
    match config.cache_type {
        CacheType::MemoryBoundedLruCache => {
            if let Some(memory_cache_config) = &config.memory_cache_config {
                let cache =
                    MemoryBoundedLruCache::new(memory_cache_config.max_cache_size_mb * 1024 * 1024);
                Some(Box::new(cache))
            } else {
                error!("Memory Bound Lru Cache is misssing configuration");
                None
            }
        }
        CacheType::DiskCache => {
            error!("Cache type:{:?} not supported yet", config.cache_type);
            None
        }
        CacheType::Redis => {
            if let Some(redis_cache_config) = &config.redis_cache_config {
                let cache = RedisCache::new(redis_cache_config);
                Some(Box::new(cache))
            } else {
                error!("Redis Cache is missing configuration");
                None
            }
        }
        CacheType::None => {
            warn!("Caching is disabled via configuration");
            None
        }
    }
}
