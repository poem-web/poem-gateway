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

#[async_trait::async_trait]
impl Nodes for FixedNodes {
    #[inline]
    async fn get<'a>(
        &'a self,
        callback: &(dyn Fn(&'a [Authority]) -> Option<Authority> + Send + Sync),
    ) -> Option<Authority> {
        callback(&self.nodes)
    }
}
