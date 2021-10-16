use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use opentelemetry::KeyValue;
use poem::{endpoint::PrometheusExporter, Endpoint, IntoEndpoint, Response};
use serde::{Deserialize, Serialize};

use crate::config::EndpointConfig;

#[derive(Serialize, Deserialize, Clone)]
struct Config {
    #[serde(default)]
    labels: HashMap<String, String>,
}

#[typetag::serde(name = "prometheusExporter")]
impl EndpointConfig for Config {
    fn create(&self) -> Result<Arc<dyn Endpoint<Output = Response>>> {
        let mut ep = PrometheusExporter::new();
        for (name, value) in &self.labels {
            ep = ep.label(KeyValue::new(name.clone(), value.clone()));
        }
        Ok(Arc::new(ep.into_endpoint()))
    }
}
