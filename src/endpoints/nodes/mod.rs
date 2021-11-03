mod fixed_nodes;
mod health_check;

pub use fixed_nodes::FixedNodes;
use poem::http::uri::Authority;

#[async_trait::async_trait]
pub trait Nodes {
    async fn get(
        &self,
        callback: &(dyn Fn(&[Authority]) -> Option<Authority> + Send + Sync),
    ) -> Option<Authority>;
}
