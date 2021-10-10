use poem::listener::{AcceptorExt, BoxAcceptor, Listener as _};
use serde::{Deserialize, Serialize};

use crate::config::ListenerConfig;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TcpListener {
    #[serde(default = "default_bind")]
    bind: String,
}

fn default_bind() -> String {
    "127.0.0.1:8080".to_string()
}

#[typetag::serde(name = "tcp")]
#[async_trait::async_trait]
impl ListenerConfig for TcpListener {
    async fn create(&self) -> anyhow::Result<BoxAcceptor> {
        Ok(poem::listener::TcpListener::bind(&self.bind)
            .into_acceptor()
            .await?
            .boxed())
    }
}
