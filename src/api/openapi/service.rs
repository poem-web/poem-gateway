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
    config::{EndpointConfig, PluginConfig, ServiceConfig},
};

#[derive(Object)]
struct Service {
    #[oai(read_only)]
    id: String,
    endpoint: Any<Box<dyn EndpointConfig>>,
    plugins: Vec<Any<Box<dyn PluginConfig>>>,
}

pub struct ServiceApi;

#[OpenApi(tag = "ApiTags::Service")]
impl ServiceApi {
    #[oai(path = "/service", method = "post")]
    async fn create(&self, service: Json<Service>) -> Result<CreateResourceResponse> {
        let services = global_provider().services()?;
        let id = services.create(from_api_service(service.0)).await?;
        Ok(CreateResourceResponse::Ok(Json(id)))
    }

    #[oai(path = "/service/:id", method = "put")]
    async fn update(
        &self,
        #[oai(name = "id", in = "path")] id: String,
        service: Json<Service>,
    ) -> Result<UpdateResourceResponse> {
        let services = global_provider().services()?;
        services.update(&id, from_api_service(service.0)).await?;
        Ok(UpdateResourceResponse::Ok)
    }

    #[oai(path = "/service/:id", method = "get")]
    async fn get(
        &self,
        #[oai(name = "id", in = "path")] id: String,
    ) -> Result<GetResourceResponse<Service>> {
        let services = global_provider().services()?;
        match services.get(&id).await? {
            Some(service) => Ok(GetResourceResponse::Ok(Json(to_api_service(id, service)))),
            None => Ok(GetResourceResponse::NotFound),
        }
    }

    #[oai(path = "/service", method = "get")]
    async fn get_all(&self) -> Result<GetResourcesResponse<Service>> {
        let services = global_provider().services()?;
        Ok(GetResourcesResponse::Ok(Json(
            services
                .get_all()
                .await?
                .into_iter()
                .map(|(id, service)| to_api_service(id, service))
                .collect(),
        )))
    }

    #[oai(path = "/service/:id", method = "delete")]
    async fn delete(
        &self,
        #[oai(name = "id", in = "path")] id: String,
    ) -> Result<DeleteResourceResponse> {
        let services = global_provider().services()?;
        match services.delete(&id).await? {
            true => Ok(DeleteResourceResponse::Ok),
            false => Ok(DeleteResourceResponse::NotFound),
        }
    }
}

fn from_api_service(service: Service) -> ServiceConfig {
    ServiceConfig {
        name: None,
        endpoint: service.endpoint.0,
        plugins: service.plugins.into_iter().map(|plugin| plugin.0).collect(),
    }
}

fn to_api_service(id: String, service: ServiceConfig) -> Service {
    Service {
        id,
        endpoint: Any(service.endpoint),
        plugins: service.plugins.into_iter().map(Any).collect(),
    }
}
