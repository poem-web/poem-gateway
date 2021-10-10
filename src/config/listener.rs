use anyhow::Result;
use poem::listener::BoxAcceptor;

#[typetag::serde(tag = "type")]
#[async_trait::async_trait]
pub trait ListenerConfig: Send + Sync + 'static {
    async fn create(&self) -> Result<BoxAcceptor>;
}
