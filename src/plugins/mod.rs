mod auth_basic;
mod auth_key;
mod circuit_breaker;
mod cors;
mod limit_count;
mod response_rewrite;

use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use poem::{web::RemoteAddr, Endpoint, Request, Response};
use tera::Tera;

#[derive(Default)]
pub struct PluginContext {
    tera_ctx: tera::Context,
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

        if let RemoteAddr::SocketAddr(addr) = req.remote_addr() {
            tera_ctx.insert("remoteAddr", &addr.ip());
        }

        Self { tera_ctx }
    }

    pub fn render_template(&self, tera: &Tera, name: &str) -> String {
        tera.render(name, &self.tera_ctx).unwrap_or_default()
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
