use std::sync::Arc;

use anyhow::Result;
use hyper::{Body, Client};
use hyper_rustls::HttpsConnector;
use poem::http::{uri::Authority, Method, StatusCode, Uri};
use tokio::{sync::mpsc, time::Duration};

use crate::endpoints::{nodes::Nodes, upstream::UpstreamScheme};

enum Command {
    Get(Box<dyn Fn(&[Authority]) -> Option<Authority> + Send + Sync>),
}

pub struct HealthCheck {
    tx: mpsc::Sender<Command>,
}

pub struct HealthConfig {
    scheme: UpstreamScheme,
    path: String,
    interval: Duration,
    status: Vec<StatusCode>,
}

impl HealthCheck {
    pub fn new(nodes: Vec<Authority>, cfg: HealthConfig) -> Self {
        let (tx, mut rx) = mpsc::channel(1);
        let cfg = Arc::new(cfg);
        tokio::spawn(checker(nodes, cfg, rx));
        Self { tx }
    }
}

impl Nodes for HealthCheck {
    fn get(&self, callback: &dyn Fn(&[Authority]) -> Option<Authority>) -> Option<Authority> {
        todo!()
    }
}

async fn checker(
    nodes: Vec<Authority>,
    cfg: Arc<HealthConfig>,
    tx_command: mpsc::Receiver<Command>,
) {
    loop {
        // tokio::select! {}
    }
}

async fn do_check(
    nodes: Vec<Authority>,
    cfg: Arc<HealthConfig>,
    reply_tx: mpsc::Sender<(Vec<Authority>, Vec<Authority>)>,
) {
    let tasks: Vec<_> = nodes
        .into_iter()
        .map({
            let cfg = cfg.clone();
            move |authority| {
                let cfg = cfg.clone();
                tokio::spawn(async move {
                    let uri = match create_uri(&authority, &cfg) {
                        Ok(uri) => uri,
                        Err(_) => return (false, authority),
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
                        Ok(resp) => (cfg.status.contains(&resp.status()), authority),
                        Err(_) => (false, authority),
                    }
                })
            }
        })
        .collect();

    let mut success = Vec::new();
    let mut fail = Vec::new();
    for task in tasks {
        match task.await.unwrap() {
            (true, authority) => success.push(authority),
            (false, authority) => fail.push(authority),
        }
    }

    let _ = reply_tx.send((success, fail)).await;
}

fn create_uri(authority: &Authority, cfg: &HealthConfig) -> Result<Uri> {
    let scheme = match cfg.scheme {
        UpstreamScheme::Http => "http",
        UpstreamScheme::Https => "https",
    };
    Ok(format!("{}://{}/{}", scheme, authority, cfg.path).parse()?)
}
