use std::{
    cmp::Ordering,
    collections::BinaryHeap,
    time::{Duration, Instant},
};

use anyhow::Result;
use dashmap::DashMap;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::plugins::limit_count::storage::{Storage, StorageConfig};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MemoryStorageConfig {}

#[typetag::serde(name = "memory")]
#[async_trait::async_trait]
impl StorageConfig for MemoryStorageConfig {
    async fn create_storage(&self, interval: u64, refill: u32) -> Result<Box<dyn Storage>> {
        Ok(Box::new(MemoryStorage::new(interval, refill)))
    }
}

struct ExpireItem {
    expire_at: Instant,
    key: String,
}

impl PartialEq for ExpireItem {
    fn eq(&self, other: &Self) -> bool {
        self.expire_at == other.expire_at
    }
}

impl Eq for ExpireItem {}

impl PartialOrd for ExpireItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(other.expire_at.cmp(&self.expire_at))
    }
}

impl Ord for ExpireItem {
    fn cmp(&self, other: &Self) -> Ordering {
        other.expire_at.cmp(&self.expire_at)
    }
}

struct MemoryStorage {
    interval: Duration,
    refill: u32,
    map: DashMap<String, (Instant, u32)>,
    expire_queue: Mutex<BinaryHeap<ExpireItem>>,
}

impl MemoryStorage {
    fn new(interval: u64, refill: u32) -> Self {
        Self {
            interval: Duration::from_secs(interval),
            refill,
            map: Default::default(),
            expire_queue: Default::default(),
        }
    }

    fn clear_expired_keys(&self, now: Instant) {
        let mut expire_queue = self.expire_queue.lock();

        while let Some(item) = expire_queue.peek() {
            if now > item.expire_at {
                self.map.remove(&item.key);
                expire_queue.pop();
            } else {
                break;
            }
        }
    }
}

#[async_trait::async_trait]
impl Storage for MemoryStorage {
    async fn check(&self, key: String) -> Result<(bool, u32)> {
        let now = Instant::now();
        self.clear_expired_keys(now);

        let mut item = self.map.entry(key).or_insert_with(|| (now, self.refill));
        let (last_fill_at, tokens) = item.value_mut();

        if now - *last_fill_at > self.interval {
            *tokens = self.refill;
        }

        if *tokens > 0 {
            *tokens -= 1;
            Ok((true, *tokens))
        } else {
            Ok((false, 0))
        }
    }
}
