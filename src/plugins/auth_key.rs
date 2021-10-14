use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use poem::{self, Request};
use serde::{Deserialize, Serialize};

use crate::{config::AuthPluginConfig, plugins::AuthPlugin};

#[derive(Serialize, Deserialize, Copy, Clone, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
enum KeyIn {
    Header,
    Query,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Config {
    key: String,
    #[serde(default = "default_key_name")]
    key_name: String,
    #[serde(rename = "in")]
    key_in: KeyIn,
}

fn default_key_name() -> String {
    "apikey".to_string()
}

#[typetag::serde(name = "basic")]
#[async_trait::async_trait]
impl AuthPluginConfig for Config {
    async fn create(&self) -> Result<Arc<dyn AuthPlugin>> {
        Ok(Arc::new(KeyAuth {
            key: self.key.clone(),
            key_name: self.key_name.clone(),
            key_in: self.key_in,
        }))
    }
}

struct KeyAuth {
    key: String,
    key_name: String,
    key_in: KeyIn,
}

#[async_trait::async_trait]
impl AuthPlugin for KeyAuth {
    async fn auth(&self, req: &Request) -> bool {
        let key = match self.key_in {
            KeyIn::Header => req
                .headers()
                .get(&self.key)
                .and_then(|value| value.to_str().ok()),
            KeyIn::Query => {
                match serde_urlencoded::from_str::<HashMap<&str, &str>>(
                    req.uri().query().unwrap_or_default(),
                ) {
                    Ok(values) => values.get(self.key_name.as_str()).copied(),
                    Err(_) => None,
                }
            }
        };

        key == Some(&self.key)
    }
}
