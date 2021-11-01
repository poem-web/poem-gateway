mod fixed_nodes;

pub use fixed_nodes::FixedNodes;
use poem::http::uri::Authority;

#[async_trait::async_trait]
pub trait Nodes {
    async fn get<'a>(
        &'a self,
        callback: &(dyn Fn(&'a [Authority]) -> Option<Authority> + Send + Sync),
    ) -> Option<Authority>;
}
