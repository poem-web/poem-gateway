use std::{collections::HashSet, str::FromStr, sync::Arc};

use anyhow::{Context, Result};
use itertools::Itertools;
use poem::{
    http::{header, header::HeaderName, HeaderValue, Method, StatusCode},
    web::headers::{
        AccessControlAllowHeaders, AccessControlAllowMethods, AccessControlExposeHeaders,
        HeaderMapExt,
    },
    Request, Response,
};
use serde::{Deserialize, Serialize};

use crate::{
    config::PluginConfig,
    plugins::{NextPlugin, Plugin, PluginContext},
};

#[derive(Serialize, Deserialize, Clone)]
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
        let allow_origins: HashSet<HeaderValue> = self
            .allow_origins
            .iter()
            .map(|s| s.parse())
            .try_collect()
            .context("invalid origin")?;
        let allow_methods: HashSet<Method> = self
            .allow_methods
            .iter()
            .map(|s| s.parse())
            .try_collect()
            .context("invalid method")?;
        let allow_headers: HashSet<HeaderName> = self
            .allow_headers
            .iter()
            .map(|s| s.parse())
            .try_collect()
            .context("invalid allow headers")?;
        let expose_headers: HashSet<HeaderName> = self
            .allow_headers
            .iter()
            .map(|s| s.parse())
            .try_collect()
            .context("invalid expose headers")?;

        Ok(Arc::new(Cors {
            allow_origins,
            allow_methods: allow_methods.clone(),
            allow_headers: allow_headers.clone(),
            expose_headers: expose_headers.clone(),
            allow_headers_header: allow_headers.into_iter().collect(),
            allow_methods_header: allow_methods.into_iter().collect(),
            expose_headers_header: expose_headers.into_iter().collect(),
            max_age: self.max_age,
            allow_credentials: self.allow_credentials,
        }))
    }
}

struct Cors {
    allow_origins: HashSet<HeaderValue>,
    allow_headers: HashSet<HeaderName>,
    allow_methods: HashSet<Method>,
    expose_headers: HashSet<HeaderName>,
    allow_headers_header: AccessControlAllowHeaders,
    allow_methods_header: AccessControlAllowMethods,
    expose_headers_header: AccessControlExposeHeaders,
    max_age: i32,
    allow_credentials: bool,
}

impl Cors {
    fn is_valid_origin(&self, origin: &HeaderValue) -> (bool, bool) {
        if self.allow_origins.contains(origin) {
            return (true, false);
        }
        (self.allow_origins.is_empty(), true)
    }

    fn build_preflight_response(&self, origin: &HeaderValue) -> Response {
        let mut builder = Response::builder()
            .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, origin)
            .typed_header(self.expose_headers_header.clone())
            .header(header::ACCESS_CONTROL_MAX_AGE, self.max_age);

        if self.allow_methods.is_empty() {
            builder = builder.typed_header(
                [
                    Method::GET,
                    Method::POST,
                    Method::PUT,
                    Method::DELETE,
                    Method::HEAD,
                    Method::OPTIONS,
                    Method::CONNECT,
                    Method::PATCH,
                    Method::TRACE,
                ]
                .iter()
                .cloned()
                .collect::<AccessControlAllowMethods>(),
            );
        } else {
            builder = builder.typed_header(self.allow_methods_header.clone());
        }

        if self.allow_headers.is_empty() {
            builder = builder.header(header::ACCESS_CONTROL_ALLOW_HEADERS, "*");
        } else {
            builder = builder.typed_header(self.allow_headers_header.clone());
        }

        if self.allow_credentials {
            builder = builder.header(header::ACCESS_CONTROL_ALLOW_CREDENTIALS, "true");
        }

        builder.body(())
    }
}

#[async_trait::async_trait]
impl Plugin for Cors {
    fn priority(&self) -> i32 {
        2000
    }

    async fn call(&self, req: Request, ctx: &mut PluginContext, next: NextPlugin<'_>) -> Response {
        let origin = match req.headers().get(header::ORIGIN) {
            Some(origin) => origin.clone(),
            None => {
                // This is not a CORS request if there is no Origin header
                return next.call(ctx, req).await;
            }
        };

        let (origin_is_allow, vary_header) = self.is_valid_origin(&origin);
        if !origin_is_allow {
            return StatusCode::UNAUTHORIZED.into();
        }

        if req.method() == Method::OPTIONS {
            let allow_method = req
                .headers()
                .get(header::ACCESS_CONTROL_REQUEST_METHOD)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<Method>().ok())
                .map(|method| {
                    if self.allow_methods.is_empty() {
                        true
                    } else {
                        self.allow_methods.contains(&method)
                    }
                });
            if !matches!(allow_method, Some(true)) {
                return StatusCode::UNAUTHORIZED.into();
            }

            let allow_headers = {
                let mut allow_headers = true;
                if !self.allow_headers.is_empty() {
                    if let Some(request_header) =
                        req.headers().get(header::ACCESS_CONTROL_REQUEST_HEADERS)
                    {
                        allow_headers = false;
                        if let Ok(s) = request_header.to_str() {
                            for header in s.split(',') {
                                if let Ok(header) = HeaderName::from_str(header.trim()) {
                                    if self.allow_headers.contains(&header) {
                                        allow_headers = true;
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
                allow_headers
            };

            if !allow_headers {
                return StatusCode::UNAUTHORIZED.into();
            }

            return self.build_preflight_response(&origin);
        }

        let mut resp = next.call(ctx, req).await;

        resp.headers_mut()
            .insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, origin);

        if self.allow_credentials {
            resp.headers_mut().insert(
                header::ACCESS_CONTROL_ALLOW_CREDENTIALS,
                HeaderValue::from_static("true"),
            );
        }

        if !self.expose_headers.is_empty() {
            resp.headers_mut()
                .typed_insert(self.expose_headers_header.clone());
        }

        if vary_header {
            resp.headers_mut()
                .insert(header::VARY, HeaderValue::from_static("Origin"));
        }

        resp
    }
}
