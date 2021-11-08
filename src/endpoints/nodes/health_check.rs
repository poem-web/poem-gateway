use std::sync::{Arc, Weak};

use anyhow::Result;
use hyper::{Body, Client};
use hyper_rustls::HttpsConnector;
use parking_lot::Mutex;
use poem::http::{uri::Authority, Method, Uri};
use serde::{Deserialize, Serialize};
use tokio::time::Duration;

use crate::endpoints::{nodes::Nodes, upstream::UpstreamConfig};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HealthConfig {
    path: String,
    #[serde(default = "default_interval")]
    interval: Duration,
    #[serde(default = "default_status")]
    status: Vec<u16>,
}

fn default_interval() -> Duration {
    Duration::from_secs(30)
}

fn default_status() -> Vec<u16> {
    vec![200]
}

pub struct HealthCheck {
    current_alive_nodes: Arc<Mutex<Vec<Authority>>>,
}

impl HealthCheck {
    pub fn new(
        upstream_config: Arc<UpstreamConfig>,
        nodes: Vec<Authority>,
        cfg: HealthConfig,
    ) -> Self {
        let current_alive_nodes = Arc::new(Mutex::new(Default::default()));
        let cfg = Arc::new(cfg);
        tokio::spawn(checker(
            upstream_config,
            nodes,
            cfg,
            Arc::downgrade(&current_alive_nodes),
        ));
        Self {
            current_alive_nodes,
        }
    }
}

impl Nodes for HealthCheck {
    fn get(&self, callback: &dyn Fn(&[Authority]) -> Option<Authority>) -> Option<Authority> {
        let current_alive_nodes = self.current_alive_nodes.lock();
        callback(&current_alive_nodes)
    }
}

async fn checker(
    upstream_config: Arc<UpstreamConfig>,
    nodes: Vec<Authority>,
    cfg: Arc<HealthConfig>,
    current_alive_nodes: Weak<Mutex<Vec<Authority>>>,
) {
    loop {
        let alive_nodes = do_check(upstream_config.clone(), nodes.clone(), cfg.clone()).await;
        if let Some(current_alive_nodes) = current_alive_nodes.upgrade() {
            *current_alive_nodes.lock() = alive_nodes;
        }
        tokio::time::sleep(cfg.interval).await;
    }
}

async fn do_check(
    upstream_config: Arc<UpstreamConfig>,
    nodes: Vec<Authority>,
    cfg: Arc<HealthConfig>,
) -> Vec<Authority> {
    let tasks: Vec<_> = nodes
        .into_iter()
        .map({
            let cfg = cfg.clone();
            move |authority| {
                let cfg = cfg.clone();
                let upstream_config = upstream_config.clone();
                tokio::spawn(async move {
                    let uri = match create_uri(&upstream_config, &authority, &cfg) {
                        Ok(uri) => uri,
                        Err(_) => return None,
                    };

                    let https = HttpsConnector::with_webpki_roots();
                    let client = Client::builder().build(https);

                    let res = client
                        .request(
                            hyper::Request::builder()
                                .method(Method::GET)
                                .uri(uri)
                                .body(Body::empty())
                                .unwrap(),
                        )
                        .await;

                    match res {
                        Ok(resp) if cfg.status.contains(&resp.status().as_u16()) => Some(authority),
                        _ => None,
                    }
                })
            }
        })
        .collect();

    let mut success = Vec::new();
    for task in tasks {
        if let Some(authority) = task.await.unwrap() {
            success.push(authority);
        }
    }

    success
}

fn create_uri(
    upstream_config: &UpstreamConfig,
    authority: &Authority,
    cfg: &HealthConfig,
) -> Result<Uri> {
    Ok(format!("{}://{}/{}", upstream_config.scheme, authority, cfg.path).parse()?)
}
