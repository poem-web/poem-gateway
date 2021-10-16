use std::sync::Arc;

use anyhow::Result;
use dyn_clone::DynClone;
use serde::{Deserialize, Serialize};

use crate::{
    config::{AuthPluginConfig, PluginConfig},
    consumer_filters::ConsumerFilter,
};

#[typetag::serde(tag = "type")]
pub trait ConsumerFilterConfig: DynClone + Send + Sync + 'static {
    fn create(&self) -> Result<Arc<dyn ConsumerFilter>>;
}

dyn_clone::clone_trait_object!(ConsumerFilterConfig);

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ConsumerConfig {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub filters: Vec<Box<dyn ConsumerFilterConfig>>,
    #[serde(default)]
    pub auth: Option<Box<dyn AuthPluginConfig>>,
    #[serde(default)]
    pub plugins: Vec<Box<dyn PluginConfig>>,
}
