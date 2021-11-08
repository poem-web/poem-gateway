use anyhow::Result;
use dyn_clone::DynClone;
use poem::listener::{AcceptorExt, BoxAcceptor, IntoTlsConfigStream};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TlsConfig {
    pub cert: String,
    pub key: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ListenerConfig {
    pub acceptor: Box<dyn AcceptorConfig>,
    pub tls: Option<TlsConfig>,
}

impl ListenerConfig {
    pub async fn create_acceptor(&self) -> Result<BoxAcceptor> {
        let mut acceptor = self.acceptor.create().await?;
        if let Some(tls) = &self.tls {
            acceptor = acceptor
                .tls(
                    poem::listener::TlsConfig::new()
                        .cert(tls.cert.clone())
                        .key(tls.key.clone())
                        .into_stream()?,
                )
                .boxed();
        }
        Ok(acceptor)
    }
}

#[typetag::serde(tag = "type")]
#[async_trait::async_trait]
pub trait AcceptorConfig: DynClone + Send + Sync + 'static {
    async fn create(&self) -> Result<BoxAcceptor>;
}

dyn_clone::clone_trait_object!(AcceptorConfig);
