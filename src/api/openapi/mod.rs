mod common_types;
mod consumer;
mod global_plugins;
mod listener;
mod route;
mod service;
mod tags;

use std::sync::Arc;

use anyhow::Result;
use poem::{Endpoint, Response, Route};
use poem_openapi::{OpenApi, OpenApiService};
use serde::{Deserialize, Serialize};

use crate::config::EndpointConfig;

#[derive(Serialize, Deserialize, Clone)]
struct Config {}

#[typetag::serde(name = "openapiAdmin")]
impl EndpointConfig for Config {
    fn create(&self) -> Result<Arc<dyn Endpoint<Output = Response>>> {
        let api = OpenApiService::new(
            listener::ListenerApi
                .combine(consumer::ConsumerApi)
                .combine(route::RouteApi)
                .combine(service::ServiceApi)
                .combine(global_plugins::GlobalPluginApi),
        )
        .title("Poem Gateway API")
        .version(env!("CARGO_PKG_VERSION"));

        Ok(Arc::new(
            Route::new().nest("/ui", api.swagger_ui()).nest("/", api),
        ))
    }
}
