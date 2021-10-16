mod auth_basic;
mod auth_key;
mod circuit_breaker;
mod consumer_restriction;
mod cors;
mod limit_count;
mod prometheus;
mod request_id;
mod response_rewrite;

use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use poem::{web::RemoteAddr, Addr, Endpoint, Request, Response};
use tera::Tera;

#[derive(Default)]
pub struct PluginContext {
    tera_ctx: tera::Context,
    consumer_name: Option<String>,
}

impl Deref for PluginContext {
    type Target = tera::Context;

    fn deref(&self) -> &Self::Target {
        &self.tera_ctx
    }
}

impl DerefMut for PluginContext {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.tera_ctx
    }
}

impl PluginContext {
    pub fn new(req: &Request) -> Self {
        let mut tera_ctx = tera::Context::default();

        if let RemoteAddr(Addr::SocketAddr(addr)) = req.remote_addr() {
            tera_ctx.insert("remoteAddr", &addr.ip());
        }

        Self {
            tera_ctx,
            consumer_name: None,
        }
    }

    pub fn render_template(&self, tera: &Tera, name: &str) -> String {
        tera.render(name, &self.tera_ctx).unwrap_or_default()
    }

    pub fn set_consumer_name(&mut self, name: impl Into<String>) {
        self.consumer_name = Some(name.into());
    }

    pub fn consumer_name(&self) -> Option<&str> {
        self.consumer_name.as_deref()
    }
}

pub struct NextPlugin<'a> {
    chain: &'a [Arc<dyn Plugin>],
    endpoint: &'a dyn Endpoint<Output = Response>,
}

impl<'a> NextPlugin<'a> {
    #[inline]
    pub fn new(
        chain: &'a [Arc<dyn Plugin>],
        endpoint: &'a dyn Endpoint<Output = Response>,
    ) -> Self {
        Self { chain, endpoint }
    }

    pub async fn call(self, ctx: &mut PluginContext, req: Request) -> Response {
        if let Some((first, next)) = self.chain.split_first() {
            first
                .call(
                    req,
                    ctx,
                    NextPlugin {
                        chain: next,
                        endpoint: self.endpoint,
                    },
                )
                .await
        } else {
            self.endpoint.call(req).await
        }
    }
}

#[async_trait::async_trait]
pub trait Plugin: Sync + Send + 'static {
    fn priority(&self) -> i32;

    async fn call(&self, req: Request, ctx: &mut PluginContext, next: NextPlugin<'_>) -> Response;
}

#[async_trait::async_trait]
pub trait AuthPlugin: Sync + Send + 'static {
    async fn auth(&self, req: &Request) -> bool;
}
