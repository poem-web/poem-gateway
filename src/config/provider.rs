use std::{path::Path, sync::Arc};

use anyhow::Result;
use dyn_clone::DynClone;
use futures_util::stream::BoxStream;

use crate::config::{
    ConsumerConfig, ListenerConfig, PluginConfig, ProxyConfig, RouteConfig, ServiceConfig,
};

#[derive(Debug, thiserror::Error)]
#[error("service `{0}` not found")]
pub struct ServiceNotFoundError(pub String);

#[async_trait::async_trait]
pub trait ResourcesOperation<T>: Send + Sync {
    async fn get_all(&self) -> Result<Vec<(String, T)>>;

    async fn get(&self, id: &str) -> Result<Option<T>>;

    async fn create(&self, config: T) -> Result<String>;

    async fn delete(&self, id: &str) -> Result<bool>;

    async fn update(&self, id: &str, config: T) -> Result<()>;
}

macro_rules! resource_operation {
    ($name:ident, $ty:ty) => {
        fn $name(&self) -> Result<Box<dyn ResourcesOperation<$ty>>> {
            bail!("not supported")
        }
    };
}

#[allow(unused_variables)]
pub trait ConfigProvider: Send + Sync + 'static {
    fn watch(&self) -> BoxStream<'_, Result<ProxyConfig>>;

    resource_operation!(listeners, ListenerConfig);
    resource_operation!(consumers, ConsumerConfig);
    resource_operation!(routes, RouteConfig);
    resource_operation!(services, ServiceConfig);
    resource_operation!(global_plugins, Box<dyn PluginConfig>);
}

#[typetag::serde(tag = "type")]
#[async_trait::async_trait]
pub trait ConfigProviderConfig: DynClone + Send + Sync + 'static {
    async fn create(&self, config_root: &Path) -> Result<Arc<dyn ConfigProvider>>;
}

dyn_clone::clone_trait_object!(ConfigProviderConfig);
