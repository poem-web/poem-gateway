use async_graphql::{InputObject, Json, ID};

use crate::config::{
    AcceptorConfig, AuthPluginConfig, ConsumerFilterConfig, EndpointConfig, PluginConfig,
};

#[derive(InputObject)]
pub struct TlsConfigInput {
    pub cert: String,
    pub key: String,
}

#[derive(InputObject)]
pub struct ListenerInput {
    pub name: String,
    pub tls: Option<TlsConfigInput>,
    pub acceptor: Json<Box<dyn AcceptorConfig>>,
}

#[derive(InputObject)]
pub struct ConsumerInput {
    pub name: String,
    pub filters: Option<Vec<Json<Box<dyn ConsumerFilterConfig>>>>,
    pub auth: Option<Json<Box<dyn AuthPluginConfig>>>,
    pub plugins: Option<Vec<Json<Box<dyn PluginConfig>>>>,
}

#[derive(InputObject)]
pub struct RouteInput {
    pub path: String,
    #[graphql(default)]
    pub strip: bool,
    pub host: Option<String>,
    pub plugins: Option<Json<Box<dyn PluginConfig>>>,
    pub service: ID,
}

#[derive(InputObject)]
pub struct ServiceInput {
    pub endpoint: Json<Box<dyn EndpointConfig>>,
    pub plugins: Option<Vec<Json<Box<dyn PluginConfig>>>>,
}
