use poem::Result;
use poem_openapi::{payload::Json, types::Any, Object, OpenApi};

use crate::{
    api::{
        global_provider,
        openapi::{
            common_types::{
                CreateResourceResponse, DeleteResourceResponse, GetResourceResponse,
                GetResourcesResponse, UpdateResourceResponse,
            },
            tags::ApiTags,
        },
    },
    config::{AuthPluginConfig, ConsumerConfig, ConsumerFilterConfig, PluginConfig},
};

#[derive(Object)]
struct Consumer {
    #[oai(read_only)]
    id: String,
    pub name: String,
    pub filters: Vec<Any<Box<dyn ConsumerFilterConfig>>>,
    pub auth: Option<Any<Box<dyn AuthPluginConfig>>>,
    pub plugins: Vec<Any<Box<dyn PluginConfig>>>,
}

pub struct ConsumerApi;

#[OpenApi(tag = "ApiTags::Consumer")]
impl ConsumerApi {
    #[oai(path = "/consumer", method = "post")]
    async fn create(&self, consumer: Json<Consumer>) -> Result<CreateResourceResponse> {
        let consumers = global_provider().consumers()?;
        let id = consumers.create(from_api_consumer(consumer.0)).await?;
        Ok(CreateResourceResponse::Ok(Json(id)))
    }

    #[oai(path = "/consumer/:id", method = "put")]
    async fn update(
        &self,

        #[oai(name = "id", in = "path")] id: String,
        consumer: Json<Consumer>,
    ) -> Result<UpdateResourceResponse> {
        let consumers = global_provider().consumers()?;
        consumers.update(&id, from_api_consumer(consumer.0)).await?;
        Ok(UpdateResourceResponse::Ok)
    }

    #[oai(path = "/consumer/:id", method = "get")]
    async fn get(
        &self,

        #[oai(name = "id", in = "path")] id: String,
    ) -> Result<GetResourceResponse<Consumer>> {
        let consumers = global_provider().consumers()?;
        match consumers.get(&id).await? {
            Some(consumer) => Ok(GetResourceResponse::Ok(Json(to_api_consumer(id, consumer)))),
            None => Ok(GetResourceResponse::NotFound),
        }
    }

    #[oai(path = "/consumer", method = "get")]
    async fn get_all(&self) -> Result<GetResourcesResponse<Consumer>> {
        let consumers = global_provider().consumers()?;
        Ok(GetResourcesResponse::Ok(Json(
            consumers
                .get_all()
                .await?
                .into_iter()
                .map(|(id, consumer)| to_api_consumer(id, consumer))
                .collect(),
        )))
    }

    #[oai(path = "/consumer/:id", method = "delete")]
    async fn delete(
        &self,

        #[oai(name = "id", in = "path")] id: String,
    ) -> Result<DeleteResourceResponse> {
        let consumers = global_provider().consumers()?;
        match consumers.delete(&id).await? {
            true => Ok(DeleteResourceResponse::Ok),
            false => Ok(DeleteResourceResponse::NotFound),
        }
    }
}

fn from_api_consumer(consumer: Consumer) -> ConsumerConfig {
    ConsumerConfig {
        name: consumer.name,
        filters: consumer
            .filters
            .into_iter()
            .map(|filter| filter.0)
            .collect(),
        auth: consumer.auth.map(|auth| auth.0),
        plugins: consumer
            .plugins
            .into_iter()
            .map(|plugin| plugin.0)
            .collect(),
    }
}

fn to_api_consumer(id: String, consumer: ConsumerConfig) -> Consumer {
    Consumer {
        id,
        name: consumer.name,
        filters: consumer.filters.into_iter().map(Any).collect(),
        auth: consumer.auth.map(Any),
        plugins: consumer.plugins.into_iter().map(Any).collect(),
    }
}
