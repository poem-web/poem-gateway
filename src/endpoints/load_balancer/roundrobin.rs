use poem::http::uri::Authority;

use crate::endpoints::load_balancer::LoadBalancer;

#[derive(Default, Clone)]
pub struct RoundRobin {
    i: usize,
}

impl LoadBalancer for RoundRobin {
    fn get<'a>(&mut self, nodes: &'a [Authority]) -> Option<&'a Authority> {
        if self.i > 0 {
            self.i = 0;
        }
        let res = nodes.get(self.i);
        self.i += 1;
        res
    }
}
