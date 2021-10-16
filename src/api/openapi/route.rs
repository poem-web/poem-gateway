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
    config::{PluginConfig, RouteConfig, ServiceRef},
};

#[derive(Object)]
struct Route {
    #[oai(read_only)]
    id: String,
    pub path: String,
    #[oai(default)]
    pub strip: bool,
    pub host: Option<String>,
    pub plugins: Vec<Any<Box<dyn PluginConfig>>>,
    pub service: String,
}

pub struct RouteApi;

#[OpenApi(tag = "ApiTags::Route")]
impl RouteApi {
    #[oai(path = "/route", method = "post")]
    async fn create(&self, route: Json<Route>) -> Result<CreateResourceResponse> {
        let routes = global_provider().routes()?;
        let id = routes.create(from_api_route(route.0)).await?;
        Ok(CreateResourceResponse::Ok(Json(id)))
    }

    #[oai(path = "/route/:id", method = "put")]
    async fn update(
        &self,
        #[oai(name = "id", in = "path")] id: String,
        route: Json<Route>,
    ) -> Result<UpdateResourceResponse> {
        let routes = global_provider().routes()?;
        routes.update(&id, from_api_route(route.0)).await?;
        Ok(UpdateResourceResponse::Ok)
    }

    #[oai(path = "/route/:id", method = "get")]
    async fn get(
        &self,
        #[oai(name = "id", in = "path")] id: String,
    ) -> Result<GetResourceResponse<Route>> {
        let routes = global_provider().routes()?;
        match routes.get(&id).await? {
            Some(route) => Ok(GetResourceResponse::Ok(Json(to_api_route(id, route)))),
            None => Ok(GetResourceResponse::NotFound),
        }
    }

    #[oai(path = "/route", method = "get")]
    async fn get_all(&self) -> Result<GetResourcesResponse<Route>> {
        let routes = global_provider().routes()?;
        Ok(GetResourcesResponse::Ok(Json(
            routes
                .get_all()
                .await?
                .into_iter()
                .map(|(id, route)| to_api_route(id, route))
                .collect(),
        )))
    }

    #[oai(path = "/route/:id", method = "delete")]
    async fn delete(
        &self,
        #[oai(name = "id", in = "path")] id: String,
    ) -> Result<DeleteResourceResponse> {
        let routes = global_provider().routes()?;
        match routes.delete(&id).await? {
            true => Ok(DeleteResourceResponse::Ok),
            false => Ok(DeleteResourceResponse::NotFound),
        }
    }
}

fn from_api_route(route: Route) -> RouteConfig {
    RouteConfig {
        path: route.path,
        strip: route.strip,
        host: route.host,
        plugins: route.plugins.into_iter().map(|plugin| plugin.0).collect(),
        service_ref: ServiceRef::Reference(route.service),
    }
}

fn to_api_route(id: String, route: RouteConfig) -> Route {
    Route {
        id,
        path: route.path,
        strip: route.strip,
        host: route.host,
        plugins: route.plugins.into_iter().map(Any).collect(),
        service: match route.service_ref {
            ServiceRef::Reference(id) => id,
            ServiceRef::Inline(_) => unreachable!(),
        },
    }
}
