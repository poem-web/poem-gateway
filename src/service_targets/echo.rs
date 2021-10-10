use std::sync::Arc;

use anyhow::Result;
use poem::{Endpoint, Response};
use serde::{Deserialize, Serialize};

use crate::config::ServiceTargetConfig;

#[derive(Serialize, Deserialize)]
struct EchoConfig {}

#[typetag::serde(name = "echo")]
impl ServiceTargetConfig for EchoConfig {
    fn create(&self) -> Result<Arc<dyn Endpoint<Output = Response>>> {
        // TODO: waitting for poem 1.0.2
        Ok(Arc::new(poem::endpoint::make_sync(|req| {
            Response::builder().body(req.into_body())
        })))
    }
}
