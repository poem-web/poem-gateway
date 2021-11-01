use poem::http::uri::Authority;

mod roundrobin;

pub use roundrobin::RoundRobin;

pub trait LoadBalancer: Send + Sync + 'static {
    fn get<'a>(&mut self, nodes: &'a [Authority]) -> Option<&'a Authority>;
}
