use serde::{Deserialize, Serialize};

use crate::config::{PluginConfig, ServiceConfig};

#[derive(Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum ServiceRef {
    Reference(String),
    Inline(ServiceConfig),
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RouteConfig {
    pub path: String,
    #[serde(default = "default_strip")]
    pub strip: bool,
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub plugins: Vec<Box<dyn PluginConfig>>,
    #[serde(rename = "service")]
    pub service_ref: ServiceRef,
}

const fn default_strip() -> bool {
    true
}
