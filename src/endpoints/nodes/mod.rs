mod fixed_nodes;
mod health_check;

pub use fixed_nodes::FixedNodes;
use poem::http::uri::Authority;

pub trait Nodes {
    fn get(&self, callback: &(dyn Fn(&[Authority]) -> Option<Authority>)) -> Option<Authority>;
}
