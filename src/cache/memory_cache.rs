use log::{error, warn};
use prometheus::IntGaugeVec;
use std::collections::VecDeque;
use std::hash::Hash;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicI64, AtomicU64, Ordering},
        Arc, RwLock,
    },
};

use super::{ByteSizeable, Cache};

pub struct MemoryBoundedLruCache<K, V> {
    items: RwLock<HashMap<K, Arc<V>>>,
    lru: RwLock<VecDeque<K>>,
    max_size_in_bytes: u64,
    hits: AtomicI64,
    misses: AtomicI64,
    evictions: AtomicI64,
    current_size: AtomicU64,
}

impl<K, V> MemoryBoundedLruCache<K, V>
where
    K: 'static + Hash + Eq + Clone + Send + Sync,
    V: 'static + ByteSizeable + Send + Sync,
{
    pub fn new(max_size_in_bytes: u64) -> MemoryBoundedLruCache<K, V> {
        MemoryBoundedLruCache {
            items: RwLock::new(HashMap::new()),
            lru: RwLock::new(VecDeque::new()),
            max_size_in_bytes,
            hits: AtomicI64::new(0),
            misses: AtomicI64::new(0),
            evictions: AtomicI64::new(0),
            current_size: AtomicU64::new(0),
        }
    }
}

impl<K, V> Cache<K, V> for MemoryBoundedLruCache<K, V>
where
    K: 'static + Hash + Eq + Clone + Send + Sync,
    V: 'static + ByteSizeable + Send + Sync,
{
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn len(&self) -> usize {
        match self.items.read() {
            Ok(item_map) => item_map.len(),
            Err(e) => {
                error!("Item cache is poisoned reason: {}, a write error was possibly encountered elsewhere", e);
                usize::MIN
            }
        }
    }

    fn put(&self, key: K, value: Arc<V>) -> bool {
        if value.size_in_bytes() >= self.max_size_in_bytes {
            warn!("Item size is bigger than configured cache size");
            false
        } else {
            match (self.items.write(), self.lru.write()) {
                (Ok(mut item_map), Ok(mut lru)) => {
                    if !item_map.contains_key(&key) {
                        let current_size = self.current_size.load(Ordering::SeqCst);
                        if self.max_size_in_bytes - current_size < value.size_in_bytes() {
                            let mut evicted_size = 0_u64;
                            let mut evicted_count = 0;
                            while evicted_size < value.size_in_bytes() && lru.len() > 0 {
                                if let Some(k) = lru.pop_front() {
                                    if let Some(item) = item_map.remove(&k) {
                                        evicted_size += item.size_in_bytes();
                                        evicted_count += 1;
                                    }
                                }
                            }
                            self.current_size.fetch_sub(evicted_size, Ordering::SeqCst);
                            self.evictions
                                .fetch_add(evicted_count as i64, Ordering::SeqCst);
                        }
                        self.current_size
                            .fetch_add(value.size_in_bytes(), Ordering::SeqCst);
                    }
                    item_map.insert(key.clone(), value);
                    lru.push_back(key);
                    true
                }
                _ => {
                    error!("Item or Lru cache is poisoned, a write error was possibly encountered elsewhere");
                    false
                }
            }
        }
    }

    fn get(&self, key: &K) -> Option<Arc<V>> {
        match (self.items.read(), self.lru.write()) {
            (Ok(item_map), Ok(mut lru)) => {
                if let Some(item) = item_map.get(key) {
                    self.hits.fetch_add(1, Ordering::SeqCst);
                    if let Some(index) = lru.iter().rposition(|k| k.eq(key)) {
                        lru.remove(index);
                        lru.push_back(key.clone());
                    }
                    Some(item.clone())
                } else {
                    self.misses.fetch_add(1, Ordering::SeqCst);
                    None
                }
            }
            _ => {
                error!("Item cache is poisoned, a write error was possibly encountered elsewhere");
                None
            }
        }
    }

    fn remove(&self, key: &K) -> Option<Arc<V>> {
        match (self.items.write(), self.lru.write()) {
            (Ok(mut item_map), Ok(mut lru)) => {
                if let Some(item) = item_map.remove(key) {
                    self.current_size
                        .fetch_sub(item.size_in_bytes(), Ordering::SeqCst);
                    if let Some(index) = lru.iter().rposition(|k| k.eq(key)) {
                        lru.remove(index);
                    }
                    Some(item)
                } else {
                    None
                }
            }
            _ => {
                error!("Item cache is poisoned, a write error was possibly encountered elsewhere");
                None
            }
        }
    }

    fn clear(&self) {
        match (self.items.write(), self.lru.write()) {
            (Ok(mut item_map), Ok(mut lru)) => {
                item_map.clear();
                lru.clear();
            }
            _ => error!("Item cache is poisoned, a write error was possibly encountered elsewhere"),
        }
    }

    fn gather_metrics(&self, metric: &IntGaugeVec) {
        metric
            .with_label_values(&["memorycache", "items"])
            .set(self.len() as i64);
        metric
            .with_label_values(&["memorycache", "mem_total_bytes"])
            .set(self.max_size_in_bytes as i64);
        metric
            .with_label_values(&["memorycache", "hits"])
            .set(self.hits.load(Ordering::SeqCst));
        metric
            .with_label_values(&["memorycache", "misses"])
            .set(self.misses.load(Ordering::SeqCst));
        metric
            .with_label_values(&["memorycache", "evictions"])
            .set(self.evictions.load(Ordering::SeqCst));
        metric
            .with_label_values(&["memorycache", "mem_used_bytes"])
            .set(self.current_size.load(Ordering::SeqCst) as i64);
    }
}

