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
                        lru.push_back(key.clone());
                    }
                    item_map.insert(key, value);
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
                self.evictions.store(0, Ordering::SeqCst);
                self.current_size.store(0, Ordering::SeqCst);
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
    use std::thread;

    use super::*;

    struct DummyData {
        id: u64,
        item_size: u64,
    }

    impl ByteSizeable for DummyData {
        fn size_in_bytes(&self) -> u64 {
            self.item_size
        }
    }

    fn get_item(id: u64, item_size: u64) -> Arc<DummyData> {
        Arc::new(DummyData { id, item_size })
    }

    // Some constants
    static CACHE_SIZE_BYTES: u64 = 65536_u64;
    static ITEM_SIZE_BYTES: u64 = 512_u64;
    static MAX_EXPECTED_ITEMS_IN_CACHE: u64 = CACHE_SIZE_BYTES / ITEM_SIZE_BYTES;

    /// Tests whether the cache size is limited by the size of the items
    /// stored within in.
    #[test]
    fn test_memory_bound_behavior() {
        let expected_item_capacity = 100_000_u64;
        let cache_size_bytes = ITEM_SIZE_BYTES * expected_item_capacity;
        let cache = MemoryBoundedLruCache::new(cache_size_bytes);

        // Initialize
        (0..expected_item_capacity).for_each(|i| {
            cache.put(i.to_string(), get_item(i, ITEM_SIZE_BYTES));
        });
        assert_eq!(cache.len(), expected_item_capacity as usize);
        assert_eq!(cache.current_size.load(Ordering::SeqCst), cache_size_bytes);

        // Inserting items over the expected item capcity should not grow the cache further
        (expected_item_capacity..expected_item_capacity + 1000).for_each(|i| {
            cache.put(i.to_string(), get_item(i, ITEM_SIZE_BYTES));
        });
        assert_eq!(cache.len(), expected_item_capacity as usize);
        assert_eq!(cache.current_size.load(Ordering::SeqCst), cache_size_bytes);

        // An item 4x in size should cause an appropriate number of evictions
        let current_evictions = cache.evictions.load(Ordering::SeqCst);
        cache.put("4x".to_string(), get_item(0, ITEM_SIZE_BYTES * 4));
        assert_eq!(
            cache.evictions.load(Ordering::SeqCst),
            current_evictions + 4
        );
        assert_eq!(cache.len(), (expected_item_capacity - 3) as usize);
        assert_eq!(cache.current_size.load(Ordering::SeqCst), cache_size_bytes);
        cache.remove(&"4x".to_string());
        assert_eq!(
            cache.current_size.load(Ordering::SeqCst),
            cache_size_bytes - (ITEM_SIZE_BYTES * 4)
        );

        // An item too big for the entire cache should not fit
        let result = cache.put("too_big".to_string(), get_item(9001, cache_size_bytes + 1));
        assert!(!result);
        assert!(cache.get(&"too_big".to_string()).is_none());

        // Clearing the cache should reset counters
        cache.clear();
        assert_eq!(cache.len(), 0);
        assert_eq!(cache.current_size.load(Ordering::SeqCst), 0);
        assert_eq!(cache.evictions.load(Ordering::SeqCst), 0);
    }

    /// Tests the LRU functionality of the cache
    #[test]
    fn test_lru_behavior() {
        let cache = MemoryBoundedLruCache::new(CACHE_SIZE_BYTES);

        // Initialize
        (0..MAX_EXPECTED_ITEMS_IN_CACHE).for_each(|i| {
            cache.put(i.to_string(), get_item(i, ITEM_SIZE_BYTES));
        });
        assert_eq!(cache.len(), MAX_EXPECTED_ITEMS_IN_CACHE as usize);
        assert_eq!(cache.current_size.load(Ordering::SeqCst), CACHE_SIZE_BYTES);

        // Access all but the first inserted item, Key = "0"
        (1..MAX_EXPECTED_ITEMS_IN_CACHE).for_each(|i| {
            let item = cache.get(&i.to_string()).unwrap();
            assert_eq!(item.id, i);
        });

        // Key = "0" should be at the front of the lru deq
        {
            let lock = cache.lru.read();
            assert!(lock.is_ok());
            let lru = lock.unwrap();
            assert!(lru[0].eq(&"0".to_string()));
            assert!(lru[lru.len() - 1].eq(&(MAX_EXPECTED_ITEMS_IN_CACHE - 1).to_string()));
        }

        // Insert a new item to trigger eviction since cache is full
        cache.put("new1".to_string(), get_item(9000, ITEM_SIZE_BYTES));

        // Assert that the least recently used item, Key = "0", is gone
        assert!(cache.get(&0.to_string()).is_none());
        assert!(cache.get(&1.to_string()).is_some());

        // Assert that updating in place should not alter the lru size or cause evictions
        let lru_len_before;
        let lru_len_after;
        {
            let lock = cache.lru.read();
            assert!(lock.is_ok());
            let lru = lock.unwrap();
            assert!(lru[lru.len() - 1].eq(&1.to_string()));
            lru_len_before = lru.len();
        }

        cache.put("new1".to_string(), get_item(9001, ITEM_SIZE_BYTES));

        {
            let lock = cache.lru.read();
            assert!(lock.is_ok());
            let lru = lock.unwrap();
            lru_len_after = lru.len();
        }
        assert_eq!(lru_len_before, lru_len_after);
        assert_eq!(lru_len_before as u64, MAX_EXPECTED_ITEMS_IN_CACHE);
        assert!(cache.get(&1.to_string()).is_some());
        assert_eq!(cache.evictions.load(Ordering::SeqCst), 1);

        // Assert that Key = "1" is at the back of the lru deq
        // and Key = "2" is at the front of the deq
        {
            let lock = cache.lru.read();
            assert!(lock.is_ok());
            let lru = lock.unwrap();
            assert!(lru[lru.len() - 1].eq(&1.to_string()));
            assert!(lru[0].eq(&2.to_string()));
        }

        // Insert a new item to trigger eviction since cache is full
        cache.put("new2".to_string(), get_item(9002, ITEM_SIZE_BYTES));

        // Assert that the least recently used item "2" is gone
        assert_eq!(cache.evictions.load(Ordering::SeqCst), 2);
        assert!(cache.get(&2.to_string()).is_none());
    }

    /// Tests whether threaded access to the cache is working
    #[test]
    fn test_thread_locks() {
        let mut children = vec![];
        let cache = Arc::new(MemoryBoundedLruCache::new(CACHE_SIZE_BYTES));

        // Run threaded insert
        for i in 0..MAX_EXPECTED_ITEMS_IN_CACHE {
            let cache_ref = cache.clone();
            children.push(thread::spawn(move || {
                cache_ref.put(i.to_string(), get_item(i, ITEM_SIZE_BYTES));
            }));
        }

        for child in children {
            let _ = child.join();
        }

        // Assert all items have been inserted
        assert_eq!(cache.len() as u64, MAX_EXPECTED_ITEMS_IN_CACHE);
        assert_eq!(cache.evictions.load(Ordering::SeqCst), 0);

        // Threaded access to a single item
        let mut children = vec![];
        let key = MAX_EXPECTED_ITEMS_IN_CACHE / 2;
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
