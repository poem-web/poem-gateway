use std::{sync::Arc, time::Duration};

use anyhow::Result;
use bytes::Bytes;
use parking_lot::RwLock;
use poem::{
    http::{HeaderMap, StatusCode},
    Request, Response,
};
use serde::{Deserialize, Serialize};

use crate::{
    config::PluginConfig,
    plugins::{NextPlugin, Plugin, PluginContext},
};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Config {
    break_response_code: u16,
    #[serde(default = "default_start_breaker_sec")]
    start_breaker_sec: u64,
    #[serde(default = "default_max_breaker_sec")]
    max_breaker_sec: u64,
    #[serde(default = "default_failures")]
    failures: u32,
}

const fn default_start_breaker_sec() -> u64 {
    2
}

const fn default_max_breaker_sec() -> u64 {
    60
}

const fn default_failures() -> u32 {
    3
}

#[typetag::serde(name = "circuitBreaker")]
#[async_trait::async_trait]
impl PluginConfig for Config {
    async fn create(&self) -> Result<Arc<dyn Plugin>> {
        Ok(Arc::new(CircuitBreaker {
            break_response_code: Default::default(),
            cb: failsafe::Config::new()
                .failure_policy(failsafe::failure_policy::consecutive_failures(
                    self.failures,
                    failsafe::backoff::exponential(
                        Duration::from_secs(self.start_breaker_sec),
                        Duration::from_secs(self.max_breaker_sec),
                    ),
                ))
                .build(),
            last_err_resp: Default::default(),
        }))
    }
}

struct CircuitBreaker<T> {
    break_response_code: StatusCode,
    cb: T,
    last_err_resp: RwLock<Option<(StatusCode, HeaderMap, Bytes)>>,
}

#[async_trait::async_trait]
impl<T> Plugin for CircuitBreaker<T>
where
    T: failsafe::futures::CircuitBreaker + Send + Sync + 'static,
{
    fn priority(&self) -> i32 {
        100
    }

    async fn call(&self, req: Request, ctx: &mut PluginContext, next: NextPlugin<'_>) -> Response {
        let break_response_code = self.break_response_code;
        match self
            .cb
            .call(async move {
                let resp = next.call(ctx, req).await;
                if resp.status() != break_response_code {
                    Ok(resp)
                } else {
                    Err(resp)
                }
            })
            .await
        {
            Ok(resp) => {
                *self.last_err_resp.write() = None;
                resp
            }
            Err(failsafe::Error::Inner(mut resp)) => {
                let status = resp.status();
                let headers = resp.headers().clone();
                let body = resp.take_body().into_bytes().await.ok().unwrap_or_default();

                *self.last_err_resp.write() = Some((status, headers, body.clone()));
                resp.set_body(body);
                resp
            }
            Err(failsafe::Error::Rejected) => {
                let last_err_resp = self.last_err_resp.read();
                let (status, headers, body) = last_err_resp.as_ref().unwrap();
                let mut resp = Response::builder().status(*status).body(body.clone());
                *resp.headers_mut() = headers.clone();
                resp
            }
        }
    }
}
