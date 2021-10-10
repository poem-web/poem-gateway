pub mod providers;

mod consumer;
mod listener;
mod plugin;
mod provider;
mod route;
mod service;

use std::{cmp::Reverse, collections::HashMap, sync::Arc};

use anyhow::Result;
use poem::{
    http::StatusCode,
    listener::{AcceptorExt, BoxAcceptor},
    Endpoint, IntoResponse, Request, Response, Route, Server,
};
use serde::Deserialize;

pub use crate::config::{
    consumer::{ConsumerConfig, ConsumerFilterConfig},
    listener::ListenerConfig,
    plugin::{AuthPluginConfig, PluginConfig},
    provider::ConfigProvider,
    route::RouteConfig,
    service::{ServiceConfig, ServiceTargetConfig},
};
use crate::{
    consumer_filters::ConsumerFilter,
    plugins::{AuthPlugin, NextPlugin, Plugin, PluginContext},
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    #[serde(default = "default_allow_anonymous")]
    allow_anonymous: bool,
    #[serde(default)]
    pub listeners: Vec<Box<dyn ListenerConfig>>,
    #[serde(default)]
    pub consumers: Vec<ConsumerConfig>,
    #[serde(default)]
    pub routes: Vec<RouteConfig>,
    #[serde(default)]
    pub services: Vec<ServiceConfig>,
    #[serde(default)]
    pub global_plugins: Vec<Box<dyn PluginConfig>>,
}

const fn default_allow_anonymous() -> bool {
    true
}

impl Config {
    pub async fn create_server(&self) -> Result<Server<BoxAcceptor>> {
        let mut iter = self.listeners.iter();

        let mut acceptor = iter
            .next()
            .ok_or_else(|| anyhow!("At least one listener is required."))?
            .create()
            .await?;

        for listener in iter {
            acceptor = acceptor.combine(listener.create().await?).boxed();
        }
        Ok(Server::new_with_acceptor(acceptor))
    }

    pub async fn create_endpoint(&self) -> Result<Route> {
        let mut consumers = Vec::new();
        let mut services = HashMap::new();
        let mut route = Route::new();
        let mut global_plugins = Vec::new();

        for plugin in &self.global_plugins {
            global_plugins.push(plugin.create().await?);
        }

        for consumer in &self.consumers {
            let mut auth: Option<Arc<dyn AuthPlugin>> = None;
            let mut filters = Vec::new();
            let mut plugins = Vec::new();

            if let Some(auth_plugin) = &consumer.auth {
                auth = Some(auth_plugin.create().await?);
            }

            for filter in &consumer.filters {
                filters.push(filter.create()?);
            }

            for plugin in &consumer.plugins {
                plugins.push(plugin.create().await?);
            }

            consumers.push((consumer.name.clone(), auth, filters, plugins));
        }

        for service in &self.services {
            let name = &service.name;
            let ep = service.target.create()?;
            let mut plugins = Vec::new();

            for plugin in &service.plugins {
                plugins.push(plugin.create().await?);
            }

            services.insert(name, (ep, plugins));
        }

        for RouteConfig {
            path,
            strip,
            plugins,
            service,
        } in &self.routes
        {
            let (service_ep, service_plugins) = services
                .get(&service)
                .ok_or_else(|| anyhow!("Service `{}` is not defined.", service))?;
            let service_ep = service_ep.clone();
            let mut route_plugins = Vec::new();

            for plugin in plugins {
                route_plugins.push(plugin.create().await?);
            }

            let mut handlers = Vec::new();

            if consumers.is_empty() && self.allow_anonymous {
                consumers.push(Default::default());
            }

            for (consumer_name, auth, filters, consumer_plugins) in consumers.clone() {
                let mut plugins = Vec::new();

                plugins.extend(service_plugins.clone());
                plugins.extend(route_plugins.clone());
                plugins.extend(consumer_plugins);
                plugins.extend(global_plugins.clone());
                plugins.sort_by_key(|plugin| Reverse(plugin.priority()));

                handlers.push((consumer_name, auth, filters, plugins));
            }

            let ep = RouteEndpoint {
                handlers,
                endpoint: service_ep.clone(),
            };

            if *strip {
                route = route.nest(path, ep);
            } else {
                route = route.nest_no_strip(path, ep);
            }
        }

        Ok(route)
    }
}

fn check_consumer(filters: &[Arc<dyn ConsumerFilter>], req: &Request) -> bool {
    filters.iter().all(|filter| filter.check(req))
}

struct RouteEndpoint {
    handlers: Vec<(
        String,
        Option<Arc<dyn AuthPlugin>>,
        Vec<Arc<dyn ConsumerFilter>>,
        Vec<Arc<dyn Plugin>>,
    )>,
    endpoint: Arc<dyn Endpoint<Output = Response>>,
}

#[async_trait::async_trait]
impl Endpoint for RouteEndpoint {
    type Output = Response;

    async fn call(&self, req: Request) -> Self::Output {
        for (consumer_name, auth, filter, plugins) in &self.handlers {
            if let Some(auth) = auth {
                if auth.auth(&req).await {
                    if check_consumer(filter, &req) {
                        let mut ctx = PluginContext::new(&req);
                        ctx.insert("consumerName", consumer_name);
                        let next = NextPlugin::new(plugins, &self.endpoint);
                        return next.call(&mut ctx, req).await;
                    }
                }
            }
        }

        StatusCode::UNAUTHORIZED.into_response()
    }
}
