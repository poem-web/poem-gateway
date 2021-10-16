use std::sync::Arc;

use anyhow::Result;
use dyn_clone::DynClone;

use crate::plugins::{AuthPlugin, Plugin};

#[typetag::serde(tag = "type")]
#[async_trait::async_trait]
pub trait PluginConfig: DynClone + Send + Sync + 'static {
    async fn create(&self) -> Result<Arc<dyn Plugin>>;
}

dyn_clone::clone_trait_object!(PluginConfig);

#[typetag::serde(tag = "type")]
#[async_trait::async_trait]
pub trait AuthPluginConfig: DynClone + Send + Sync + 'static {
    async fn create(&self) -> Result<Arc<dyn AuthPlugin>>;
}

dyn_clone::clone_trait_object!(AuthPluginConfig);
