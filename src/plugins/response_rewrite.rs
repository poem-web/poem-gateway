use std::{collections::HashMap, convert::TryInto, sync::Arc};

use anyhow::{Context, Result};
use bytes::Bytes;
use poem::{
    http::{header, header::HeaderName, HeaderValue, StatusCode},
    Request, Response,
};
use serde::{Deserialize, Serialize};
use tera::Tera;

use crate::{
    config::PluginConfig,
    plugins::{NextPlugin, Plugin, PluginContext},
};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Config {
    status_code: Option<u16>,
    body: Option<String>,
    #[serde(default)]
    body_base64: bool,
    #[serde(default)]
    headers: HashMap<String, String>,
}

#[typetag::serde(name = "responseRewrite")]
#[async_trait::async_trait]
impl PluginConfig for Config {
    async fn create(&self) -> Result<Arc<dyn Plugin>> {
        let status_code = match self.status_code {
            Some(status_code) => Some(
                status_code
                    .try_into()
                    .context("failed to parse the status code")?,
            ),
            None => None,
        };
        let body = match &self.body {
            Some(body) => Some(if self.body_base64 {
                base64::decode(&body)
                    .context("failed to decode the body")?
                    .into()
            } else {
                body.clone().into_bytes().into()
            }),
            None => None,
        };
        let mut headers = HashMap::new();

        for (name, template) in &self.headers {
            let header_name: HeaderName = name
                .parse()
                .with_context(|| format!("failed to parse header name `{}`", name))?;

            let mut tera = Tera::default();
            tera.add_raw_template("value", template)
                .with_context(|| format!("failed to parse the value of header `{}`", name))?;

            headers.insert(header_name, tera);
        }

        Ok(Arc::new(ResponseRewrite {
            status_code,
            body,
            headers,
        }))
    }
}

struct ResponseRewrite {
    status_code: Option<StatusCode>,
    body: Option<Bytes>,
    headers: HashMap<HeaderName, Tera>,
}

#[async_trait::async_trait]
impl Plugin for ResponseRewrite {
    fn priority(&self) -> i32 {
        0
    }

    async fn call(&self, req: Request, ctx: &mut PluginContext, next: NextPlugin<'_>) -> Response {
        let mut resp = next.call(ctx, req).await;

        if let Some(status_code) = self.status_code {
            resp.set_status(status_code);
        }

        if let Some(body) = &self.body {
            resp.headers_mut().remove(header::CONTENT_LENGTH);
            resp.set_body(body.clone());
        }

        let headers = resp.headers_mut();
        for (name, value) in &self.headers {
            let value = ctx.render_template(value, "value");
            if !value.is_empty() {
                if let Ok(header_value) = value.parse::<HeaderValue>() {
                    headers.insert(name.clone(), header_value);
                }
            }
        }

        resp
    }
}
