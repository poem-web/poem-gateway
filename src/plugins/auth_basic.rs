use std::sync::Arc;

use anyhow::Result;
use poem::{
    web::{
        headers,
        headers::{authorization::Basic, HeaderMapExt},
    },
    Request,
};
use serde::{Deserialize, Serialize};

use crate::{config::AuthPluginConfig, plugins::AuthPlugin};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Config {
    username: String,
    password: String,
}

#[typetag::serde(name = "basic")]
#[async_trait::async_trait]
impl AuthPluginConfig for Config {
    async fn create(&self) -> Result<Arc<dyn AuthPlugin>> {
        Ok(Arc::new(BasicAuth {
            username: self.username.clone(),
            password: self.password.clone(),
        }))
    }
}

struct BasicAuth {
    username: String,
    password: String,
}

#[async_trait::async_trait]
impl AuthPlugin for BasicAuth {
    async fn auth(&self, req: &Request) -> bool {
        if let Some(auth) = req.headers().typed_get::<headers::Authorization<Basic>>() {
            if self.username == auth.0.username() && self.password == auth.0.password() {
                return true;
            }
        }
        false
    }
}
