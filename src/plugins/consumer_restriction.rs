use std::{collections::HashSet, convert::TryInto, sync::Arc};

use anyhow::{Context, Result};
use poem::{http::StatusCode, Request, Response};
use serde::{Deserialize, Serialize};

use crate::{
    config::PluginConfig,
    plugins::{NextPlugin, Plugin, PluginContext},
};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Config {
    #[serde(default)]
    whitelist: HashSet<String>,
    #[serde(default)]
    blacklist: HashSet<String>,
    #[serde(default = "default_rejected_code")]
    rejected_code: u16,
}

const fn default_rejected_code() -> u16 {
    403
}

#[typetag::serde(name = "consumerRestriction ")]
#[async_trait::async_trait]
impl PluginConfig for Config {
    async fn create(&self) -> Result<Arc<dyn Plugin>> {
        Ok(Arc::new(ConsumerRestriction {
            validator: if !self.whitelist.is_empty() {
                ConsumerNameValidator::WhiteList(self.whitelist.clone())
            } else if !self.blacklist.is_empty() {
                ConsumerNameValidator::BlackList(self.blacklist.clone())
            } else {
                ConsumerNameValidator::WhiteList(Default::default())
            },
            rejected_code: self
                .rejected_code
                .try_into()
                .context("invalid status code")?,
        }))
    }
}

enum ConsumerNameValidator {
    BlackList(HashSet<String>),
    WhiteList(HashSet<String>),
}

impl ConsumerNameValidator {
    fn check(&self, consumer_name: Option<&str>) -> bool {
        match self {
            ConsumerNameValidator::BlackList(names) => match consumer_name {
                Some(name) => !names.contains(name),
                None => true,
            },
            ConsumerNameValidator::WhiteList(names) => match consumer_name {
                Some(name) => names.contains(name),
                None => false,
            },
        }
    }
}

struct ConsumerRestriction {
    validator: ConsumerNameValidator,
    rejected_code: StatusCode,
}

#[async_trait::async_trait]
impl Plugin for ConsumerRestriction {
    fn priority(&self) -> i32 {
        1000
    }

    async fn call(&self, req: Request, ctx: &mut PluginContext, next: NextPlugin<'_>) -> Response {
        if self.validator.check(ctx.consumer_name()) {
            return self.rejected_code.into();
        }
        next.call(ctx, req).await
    }
}
