use std::sync::Arc;

use anyhow::Result;
use cidr::IpCidr;
use poem::{web::RemoteAddr, Addr, Request};
use serde::{Deserialize, Serialize};

use crate::{config::ConsumerFilterConfig, consumer_filters::ConsumerFilter};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct CidrConfig {
    ip: Vec<IpCidr>,
}

#[typetag::serde(name = "cidr")]
impl ConsumerFilterConfig for CidrConfig {
    fn create(&self) -> Result<Arc<dyn ConsumerFilter>> {
        Ok(Arc::new(Cidr {
            ip: self.ip.clone(),
        }))
    }
}

struct Cidr {
    ip: Vec<IpCidr>,
}

impl ConsumerFilter for Cidr {
    fn check(&self, req: &Request) -> bool {
        let remote_addr = req.remote_addr();
        if let RemoteAddr(Addr::SocketAddr(remote_addr)) = remote_addr {
            self.ip.iter().any(|ip| ip.contains(&remote_addr.ip()))
        } else {
            false
        }
    }
}
