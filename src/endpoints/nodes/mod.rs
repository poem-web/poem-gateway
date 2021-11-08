mod fixed_nodes;
mod health_check;

pub use fixed_nodes::FixedNodes;
pub use health_check::{HealthCheck, HealthConfig};
use poem::http::uri::Authority;

pub trait Nodes: Send + Sync + 'static {
    fn get(&self, callback: &(dyn Fn(&[Authority]) -> Option<Authority>)) -> Option<Authority>;
}
