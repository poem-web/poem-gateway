use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use anyhow::Result;
use futures_util::stream::BoxStream;
use serde::{Deserialize, Serialize};

use crate::config::{ConfigProvider, ConfigProviderConfig, ProxyConfig};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Config {
    path: PathBuf,
}

#[typetag::serde(name = "file")]
#[async_trait::async_trait]
impl ConfigProviderConfig for Config {
    async fn create(&self, config_root: &Path) -> Result<Arc<dyn ConfigProvider>> {
        Ok(Arc::new(FileProvider {
            path: config_root.join(&self.path),
        }))
    }
}

struct FileProvider {
    path: PathBuf,
}

impl ConfigProvider for FileProvider {
    fn watch(&self) -> BoxStream<'_, Result<ProxyConfig>> {
        Box::pin(async_stream::stream! {
            let mut current_data: Option<String> = None;

            info!(path = %self.path.display(), "watch the configuration file.");

            loop {
                if let Ok(data) = tokio::fs::read_to_string(&self.path).await {
                    if current_data.as_ref() != Some(&data) {
                        info!("configuration file changed.");

                        current_data = Some(data.clone());
                        match serde_yaml::from_str::<ProxyConfig>(&data) {
                            Ok(cfg) => yield Ok(cfg),
                            Err(err) => {
                                error!(
                                    error = %err,
                                    "invalid configuration file.",
                                );
                            }
                        }
                    }
                }

                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        })
    }
}
