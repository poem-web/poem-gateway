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
    config::PluginConfig,
};

#[derive(Object)]
struct GlobalPlugin {
    #[oai(read_only)]
    id: String,
    config: Any<Box<dyn PluginConfig>>,
}

pub struct GlobalPluginApi;

#[OpenApi(tag = "ApiTags::GlobalPlugin")]
impl GlobalPluginApi {
    #[oai(path = "/globalPlugins", method = "post")]
    async fn create(&self, plugin: Json<GlobalPlugin>) -> Result<CreateResourceResponse> {
        let plugins = global_provider().global_plugins()?;
        let id = plugins.create(plugin.0.config.0).await?;
        Ok(CreateResourceResponse::Ok(Json(id)))
    }

    #[oai(path = "/globalPlugins/:id", method = "put")]
    async fn update(
        &self,
        #[oai(name = "id", in = "path")] id: String,
        plugin: Json<GlobalPlugin>,
    ) -> Result<UpdateResourceResponse> {
        let plugins = global_provider().global_plugins()?;
        plugins.update(&id, plugin.0.config.0).await?;
        Ok(UpdateResourceResponse::Ok)
    }

    #[oai(path = "/globalPlugins/:id", method = "get")]
    async fn get(
        &self,
        #[oai(name = "id", in = "path")] id: String,
    ) -> Result<GetResourceResponse<GlobalPlugin>> {
        let plugins = global_provider().global_plugins()?;
        match plugins.get(&id).await? {
            Some(plugin) => Ok(GetResourceResponse::Ok(Json(GlobalPlugin {
                id,
                config: Any(plugin),
            }))),
            None => Ok(GetResourceResponse::NotFound),
        }
    }

    #[oai(path = "/globalPlugins", method = "get")]
    async fn get_all(&self) -> Result<GetResourcesResponse<GlobalPlugin>> {
        let plugins = global_provider().global_plugins()?;
        Ok(GetResourcesResponse::Ok(Json(
            plugins
                .get_all()
                .await?
                .into_iter()
                .map(|(id, plugin)| GlobalPlugin {
                    id,
                    config: Any(plugin),
                })
                .collect(),
        )))
    }

    #[oai(path = "/globalPlugins/:id", method = "delete")]
    async fn delete(
        &self,
        #[oai(name = "id", in = "path")] id: String,
    ) -> Result<DeleteResourceResponse> {
        let plugins = global_provider().global_plugins()?;
        match plugins.delete(&id).await? {
            true => Ok(DeleteResourceResponse::Ok),
            false => Ok(DeleteResourceResponse::NotFound),
        }
    }
}
