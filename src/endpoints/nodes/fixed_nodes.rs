use poem::http::uri::Authority;

use crate::endpoints::nodes::Nodes;

pub struct FixedNodes {
    nodes: Vec<Authority>,
}

impl FixedNodes {
    pub fn new(nodes: Vec<Authority>) -> Self {
        Self { nodes }
    }
}

impl Nodes for FixedNodes {
    #[inline]
    fn get(&self, callback: &(dyn Fn(&[Authority]) -> Option<Authority>)) -> Option<Authority> {
        callback(&self.nodes)
    }
}
