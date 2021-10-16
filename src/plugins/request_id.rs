use std::sync::Arc;

use anyhow::{Context, Result};
use poem::{
    http::{header::HeaderName, HeaderValue},
    Request, Response,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    config::PluginConfig,
    plugins::{NextPlugin, Plugin, PluginContext},
};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Config {
    #[serde(default = "default_header_name")]
    header_name: String,
    #[serde(default)]
    include_in_response: bool,
}

fn default_header_name() -> String {
    "X-Request-Id".to_string()
}

#[typetag::serde(name = "requestId")]
#[async_trait::async_trait]
impl PluginConfig for Config {
    async fn create(&self) -> Result<Arc<dyn Plugin>> {
        Ok(Arc::new(RequestId {
            header_name: self.header_name.parse().context("invalid header name")?,
            include_in_response: self.include_in_response,
        }))
    }
}

struct RequestId {
    header_name: HeaderName,
    include_in_response: bool,
}

#[async_trait::async_trait]
impl Plugin for RequestId {
    fn priority(&self) -> i32 {
        1000
    }

    async fn call(
        &self,
        mut req: Request,
        ctx: &mut PluginContext,
        next: NextPlugin<'_>,
    ) -> Response {
        let id = Uuid::new_v4().to_string();
        let id = HeaderValue::from_str(&id).unwrap();
        req.headers_mut()
            .insert(self.header_name.clone(), id.clone());
        let mut resp = next.call(ctx, req).await;
        if self.include_in_response {
            resp.headers_mut()
                .insert(self.header_name.clone(), id.clone());
        }
        resp
    }
}
