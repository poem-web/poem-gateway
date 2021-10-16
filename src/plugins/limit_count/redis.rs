use anyhow::Result;
use chrono::Utc;
use redis::{
    aio::ConnectionManager, Client, ConnectionAddr, ConnectionInfo, RedisConnectionInfo, Script,
};
use serde::{Deserialize, Serialize};

use crate::plugins::limit_count::storage::{Storage, StorageConfig};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct RedisStorageConfig {
    host: String,
    port: u16,
    #[serde(default)]
    database: i64,
    #[serde(default)]
    username: Option<String>,
    #[serde(default)]
    password: Option<String>,
}

impl RedisStorageConfig {
    fn to_connection_info(&self) -> ConnectionInfo {
        ConnectionInfo {
            addr: ConnectionAddr::Tcp(self.host.clone(), self.port),
            redis: RedisConnectionInfo {
                db: self.database,
                username: self.username.clone(),
                password: self.password.clone(),
            },
        }
    }
}

#[typetag::serde(name = "redis")]
#[async_trait::async_trait]
impl StorageConfig for RedisStorageConfig {
    async fn create_storage(&self, interval: u64, refill: u32) -> Result<Box<dyn Storage>> {
        Ok(Box::new(RedisStorage::new(interval, refill, self).await?))
    }
}

struct RedisStorage {
    interval: u64,
    refill: u32,
    conn: ConnectionManager,
    script: Script,
}

impl RedisStorage {
    async fn new(interval: u64, refill: u32, cfg: &RedisStorageConfig) -> Result<Self> {
        let connection_info = cfg.to_connection_info();
        let client = Client::open(connection_info)?;
        let script = Script::new(include_str!("limiter.lua"));
        Ok(Self {
            interval,
            refill,
            conn: ConnectionManager::new(client).await?,
            script,
        })
    }
}

#[async_trait::async_trait]
impl Storage for RedisStorage {
    async fn check(&self, key: String) -> Result<(bool, u32)> {
        let mut invocation = self.script.prepare_invoke();
        let now = Utc::now();
        let now_ms = now.timestamp() * 1000 + i64::from(now.timestamp_subsec_millis());
        let expire = self.interval * 2 + 15;

        invocation
            .key(&key)
            .arg(self.interval * 1000)
            .arg(self.refill)
            .arg(now_ms)
            .arg(expire);

        match invocation
            .invoke_async::<_, (bool, u32)>(&mut self.conn.clone())
            .await?
        {
            (true, remaining_tokens) => Ok((true, remaining_tokens)),
            (false, _) => Ok((false, 0)),
        }
    }
}
