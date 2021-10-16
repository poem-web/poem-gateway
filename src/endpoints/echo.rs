use std::sync::Arc;

use anyhow::Result;
use poem::{Endpoint, IntoResponse, Response};
use serde::{Deserialize, Serialize};

use crate::config::EndpointConfig;

#[derive(Serialize, Deserialize, Clone)]
struct Config {}

#[typetag::serde(name = "echo")]
impl EndpointConfig for Config {
    fn create(&self) -> Result<Arc<dyn Endpoint<Output = Response>>> {
        Ok(Arc::new(poem::endpoint::make_sync(|req| {
            req.into_body().into_response()
        })))
    }
}
