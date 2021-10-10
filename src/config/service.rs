use std::sync::Arc;

use anyhow::Result;
use poem::{Endpoint, Response};
use serde::{Deserialize, Serialize};

use crate::config::PluginConfig;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceConfig {
    pub name: String,
    pub target: Box<dyn ServiceTargetConfig>,
    #[serde(default)]
    pub plugins: Vec<Box<dyn PluginConfig>>,
}

#[typetag::serde(tag = "type")]
pub trait ServiceTargetConfig: Send + Sync + 'static {
    fn create(&self) -> Result<Arc<dyn Endpoint<Output = Response>>>;
}
