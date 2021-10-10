use anyhow::Result;
use futures_util::stream::BoxStream;

use crate::config::{Config, ConsumerConfig, ListenerConfig, RouteConfig, ServiceConfig};

pub trait ResourcesOperation<T> {
    fn get_all(&self) -> Result<Vec<(String, T)>>;

    fn create(&self, config: T) -> Result<()>;

    fn delete(&self, name: &str) -> Result<()>;

    fn update(&self, name: &str, config: T) -> Result<()>;
}

macro_rules! resource_operation {
    ($name:ident, $ty:ty) => {
        fn $name(&self) -> Result<&dyn ResourcesOperation<$ty>> {
            bail!("not supported")
        }
    };
}

#[allow(unused_variables)]
pub trait ConfigProvider {
    fn watch(&self) -> BoxStream<'_, Config>;

    resource_operation!(listeners, Box<dyn ListenerConfig>);
    resource_operation!(consumers, ConsumerConfig);
    resource_operation!(routes, RouteConfig);
    resource_operation!(services, ServiceConfig);
}
