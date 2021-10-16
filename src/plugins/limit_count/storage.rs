use anyhow::Result;
use dyn_clone::DynClone;

#[typetag::serde(tag = "type")]
#[async_trait::async_trait]
pub trait StorageConfig: DynClone + Send + Sync + 'static {
    async fn create_storage(&self, interval: u64, refill: u32) -> Result<Box<dyn Storage>>;
}

dyn_clone::clone_trait_object!(StorageConfig);

#[async_trait::async_trait]
pub trait Storage: Send + Sync + 'static {
    async fn check(&self, key: String) -> Result<(bool, u32)>;
}
