mod cidr;

use poem::Request;

pub trait ConsumerFilter: Send + Sync + 'static {
    fn check(&self, req: &Request) -> bool;
}
