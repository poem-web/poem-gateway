mod consumer;
mod debounced_stream;
mod listener;
mod plugin;
mod provider;
mod route;
mod service;

use std::{cmp::Reverse, collections::HashMap, future::Future, sync::Arc};

use anyhow::Result;
use poem::{
    http::StatusCode,
    listener::{Acceptor, AcceptorExt},
    web::{LocalAddr, RemoteAddr},
    Endpoint, IntoResponse, Request, Response, Route, RouteDomain, Server,
};
use serde::Deserialize;

pub use crate::config::{
    consumer::{ConsumerConfig, ConsumerFilterConfig},
    debounced_stream::DebouncedStream,
    listener::{AcceptorConfig, ListenerConfig, TlsConfig},
    plugin::{AuthPluginConfig, PluginConfig},
    provider::{ConfigProvider, ConfigProviderConfig, ResourcesOperation, ServiceNotFoundError},
    route::{RouteConfig, ServiceRef},
    service::{EndpointConfig, ServiceConfig},
};
use crate::{
    consumer_filters::ConsumerFilter,
    plugins::{AuthPlugin, NextPlugin, Plugin, PluginContext},
};

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GatewayConfig {
    pub provider: Box<dyn ConfigProviderConfig>,
    pub admin: ProxyConfig,
}

#[derive(Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProxyConfig {
    #[serde(default)]
    pub listeners: Vec<ListenerConfig>,
    #[serde(default)]
    pub consumers: Vec<ConsumerConfig>,
    #[serde(default)]
    pub routes: Vec<RouteConfig>,
    #[serde(default)]
    pub services: Vec<ServiceConfig>,
    #[serde(default)]
    pub global_plugins: Vec<Box<dyn PluginConfig>>,
}

impl ProxyConfig {
    pub async fn start_server(&self) -> Result<impl Future<Output = ()> + Send + 'static> {
        use std::io::Result as IoResult;

        let ep = self.create_endpoint().await?;

        struct NopAcceptor;

        #[async_trait::async_trait]
        impl Acceptor for NopAcceptor {
            type Io = tokio::net::TcpStream;

            fn local_addr(&self) -> Vec<LocalAddr> {
                vec![]
            }

            async fn accept(&mut self) -> IoResult<(Self::Io, LocalAddr, RemoteAddr)> {
                std::future::pending().await
            }
        }

        let mut iter = self.listeners.iter();

        let acceptor = match iter.next() {
            Some(listener) => {
                let mut acceptor = listener.create_acceptor().await?;
                for listener in iter {
                    acceptor = acceptor.combine(listener.create_acceptor().await?).boxed();
                }
                acceptor
            }
            None => NopAcceptor.boxed(),
        };
        let server = Server::new_with_acceptor(acceptor);

        Ok(async move {
            let _ = server.run(ep).await;
        })
    }

    async fn create_endpoint(&self) -> Result<impl Endpoint> {
        let mut consumers = Vec::new();
        let mut services = HashMap::new();
        let mut route_items: HashMap<_, Vec<_>> = HashMap::new();
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
            let name = service
                .name
                .as_ref()
                .ok_or_else(|| anyhow!("Missing service name."))?;
            let ep = service.endpoint.create()?;
            let mut plugins = Vec::new();

            for plugin in &service.plugins {
                plugins.push(plugin.create().await?);
            }

            services.insert(name, (ep, plugins));
        }

        for RouteConfig {
            path,
            strip,
            host,
            plugins,
            service_ref,
        } in &self.routes
        {
            let (service_ep, service_plugins) = match service_ref {
                ServiceRef::Reference(service_name) => services
                    .get(&service_name)
                    .map(|(ep, plugins)| (ep.clone(), plugins.clone()))
                    .ok_or_else(|| anyhow!("Service `{}` is not defined.", service_name))?,
                ServiceRef::Inline(service) => {
                    let ep = service.endpoint.create()?;
                    let mut plugins = Vec::new();
                    for plugin in &service.plugins {
                        plugins.push(plugin.create().await?);
                    }
                    (ep, plugins)
                }
            };

            let mut route_plugins = Vec::new();

            for plugin in plugins {
                route_plugins.push(plugin.create().await?);
            }

            let mut handlers = Vec::new();

            // with consumer
            for (consumer_name, auth, filters, consumer_plugins) in consumers.clone() {
                let mut plugins = Vec::new();

                plugins.extend(service_plugins.clone());
                plugins.extend(route_plugins.clone());
                plugins.extend(consumer_plugins);
                plugins.extend(global_plugins.clone());
                plugins.sort_by_key(|plugin| Reverse(plugin.priority()));

                handlers.push(Handler::WithConsumer {
                    consumer_name,
                    auth,
                    filters,
                    plugins,
                });
            }

            // without consumer
            let mut plugins = Vec::new();
            plugins.extend(service_plugins.clone());
            plugins.extend(route_plugins.clone());
            plugins.extend(global_plugins.clone());
            plugins.sort_by_key(|plugin| Reverse(plugin.priority()));
            handlers.push(Handler::WithoutConsumer { plugins });

            let ep = RouteEndpoint {
                handlers,
                endpoint: service_ep.clone(),
            };

            route_items
                .entry(host)
                .or_default()
                .push((*strip, path, ep));
        }

        let route = route_items
            .into_iter()
            .fold(RouteDomain::new(), |route, (host, items)| {
                route.add(
                    match host {
                        Some(path) if !path.is_empty() => path,
                        _ => "*",
                    },
                    items
                        .into_iter()
                        .fold(Route::new(), |route, (strip, path, ep)| {
                            if strip {
                                route.nest(path, ep)
                            } else {
                                route.nest_no_strip(path, ep)
                            }
                        }),
                )
            });

        Ok(route)
    }
}

fn check_consumer(filters: &[Arc<dyn ConsumerFilter>], req: &Request) -> bool {
    filters.iter().all(|filter| filter.check(req))
}

enum Handler {
    WithConsumer {
        consumer_name: String,
        auth: Option<Arc<dyn AuthPlugin>>,
        filters: Vec<Arc<dyn ConsumerFilter>>,
        plugins: Vec<Arc<dyn Plugin>>,
    },
    WithoutConsumer {
        plugins: Vec<Arc<dyn Plugin>>,
    },
}

struct RouteEndpoint {
    handlers: Vec<Handler>,
    endpoint: Arc<dyn Endpoint<Output = Response>>,
}

#[async_trait::async_trait]
impl Endpoint for RouteEndpoint {
    type Output = Response;

    async fn call(&self, req: Request) -> Self::Output {
        for handler in &self.handlers {
            match handler {
                Handler::WithConsumer {
                    consumer_name,
                    auth,
                    filters,
                    plugins,
                } => {
                    if let Some(auth) = auth {
                        if !auth.auth(&req).await {
                            continue;
                        }
                    }

                    if check_consumer(filters, &req) {
                        let mut ctx = PluginContext::new(&req);
                        ctx.insert("consumerName", consumer_name);
                        let next = NextPlugin::new(plugins, &self.endpoint);
                        return next.call(&mut ctx, req).await;
                    }
                }
                Handler::WithoutConsumer { plugins } => {
                    let mut ctx = PluginContext::new(&req);
                    let next = NextPlugin::new(plugins, &self.endpoint);
                    return next.call(&mut ctx, req).await;
                }
            }
        }

        StatusCode::UNAUTHORIZED.into_response()
    }
}
