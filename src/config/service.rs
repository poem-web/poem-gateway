use std::sync::Arc;

use anyhow::Result;
use dyn_clone::DynClone;
use poem::{Endpoint, Response};
use serde::{Deserialize, Serialize};

use crate::config::PluginConfig;

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ServiceConfig {
    pub name: Option<String>,
    pub endpoint: Box<dyn EndpointConfig>,
    #[serde(default)]
    pub plugins: Vec<Box<dyn PluginConfig>>,
}

#[typetag::serde(tag = "type")]
pub trait EndpointConfig: DynClone + Send + Sync + 'static {
    fn create(&self) -> Result<Arc<dyn Endpoint<Output = Response>>>;
}

dyn_clone::clone_trait_object!(EndpointConfig);