#[cfg(test)]
mod tests {
    use core::time;
    use std::thread;

    use super::*;

    struct DummyData {
        id: u64,
        item_size: u64,
        drop_counter: Option<Arc<AtomicU64>>,
    }

    impl ByteSizeable for DummyData {
        fn size_in_bytes(&self) -> u64 {
            self.item_size
        }
    }

    impl Drop for DummyData {
        fn drop(&mut self) {
            self.drop_counter
                .as_ref()
                .map(|c| c.fetch_add(1, Ordering::SeqCst));
        }
    }

    fn get_item(id: u64, item_size: u64) -> Arc<DummyData> {
        Arc::new(DummyData {
            id,
            item_size,
            drop_counter: None,
        })
    }

    fn get_item_with_drop_counter(
        id: u64,
        item_size: u64,
        drop_counter: Arc<AtomicU64>,
    ) -> Arc<DummyData> {
        Arc::new(DummyData {
            id,
            item_size,
            drop_counter: Some(drop_counter),
        })
    }

    /// Tests whether the cache size is limited by the size of the items
    /// stored within in.
    #[test]
    fn test_memory_bound_behavior() {
        let max_cache_size_in_bytes = 512_u64 * 100_000_u64;
        let item_size_in_bytes = 512_u64;
        let expected_cache_capacity = max_cache_size_in_bytes / item_size_in_bytes;

        let cache = MemoryBoundedLruCache::new(max_cache_size_in_bytes);

        // Insert many times the expected value
        (0..expected_cache_capacity + 1000).for_each(|i| {
            cache.put(i.to_string(), get_item(i, item_size_in_bytes));
        });

        assert_eq!(cache.len(), expected_cache_capacity as usize);
        assert_eq!(
            cache.current_size.load(Ordering::SeqCst),
            max_cache_size_in_bytes
        );

        // Insert an item with four times the size of a regular item
        // and assert that the cache len has reduced by 3
        cache.put("4x".to_string(), get_item(0, item_size_in_bytes * 4));
        assert_eq!(cache.len(), (expected_cache_capacity - 3) as usize);
        assert_eq!(
            cache.current_size.load(Ordering::SeqCst),
            max_cache_size_in_bytes
        );
        cache.remove(&"4x".to_string());
        assert_eq!(
            cache.current_size.load(Ordering::SeqCst),
            max_cache_size_in_bytes - (item_size_in_bytes * 4)
        );

        // Insert an item that should not fit into the cache as per mem limit
        cache.put(
            "too_big".to_string(),
            get_item(9001, max_cache_size_in_bytes + 1),
        );

        // Assert that the least recently used item is gone
        assert!(cache.get(&"too_big".to_string()).is_none());

        cache.clear();
        assert_eq!(cache.len(), 0);
    }

