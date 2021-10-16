mod memory;
mod redis;
mod storage;

use std::{convert::TryInto, sync::Arc};

use anyhow::{Context, Result};
use poem::{http::StatusCode, IntoResponse, Request, Response};
use serde::{Deserialize, Serialize};

use crate::{
    config::PluginConfig,
    plugins::{
        limit_count::{
            memory::MemoryStorageConfig,
            storage::{Storage, StorageConfig},
        },
        NextPlugin, Plugin, PluginContext,
    },
};

#[derive(Serialize, Deserialize, Copy, Clone)]
#[serde(rename_all = "camelCase")]
enum Key {
    RemoteIp,
    XRealIp,
    XForwardedFor,
    ConsumerName,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Config {
    #[serde(default = "default_interval")]
    interval: u64,
    #[serde(default = "default_refill")]
    refill: u32,
    #[serde(default = "default_key")]
    key: Key,
    #[serde(default = "default_rejected_code")]
    rejected_code: u16,
    #[serde(default)]
    rejected_msg: Option<String>,
    #[serde(default = "default_show_limit_quota_header")]
    show_limit_quota_header: bool,
    #[serde(default = "default_storage_config")]
    storage: Box<dyn StorageConfig>,
    #[serde(default)]
    redis_host: Option<String>,
    #[serde(default)]
    redis_password: Option<String>,
    #[serde(default)]
    redis_database: Option<usize>,
}

const fn default_interval() -> u64 {
    1
}

fn default_refill() -> u32 {
    1
}

const fn default_key() -> Key {
    Key::RemoteIp
}

const fn default_rejected_code() -> u16 {
    503
}

fn default_storage_config() -> Box<dyn StorageConfig> {
    Box::new(MemoryStorageConfig {})
}

const fn default_show_limit_quota_header() -> bool {
    true
}

#[typetag::serde(name = "limitCount")]
#[async_trait::async_trait]
impl PluginConfig for Config {
    async fn create(&self) -> Result<Arc<dyn Plugin>> {
        Ok(Arc::new(LimitCount {
            refill: self.refill,
            key: Key::RemoteIp,
            rejected_code: self
                .rejected_code
                .try_into()
                .with_context(|| format!("invalid rejected code `{}`", self.rejected_code))?,
            rejected_msg: self.rejected_msg.clone(),
            show_limit_quota_header: self.show_limit_quota_header,
            storage: self
                .storage
                .create_storage(self.interval, self.refill)
                .await
                .with_context(|| "failed to create storage for `limitCount` plugin")?,
        }))
    }
}

struct LimitCount {
    refill: u32,
    key: Key,
    rejected_code: StatusCode,
    rejected_msg: Option<String>,
    show_limit_quota_header: bool,
    storage: Box<dyn Storage>,
}

impl LimitCount {
    fn add_quota_headers(&self, resp: impl IntoResponse, remain_tokens: u32) -> Response {
        if self.show_limit_quota_header {
            resp.with_header("X-RateLimit-Limit", format!("{}", self.refill))
                .with_header("X-RateLimit-Remaining", format!("{}", remain_tokens))
                .into_response()
        } else {
            resp.into_response()
        }
    }
}

#[async_trait::async_trait]
impl Plugin for LimitCount {
    fn priority(&self) -> i32 {
        1000
    }

    async fn call(&self, req: Request, ctx: &mut PluginContext, next: NextPlugin<'_>) -> Response {
        let key = match &self.key {
            Key::RemoteIp => req
                .remote_addr()
                .as_socket_addr()
                .map(|ip| ip.ip().to_string())
                .unwrap_or_default(),
            Key::XRealIp => req
                .headers()
                .get("x-real-ip")
                .and_then(|value| value.to_str().ok().map(ToString::to_string))
                .unwrap_or_default(),
            Key::XForwardedFor => req
                .headers()
                .get("x-forwarded-for")
                .and_then(|value| value.to_str().ok().map(ToString::to_string))
                .unwrap_or_default(),
            Key::ConsumerName => ctx
                .get("consumerName")
                .and_then(|value| value.as_str().map(ToString::to_string))
                .unwrap_or_default(),
        };

        match self.storage.check(key).await {
            Ok((true, remaining_tokens)) => {
                self.add_quota_headers(next.call(ctx, req).await, remaining_tokens)
            }
            Ok((false, _)) => self.add_quota_headers(
                Response::builder()
                    .status(self.rejected_code)
                    .body(self.rejected_msg.clone().unwrap_or_default()),
                0,
            ),
            Err(err) => {
                error!(error = %err, "failed to check the limit count");
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .finish()
            }
        }
    }
}
