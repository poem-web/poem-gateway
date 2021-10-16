use async_graphql::{FieldResult, Json, Object, ID};

use crate::{
    api::{
        global_provider,
        graphql::input::{ConsumerInput, ListenerInput, RouteInput, ServiceInput},
    },
    config::{
        ConsumerConfig, ListenerConfig, PluginConfig, RouteConfig, ServiceConfig, ServiceRef,
        TlsConfig,
    },
};

pub struct Mutation;

#[Object]
impl Mutation {
    async fn create_listener(&self, config: ListenerInput) -> FieldResult<ID> {
        Ok(global_provider()
            .listeners()?
            .create(from_gql_listener(config))
            .await?
            .into())
    }

    async fn update_listener(&self, id: ID, config: ListenerInput) -> FieldResult<bool> {
        global_provider()
            .listeners()?
            .update(&id, from_gql_listener(config))
            .await?;
        Ok(true)
    }

    async fn delete_listener(&self, id: String) -> FieldResult<bool> {
        global_provider().listeners()?.delete(&id).await?;
        Ok(true)
    }

    async fn create_consumer(&self, consumer: ConsumerInput) -> FieldResult<ID> {
        let consumer_config = from_gql_consumer(consumer);
        Ok(global_provider()
            .consumers()?
            .create(consumer_config)
            .await?
            .into())
    }

    async fn update_consumer(&self, id: ID, consumer: ConsumerInput) -> FieldResult<bool> {
        let consumer_config = from_gql_consumer(consumer);
        global_provider()
            .consumers()?
            .update(&id, consumer_config)
            .await?;
        Ok(true)
    }

    async fn delete_consumer(&self, id: String) -> FieldResult<bool> {
        global_provider().consumers()?.delete(&id).await?;
        Ok(true)
    }

    async fn create_route(&self, route: RouteInput) -> FieldResult<ID> {
        let route_config = from_gql_route(route);
        Ok(global_provider()
            .routes()?
            .create(route_config)
            .await?
            .into())
    }

    async fn update_route(&self, id: ID, route: RouteInput) -> FieldResult<bool> {
        let route_config = from_gql_route(route);
        global_provider()
            .routes()?
            .update(&id, route_config)
            .await?;
        Ok(true)
    }

    async fn delete_route(&self, id: String) -> FieldResult<bool> {
        global_provider().routes()?.delete(&id).await?;
        Ok(true)
    }

    async fn create_service(&self, service: ServiceInput) -> FieldResult<ID> {
        let service_config = from_gql_service(service);
        Ok(global_provider()
            .services()?
            .create(service_config)
            .await?
            .into())
    }

    async fn update_service(&self, id: ID, service: ServiceInput) -> FieldResult<bool> {
        let service_config = from_gql_service(service);
        global_provider()
            .services()?
            .update(&id, service_config)
            .await?;
        Ok(true)
    }

    async fn delete_services(&self, id: String) -> FieldResult<bool> {
        global_provider().services()?.delete(&id).await?;
        Ok(true)
    }

    async fn create_global_plugin(&self, config: Json<Box<dyn PluginConfig>>) -> FieldResult<ID> {
        Ok(global_provider()
            .global_plugins()?
            .create(config.0)
            .await?
            .into())
    }

    async fn update_global_plugin(
        &self,
        id: ID,
        config: Json<Box<dyn PluginConfig>>,
    ) -> FieldResult<bool> {
        global_provider()
            .global_plugins()?
            .update(&id, config.0)
            .await?;
        Ok(true)
    }

    async fn delete_global_plugin(&self, id: String) -> FieldResult<bool> {
        global_provider().global_plugins()?.delete(&id).await?;
        Ok(true)
    }
}

fn from_gql_listener(listener: ListenerInput) -> ListenerConfig {
    ListenerConfig {
        acceptor: listener.acceptor.0,
        tls: listener.tls.map(|tls| TlsConfig {
            cert: tls.cert,
            key: tls.key,
        }),
    }
}

fn from_gql_consumer(consumer: ConsumerInput) -> ConsumerConfig {
    ConsumerConfig {
        name: consumer.name,
        filters: consumer
            .filters
            .unwrap_or_default()
            .into_iter()
            .map(|value| value.0)
            .collect(),
        auth: consumer.auth.map(|value| value.0),
        plugins: consumer
            .plugins
            .unwrap_or_default()
            .into_iter()
            .map(|value| value.0)
            .collect(),
    }
}

fn from_gql_route(route: RouteInput) -> RouteConfig {
    RouteConfig {
        path: route.path,
        strip: route.strip,
        host: route.host,
        plugins: route.plugins.into_iter().map(|value| value.0).collect(),
        service_ref: ServiceRef::Reference(route.service.0),
    }
}

fn from_gql_service(service: ServiceInput) -> ServiceConfig {
    ServiceConfig {
        name: None,
        endpoint: service.endpoint.0,
        plugins: service
            .plugins
            .unwrap_or_default()
            .into_iter()
            .map(|value| value.0)
            .collect(),
    }
}
