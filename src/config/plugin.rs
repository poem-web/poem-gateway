use std::sync::Arc;

use anyhow::Result;

use crate::plugins::{AuthPlugin, Plugin};

#[typetag::serde(tag = "type")]
#[async_trait::async_trait]
pub trait PluginConfig: Send + Sync + 'static {
    async fn create(&self) -> Result<Arc<dyn Plugin>>;
}

#[typetag::serde(tag = "type")]
#[async_trait::async_trait]
pub trait AuthPluginConfig: Send + Sync + 'static {
    async fn create(&self) -> Result<Arc<dyn AuthPlugin>>;
}
