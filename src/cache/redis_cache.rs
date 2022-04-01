use crate::metrics;

use super::{ByteSizeable, Cache, RedisCacheConfig};
use deadpool_redis::{
    redis::{cmd, FromRedisValue},
    Config, Pool, Runtime,
};
use log::{error, warn};
use prometheus::IntGaugeVec;
use redis::Cmd;
use std::sync::Arc;
use std::{hash::Hash, marker::PhantomData};

pub struct RedisCache<K, V> {
    pool: Pool,
    _phantom_key: PhantomData<K>,
    _phantom_value: PhantomData<V>,
}

impl<K, V> RedisCache<K, V>
where
    K: 'static + Hash + Eq + Clone + Send + Sync,
    V: 'static + ByteSizeable + Send + Sync,
{
    pub fn new(conf: &RedisCacheConfig) -> RedisCache<K, V> {
        let url = format!(
            "{}://{}:{}",
            conf.server.protocol, conf.server.host, conf.server.port
        );
        let cfg = Config::from_url(url);
        let pool = cfg.create_pool(Some(Runtime::Tokio1)).unwrap();
        RedisCache {
            pool,
            _phantom_key: PhantomData,
            _phantom_value: PhantomData,
        }
    }

    fn execute<T: FromRedisValue>(&self, cmd: Cmd) -> Option<T> {
        let mut con = self.pool.get().await;
        match con {
            Ok(connection) => {
                cmd.query_async(connnection)
                    .await
                    .map_err(|e| {
                        error!("Unable to query redis, reason={}", e);
                        metrics::ERRORS.inc(); //TODO: add redis specific error
                        e
                    })
                    .ok()
            }
            Err(e) => {
                error!("Unable to query redis, reason={}", e);
                metrics::ERRORS.inc(); //TODO: add redis specific error
                None
            }
        }
    }
}

impl<K, V> Cache<K, V> for RedisCache<K, V>
where
    K: 'static + Hash + Eq + Clone + Send + Sync,
    V: 'static + ByteSizeable + Send + Sync,
{
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn len(&self) -> usize {
        match self.execute::<i64>(cmd("DBSIZE")) {
            Some(size) => size as usize,
            None => 0,
        }
    }

    fn put(&self, key: K, value: Arc<V>) -> bool {
        todo!()
    }

    fn get(&self, key: &K) -> Option<Arc<V>> {
        todo!()
    }

    fn remove(&self, key: &K) -> Option<Arc<V>> {
        todo!()
    }

    fn clear(&self) {
        todo!()
    }

    fn gather_metrics(&self, metric: &IntGaugeVec) {
        // metric
        //     .with_label_values(&["memorycache", "items"])
        //     .set(self.len() as i64);
        // metric
        //     .with_label_values(&["memorycache", "mem_total_bytes"])
        //     .set(self.max_size_in_bytes as i64);
        // metric
        //     .with_label_values(&["memorycache", "hits"])
        //     .set(self.hits.load(Ordering::SeqCst));
        // metric
        //     .with_label_values(&["memorycache", "misses"])
        //     .set(self.misses.load(Ordering::SeqCst));
        // metric
        //     .with_label_values(&["memorycache", "evictions"])
        //     .set(self.evictions.load(Ordering::SeqCst));
        // metric
        //     .with_label_values(&["memorycache", "mem_used_bytes"])
        //     .set(self.current_size.load(Ordering::SeqCst) as i64);
    }
}
