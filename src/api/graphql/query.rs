use async_graphql::{ComplexObject, FieldResult, Json, Object, SimpleObject, ID};

use crate::{
    api::global_provider,
    config::{
        AcceptorConfig, AuthPluginConfig, ConsumerConfig, ConsumerFilterConfig, EndpointConfig,
        ListenerConfig, PluginConfig, RouteConfig, ServiceConfig,
    },
};

#[derive(SimpleObject)]
struct Tls {
    cert: String,
    key: String,
}

#[derive(SimpleObject)]
struct Listener {
    id: ID,
    tls: Option<Tls>,
    acceptor: Json<Box<dyn AcceptorConfig>>,
}

#[derive(SimpleObject)]
struct Consumer {
    id: ID,
    name: String,
    filters: Vec<Json<Box<dyn ConsumerFilterConfig>>>,
    auth: Option<Json<Box<dyn AuthPluginConfig>>>,
    plugins: Vec<Json<Box<dyn PluginConfig>>>,
}

enum ServiceRef {
    Reference(ID),
    Inline(Service),
}

#[derive(SimpleObject)]
#[graphql(complex)]
struct Route {
    id: ID,
    path: String,
    strip: bool,
    host: Option<String>,
    plugins: Vec<Json<Box<dyn PluginConfig>>>,
    #[graphql(skip)]
    service_ref: ServiceRef,
}

#[ComplexObject]
impl Route {
    async fn service(&self) -> FieldResult<Option<Service>> {
        match &self.service_ref {
            ServiceRef::Reference(id) => {
                Ok(global_provider()
                    .services()?
                    .get(id)
                    .await?
                    .map(|service| Service {
                        id: Some(id.clone()),
                        endpoint: Json(service.endpoint),
                        plugins: service.plugins.into_iter().map(Json).collect(),
                    }))
            }
            ServiceRef::Inline(service) => Ok(Some(service.clone())),
        }
    }
}

#[derive(SimpleObject, Clone)]
struct Service {
    id: Option<ID>,
    endpoint: Json<Box<dyn EndpointConfig>>,
    plugins: Vec<Json<Box<dyn PluginConfig>>>,
}

#[derive(SimpleObject)]
struct GlobalPlugin {
    id: ID,
    config: Json<Box<dyn PluginConfig>>,
}

pub struct Query;

#[Object]
impl Query {
    async fn version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }

    async fn listeners(&self) -> FieldResult<Vec<Listener>> {
        Ok(global_provider()
            .listeners()?
            .get_all()
            .await?
            .into_iter()
            .map(|(id, config)| to_gql_listener(id, config))
            .collect())
    }

    async fn listener(&self, id: String) -> FieldResult<Option<Listener>> {
        let listener = global_provider().listeners()?.get(&id).await?;
        Ok(listener.map(|listener| to_gql_listener(id, listener)))
    }

    async fn consumers(&self) -> FieldResult<Vec<Consumer>> {
        Ok(global_provider()
            .consumers()?
            .get_all()
            .await?
            .into_iter()
            .map(|(id, consumer)| to_gql_consumer(id, consumer))
            .collect())
    }

    async fn consumer(&self, id: String) -> FieldResult<Option<Consumer>> {
        let consumer = global_provider().consumers()?.get(&id).await?;
        Ok(consumer.map(|consumer| to_gql_consumer(id, consumer)))
    }

    async fn routes(&self) -> FieldResult<Vec<Route>> {
        Ok(global_provider()
            .routes()?
            .get_all()
            .await?
            .into_iter()
            .map(|(id, route)| to_gql_route(id, route))
            .collect())
    }

    async fn route(&self, id: String) -> FieldResult<Option<Route>> {
        let route = global_provider().routes()?.get(&id).await?;
        Ok(route.map(|route| to_gql_route(id, route)))
    }

    async fn services(&self) -> FieldResult<Vec<Service>> {
        Ok(global_provider()
            .services()?
            .get_all()
            .await?
            .into_iter()
            .map(|(id, service)| to_gql_service(id, service))
            .collect())
    }

    async fn service(&self, id: String) -> FieldResult<Option<Service>> {
        let service = global_provider().services()?.get(&id).await?;
        Ok(service.map(|service| to_gql_service(id, service)))
    }

    async fn global_plugins(&self) -> FieldResult<Vec<GlobalPlugin>> {
        Ok(global_provider()
            .global_plugins()?
            .get_all()
            .await?
            .into_iter()
            .map(|(id, config)| to_gql_global_plugin(id, config))
            .collect())
    }

    async fn global_plugin(&self, id: String) -> FieldResult<Option<GlobalPlugin>> {
        let global_plugin = global_provider().global_plugins()?.get(&id).await?;
        Ok(global_plugin.map(|global_plugin| to_gql_global_plugin(id, global_plugin)))
    }
}

fn to_gql_listener(id: String, config: ListenerConfig) -> Listener {
    Listener {
        id: id.into(),
        tls: config.tls.map(|tls| Tls {
            cert: tls.cert,
            key: tls.key,
        }),
        acceptor: Json(config.acceptor),
    }
}

fn to_gql_consumer(id: String, consumer: ConsumerConfig) -> Consumer {
    Consumer {
        id: id.into(),
        name: consumer.name,
        filters: consumer.filters.into_iter().map(Json).collect(),
        auth: consumer.auth.map(Json),
        plugins: consumer.plugins.into_iter().map(Json).collect(),
    }
}

fn to_gql_route(id: String, route: RouteConfig) -> Route {
    Route {
        id: id.into(),
        path: route.path,
        strip: route.strip,
        host: route.host,
        plugins: route.plugins.into_iter().map(Json).collect(),
        service_ref: match route.service_ref {
            crate::config::ServiceRef::Reference(id) => ServiceRef::Reference(id.into()),
            crate::config::ServiceRef::Inline(service_cfg) => ServiceRef::Inline(Service {
                id: None,
                endpoint: Json(service_cfg.endpoint),
                plugins: service_cfg.plugins.into_iter().map(Json).collect(),
            }),
        },
    }
}

fn to_gql_service(id: String, service: ServiceConfig) -> Service {
    Service {
        id: Some(id.into()),
        endpoint: Json(service.endpoint),
        plugins: service.plugins.into_iter().map(Json).collect(),
    }
}

fn to_gql_global_plugin(id: String, config: Box<dyn PluginConfig>) -> GlobalPlugin {
    GlobalPlugin {
        id: id.into(),
        config: Json(config),
    }
}
