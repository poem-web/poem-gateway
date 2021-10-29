use std::sync::Arc;

use anyhow::Result;
use jsonschema::JSONSchema;
use poem::{http::StatusCode, IntoResponse, Request, Response};
use serde::{Deserialize, Serialize};

use crate::{
    config::PluginConfig,
    plugins::{NextPlugin, Plugin, PluginContext},
};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Config {
    schema: serde_json::Value,
}

#[typetag::serde(name = "requestValidation")]
#[async_trait::async_trait]
impl PluginConfig for Config {
    async fn create(&self) -> Result<Arc<dyn Plugin>> {
        Ok(Arc::new(RequestValidation {
            schema: JSONSchema::compile(&self.schema).map_err(|err| anyhow::anyhow!("{}", err))?,
        }))
    }
}

struct RequestValidation {
    schema: JSONSchema,
}

#[async_trait::async_trait]
impl Plugin for RequestValidation {
    fn priority(&self) -> i32 {
        0
    }

    async fn call(
        &self,
        mut req: Request,
        ctx: &mut PluginContext,
        next: NextPlugin<'_>,
    ) -> Response {
        let data = match req.take_body().into_vec().await {
            Ok(data) => data,
            Err(err) => return err.into_response(),
        };
        let value = match serde_json::from_slice(&data) {
            Ok(value) => value,
            Err(_) => return StatusCode::BAD_GATEWAY.into(),
        };
        if !self.schema.is_valid(&value) {
            return StatusCode::BAD_GATEWAY.into();
        }
        req.set_body(data);
        next.call(ctx, req).await
    }
}
