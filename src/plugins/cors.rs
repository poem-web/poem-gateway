use std::sync::Arc;

use anyhow::{Context, Result};
use itertools::Itertools;
use poem::{
    http::{header, header::HeaderName, HeaderValue, Method, StatusCode},
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
    #[serde(default)]
    allow_origins: Vec<String>,
    #[serde(default)]
    allow_methods: Vec<String>,
    #[serde(default)]
    allow_headers: Vec<String>,
    #[serde(default)]
    expose_headers: Vec<String>,
    #[serde(default)]
    max_age: i32,
    #[serde(default)]
    allow_credentials: bool,
}

#[typetag::serde(name = "cors")]
#[async_trait::async_trait]
impl PluginConfig for Config {
    async fn create(&self) -> Result<Arc<dyn Plugin>> {
        Ok(Arc::new(Cors {
            allow_origins: self.allow_origins.clone(),
            allow_methods: self
                .allow_methods
                .iter()
                .map(|s| s.parse())
                .try_collect()
                .context("invalid method")?,
            allow_headers: self
                .allow_headers
                .iter()
                .map(|s| s.parse())
                .try_collect()
                .context("invalid allow headers")?,
            expose_headers: self
                .allow_headers
                .iter()
                .map(|s| s.parse())
                .try_collect()
                .context("invalid expose headers")?,
            max_age: self.max_age,
            allow_credentials: self.allow_credentials,
        }))
    }
}

struct Cors {
    allow_origins: Vec<String>,
    allow_methods: Vec<Method>,
    allow_headers: Vec<HeaderName>,
    expose_headers: Vec<HeaderName>,
    max_age: i32,
    allow_credentials: bool,
}

impl Cors {
    fn is_valid_origin(&self, origin: &str) -> bool {
        if self.allow_origins.is_empty() {
            true
        } else {
            self.allow_origins.iter().any(|x| {
                if x == "*" {
                    return true;
                }
                x == origin
            })
        }
    }

    fn build_preflight_response(&self) -> Response {
        let mut builder = Response::builder();

        if !self.allow_origins.is_empty() {
            for origin in &self.allow_origins {
                builder = builder.header(header::ACCESS_CONTROL_ALLOW_ORIGIN, origin.clone());
            }
        } else {
            builder = builder.header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*");
        }

        for method in &self.allow_methods {
            builder = builder.header(header::ACCESS_CONTROL_ALLOW_METHODS, method.as_str());
        }

        if !self.allow_headers.is_empty() {
            for header in &self.allow_headers {
                builder = builder.header(header::ACCESS_CONTROL_ALLOW_HEADERS, header.clone());
            }
        } else {
            builder = builder.header(header::ACCESS_CONTROL_ALLOW_HEADERS, "*");
        }

        builder = builder.header(header::ACCESS_CONTROL_MAX_AGE, self.max_age);

        if self.allow_credentials {
            builder = builder.header(header::ACCESS_CONTROL_ALLOW_CREDENTIALS, "true");
        }

        for header in &self.expose_headers {
            builder = builder.header(header::ACCESS_CONTROL_EXPOSE_HEADERS, header.clone());
        }

        builder.finish()
    }
}

#[async_trait::async_trait]
impl Plugin for Cors {
    fn priority(&self) -> i32 {
        2000
    }

    async fn call(&self, req: Request, ctx: &mut PluginContext, next: NextPlugin<'_>) -> Response {
        let origin = match req.headers().get(header::ORIGIN) {
            Some(origin) => origin.to_str().map(ToString::to_string),
            None => {
                // This is not a CORS request if there is no Origin header
                return next.call(ctx, req).await;
            }
        };
        let origin = match origin {
            Ok(origin) => origin,
            Err(_) => return StatusCode::BAD_REQUEST.into(),
        };

        if !self.is_valid_origin(&origin) {
            return StatusCode::UNAUTHORIZED.into();
        }

        if req.method() == Method::OPTIONS {
            return self.build_preflight_response();
        }

        let mut resp = next.call(ctx, req).await;

        if self.allow_origins.is_empty() {
            resp.headers_mut().insert(
                header::ACCESS_CONTROL_ALLOW_ORIGIN,
                HeaderValue::from_static("*"),
            );
        } else {
            resp.headers_mut().insert(
                header::ACCESS_CONTROL_ALLOW_ORIGIN,
                HeaderValue::from_str(&origin).unwrap(),
            );
        }

        if self.allow_credentials {
            resp.headers_mut().insert(
                header::ACCESS_CONTROL_ALLOW_CREDENTIALS,
                HeaderValue::from_static("true"),
            );
        }

        resp.headers_mut().extend(
            self.expose_headers
                .iter()
                .map(|value| (header::ACCESS_CONTROL_EXPOSE_HEADERS, value.clone().into())),
        );

        resp
    }
}
