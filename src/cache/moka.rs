use std::{
    convert::TryInto,
    sync::atomic::{AtomicI64, Ordering},
};

use super::{Cache, Key, Value};
use moka::sync::Cache as MokaCache;
use serde::Deserialize;

pub struct InMemoryCache {
    cache: MokaCache<Key, Value>,
    hit: AtomicI64,
    miss: AtomicI64,
    insert: AtomicI64,
    eviction: AtomicI64,
    max_cache_size_mb: u64,
}

#[derive(Deserialize, Clone)]
pub struct InMemoryCacheConfig {
    pub max_cache_size_mb: u64,
}

impl Cache for InMemoryCache {
    fn put(&self, key: &Key, value: &Value) -> bool {
        self.cache.insert(key.clone(), value.clone());
        self.insert.fetch_add(1, Ordering::SeqCst);
        true
    }

    fn get(&self, key: &Key) -> Option<Value> {
        let item = self.cache.get(key);
        if item.is_some() {
            self.hit.fetch_add(1, Ordering::SeqCst);
        } else {
            self.miss.fetch_add(1, Ordering::SeqCst);
        }
        item
    }

    fn len(&self) -> usize {
        self.cache.entry_count() as usize
    }

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn clear(&self) {
        self.cache.invalidate_all();
    }

    fn gather_metrics(&self, metrics: &prometheus::IntGaugeVec) {
        let current_eviction = self.insert.load(Ordering::SeqCst) - self.len() as i64;
        let eviction = self.eviction.fetch_add(current_eviction, Ordering::SeqCst);
        let bytes_used = self
            .cache
            .iter()
            .fold(0_i64, |acc, (_, v)| acc + v.bytes.len() as i64);
        metrics
            .with_label_values(&["memorycache", "items"])
            .set(self.len() as i64);
        metrics
            .with_label_values(&["memorycache", "hit"])
            .set(self.hit.load(Ordering::SeqCst));
        metrics
            .with_label_values(&["memorycache", "miss"])
            .set(self.miss.load(Ordering::SeqCst));
        metrics
            .with_label_values(&["memorycache", "insert"])
            .set(self.insert.load(Ordering::SeqCst));
        metrics
            .with_label_values(&["memorycache", "eviction"])
            .set(eviction + current_eviction);
        metrics
            .with_label_values(&["memorycache", "mem_total_bytes"])
            .set(self.max_cache_size_mb as i64 * 1024_i64 * 1024_i64);
        metrics
            .with_label_values(&["memorycache", "mem_used_bytes"])
            .set(bytes_used);
    }
}

impl InMemoryCache {
    pub fn new(config: &InMemoryCacheConfig) -> Self {
        let cache: MokaCache<Key, Value> = MokaCache::builder()
            .weigher(|_key, value: &Value| -> u32 {
                value.bytes.len().try_into().unwrap_or(u32::MAX)
            })
            .max_capacity(config.max_cache_size_mb)
            .build();
        InMemoryCache {
            cache,
            hit: AtomicI64::new(0),
            miss: AtomicI64::new(0),
            insert: AtomicI64::new(0),
            eviction: AtomicI64::new(0),
            max_cache_size_mb: config.max_cache_size_mb,
        }
    }
}
