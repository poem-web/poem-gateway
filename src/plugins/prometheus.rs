use std::{sync::Arc, time::Instant};

use anyhow::Result;
use once_cell::sync::Lazy;
use opentelemetry::{
    global,
    metrics::{Counter, Unit, ValueRecorder},
    Key,
};
use poem::{Request, Response};
use serde::{Deserialize, Serialize};

use crate::{
    config::PluginConfig,
    plugins::{NextPlugin, Plugin, PluginContext},
};

const METHOD_KEY: Key = Key::from_static_str("request_method");
const PATH_KEY: Key = Key::from_static_str("request_path");
const STATUS_KEY: Key = Key::from_static_str("response_status_code");

static REQUESTS_COUNT: Lazy<Counter<u64>> = Lazy::new(|| {
    let meter = global::meter("poem-gateway");
    meter
        .u64_counter("requests_count")
        .with_description("total request count (since start of service)")
        .init()
});

static ERRORS_COUNT: Lazy<Counter<u64>> = Lazy::new(|| {
    let meter = global::meter("poem-gateway");
    meter
        .u64_counter("errors_count")
        .with_description("failed request count (since start of service)")
        .init()
});

static REQUEST_DURATION_MS: Lazy<ValueRecorder<f64>> = Lazy::new(|| {
    let meter = global::meter("poem-gateway");
    meter
        .f64_value_recorder("request_duration_ms")
        .with_unit(Unit::new("milliseconds"))
        .with_description("request duration histogram (in milliseconds, since start of service)")
        .init()
});

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Config {}

#[typetag::serde(name = "prometheus")]
#[async_trait::async_trait]
impl PluginConfig for Config {
    async fn create(&self) -> Result<Arc<dyn Plugin>> {
        Ok(Arc::new(Prometheus))
    }
}

struct Prometheus;

#[async_trait::async_trait]
impl Plugin for Prometheus {
    fn priority(&self) -> i32 {
        0
    }

    async fn call(&self, req: Request, ctx: &mut PluginContext, next: NextPlugin<'_>) -> Response {
        let mut labels = Vec::with_capacity(3);
        labels.push(METHOD_KEY.string(req.method().to_string()));
        labels.push(PATH_KEY.string(req.uri().path().to_string()));

        let s = Instant::now();
        let resp = next.call(ctx, req).await;
        let elapsed = s.elapsed();

        labels.push(STATUS_KEY.i64(resp.status().as_u16() as i64));

        if resp.status().is_server_error() {
            ERRORS_COUNT.add(1, &labels)
        }
        REQUESTS_COUNT.add(1, &labels);
        REQUEST_DURATION_MS.record(elapsed.as_secs_f64() / 1000.0, &labels);

        resp
    }
}