    /// Tests whether cache values are dropped and not accidentally being held on to.
    /// This test is probably not required
    #[test]
    fn test_mem_cleanup() {
        let max_cache_size_in_bytes = 65536_u64;
        let item_size_in_bytes = 1024_u64;
        let expected_cache_capacity = max_cache_size_in_bytes / item_size_in_bytes;
        let drop_counter = Arc::new(AtomicU64::new(0));
        let cache = MemoryBoundedLruCache::new(max_cache_size_in_bytes);

        (0..expected_cache_capacity).for_each(|i| {
            cache.put(
                i.to_string(),
                get_item_with_drop_counter(i, item_size_in_bytes, drop_counter.clone()),
            );
        });

        // Fetch all values and assert nothing has been dropped
        (0..expected_cache_capacity).for_each(|i| {
            let item = cache.get(&i.to_string());
            assert!(item.is_some());
            let item = item.unwrap();
            assert_eq!(item.id, i);
        });
        assert_eq!(drop_counter.load(Ordering::SeqCst), 0);

        // Fetch an item and hold on to it
        let long_lived_item = cache.get(&0.to_string());

        // Remove all the values from the cache
        (0..expected_cache_capacity).for_each(|i| {
            cache.remove(&i.to_string());
        });

        // Assert that everything but the captured item was dropped
        assert_eq!(
            drop_counter.load(Ordering::SeqCst),
            expected_cache_capacity - 1
        );
        // Asset that the cache has no reference to the captured item
        assert_eq!(cache.len(), 0);
        let check_item = cache.get(&0.to_string());
        assert!(check_item.is_none());
        // Consume the long lived item
        assert_eq!(long_lived_item.map(|i| i.id).unwrap_or(9000_u64), 0);
        // Check the counter again
        assert_eq!(drop_counter.load(Ordering::SeqCst), expected_cache_capacity);
    }

    /// Tests the LRU functionality of the cache
    #[test]
    fn test_lru_behavior() {
        let max_cache_size_in_bytes = 65536_u64;
        let item_size_in_bytes = 1024_u64;
        let expected_cache_capacity = max_cache_size_in_bytes / item_size_in_bytes;

        let cache = MemoryBoundedLruCache::new(max_cache_size_in_bytes);

        (0..expected_cache_capacity).for_each(|i| {
            cache.put(i.to_string(), get_item(i, item_size_in_bytes));
        });

        assert_eq!(cache.len(), expected_cache_capacity as usize);

        // Access all but the first inserted item, Key = "0"
        (1..expected_cache_capacity).for_each(|i| {
            let item = cache.get(&i.to_string()).unwrap();
            assert_eq!(item.id, i);
            thread::sleep(time::Duration::from_millis(10))
        });

        // Insert a new item to trigger eviction since cache is full
        cache.put("new1".to_string(), get_item(9000, item_size_in_bytes));

        // Assert that the least recently used item, Key = "0", is gone
        assert!(cache.get(&0.to_string()).is_none());
        assert!(cache.get(&1.to_string()).is_some());

        // Update in place and assert no eviction took place
        cache.put("new1".to_string(), get_item(9001, item_size_in_bytes));

        assert!(cache.get(&1.to_string()).is_some());
        assert_eq!(cache.evictions.load(Ordering::SeqCst), 1);

        // Insert a new item to trigger eviction since cache is full
        cache.put("new2".to_string(), get_item(9002, item_size_in_bytes));

        // Assert that the least recently used item "2" is gone
        assert_eq!(cache.evictions.load(Ordering::SeqCst), 2);
        assert!(cache.get(&2.to_string()).is_none());
    }

    /// Tests whether threaded access to the cache is working
    #[test]
    fn test_thread_locks() {
        let mut children = vec![];
        let max_cache_size_in_bytes = 65536_u64;
        let item_size_in_bytes = 1024_u64;
        let expected_cache_capacity = max_cache_size_in_bytes / item_size_in_bytes;
        let cache = Arc::new(MemoryBoundedLruCache::new(max_cache_size_in_bytes));

        // Run threaded insert
        for i in 0..expected_cache_capacity {
            let cache_ref = cache.clone();
            children.push(thread::spawn(move || {
                cache_ref.put(i.to_string(), get_item(i, item_size_in_bytes));
            }));
        }

        for child in children {
            let _ = child.join();
        }

        // Assert all items have been inserted
        assert_eq!(cache.len() as u64, expected_cache_capacity);
        assert_eq!(cache.evictions.load(Ordering::SeqCst), 0);

        // Threaded access to a single item
        let mut children = vec![];
        let key = expected_cache_capacity / 2;
        for _ in 0..8192 {
            let cache_ref = cache.clone();
            children.push(thread::spawn(move || {
                let item = cache_ref.get(&key.to_string());
                assert!(item.is_some());
                let item = item.unwrap();
                assert_eq!(item.id, key);
            }));
        }

        for child in children {
            let _ = child.join();
        }

        // Item should be at the tail of the lru queue
        let lock = cache.lru.read();
        assert!(lock.is_ok());
        let lock = lock.unwrap();
        let lru_entry = lock.back();
        assert!(lru_entry.is_some());
        assert!(lru_entry.unwrap().eq(&key.to_string()));
    }
}
