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
    config::{self, AcceptorConfig, ListenerConfig},
};

#[derive(Object)]
struct TlsConfig {
    cert: String,
    key: String,
}

#[derive(Object)]
struct Listener {
    #[oai(read_only)]
    id: String,
    tls: Option<TlsConfig>,
    acceptor: Any<Box<dyn AcceptorConfig>>,
}

pub struct ListenerApi;

#[OpenApi(tag = "ApiTags::Listener")]
impl ListenerApi {
    #[oai(path = "/listener", method = "post")]
    async fn create(&self, listener: Json<Listener>) -> Result<CreateResourceResponse> {
        let listeners = global_provider().listeners()?;
        let id = listeners.create(from_api_listener(listener.0)).await?;
        Ok(CreateResourceResponse::Ok(Json(id)))
    }

    #[oai(path = "/listener/:id", method = "put")]
    async fn update(
        &self,
        #[oai(name = "id", in = "path")] id: String,
        listener: Json<Listener>,
    ) -> Result<UpdateResourceResponse> {
        let listeners = global_provider().listeners()?;
        listeners.update(&id, from_api_listener(listener.0)).await?;
        Ok(UpdateResourceResponse::Ok)
    }

    #[oai(path = "/listener/:id", method = "get")]
    async fn get(
        &self,
        #[oai(name = "id", in = "path")] id: String,
    ) -> Result<GetResourceResponse<Listener>> {
        let listeners = global_provider().listeners()?;
        match listeners.get(&id).await? {
            Some(listener) => Ok(GetResourceResponse::Ok(Json(to_api_listener(id, listener)))),
            None => Ok(GetResourceResponse::NotFound),
        }
    }

    #[oai(path = "/listener", method = "get")]
    async fn get_all(&self) -> Result<GetResourcesResponse<Listener>> {
        let listeners = global_provider().listeners()?;
        Ok(GetResourcesResponse::Ok(Json(
            listeners
                .get_all()
                .await?
                .into_iter()
                .map(|(id, listener)| to_api_listener(id, listener))
                .collect(),
        )))
    }

    #[oai(path = "/listener/:id", method = "delete")]
    async fn delete(
        &self,
        #[oai(name = "id", in = "path")] id: String,
    ) -> Result<DeleteResourceResponse> {
        let listeners = global_provider().listeners()?;
        match listeners.delete(&id).await? {
            true => Ok(DeleteResourceResponse::Ok),
            false => Ok(DeleteResourceResponse::NotFound),
        }
    }
}

fn from_api_listener(listener: Listener) -> ListenerConfig {
    ListenerConfig {
        acceptor: listener.acceptor.0,
        tls: listener.tls.map(|tls| config::TlsConfig {
            cert: tls.cert,
            key: tls.key,
        }),
    }
}

fn to_api_listener(id: String, listener: ListenerConfig) -> Listener {
    Listener {
        id,
        tls: listener.tls.map(|tls| TlsConfig {
            cert: tls.cert,
            key: tls.key,
        }),
        acceptor: Any(listener.acceptor),
    }
}
