mod graphql;
mod openapi;

use std::sync::Arc;

use once_cell::sync::OnceCell;

use crate::config::ConfigProvider;

static GLOBAL_PROVIDER: OnceCell<Arc<dyn ConfigProvider>> = OnceCell::new();

pub fn set_global_provider(provider: Arc<dyn ConfigProvider>) {
    let _ = GLOBAL_PROVIDER.set(provider);
}

pub fn global_provider() -> Arc<dyn ConfigProvider> {
    GLOBAL_PROVIDER.get().cloned().unwrap()
}
