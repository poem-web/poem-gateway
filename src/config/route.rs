use serde::{Deserialize, Serialize};

use crate::config::PluginConfig;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteConfig {
    pub path: String,
    #[serde(default)]
    pub strip: bool,
    #[serde(default)]
    pub plugins: Vec<Box<dyn PluginConfig>>,
    pub service: String,
}
