use anyhow::Result;

#[typetag::serde(tag = "type")]
#[async_trait::async_trait]
pub trait StorageConfig: Send + Sync + 'static {
    async fn create_storage(&self, interval: u64, refill: u32) -> Result<Box<dyn Storage>>;
}

#[async_trait::async_trait]
pub trait Storage: Send + Sync + 'static {
    async fn check(&self, key: String) -> Result<(bool, u32)>;
}
