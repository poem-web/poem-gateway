use poem::listener::{AcceptorExt, BoxAcceptor, Listener};
use serde::{Deserialize, Serialize};

use crate::config::AcceptorConfig;

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Config {
    #[serde(default = "default_bind")]
    bind: String,
}

fn default_bind() -> String {
    "127.0.0.1:8080".to_string()
}

#[typetag::serde(name = "tcp")]
#[async_trait::async_trait]
impl AcceptorConfig for Config {
    async fn create(&self) -> anyhow::Result<BoxAcceptor> {
        Ok(poem::listener::TcpListener::bind(&self.bind)
            .into_acceptor()
            .await?
            .boxed())
    }
}
