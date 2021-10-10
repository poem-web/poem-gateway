use std::{path::PathBuf, time::Duration};

use futures_util::stream::BoxStream;

use crate::config::{Config, ConfigProvider};

pub struct FileProvider {
    path: PathBuf,
}

impl FileProvider {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

impl ConfigProvider for FileProvider {
    fn watch(&self) -> BoxStream<'_, Config> {
        Box::pin(async_stream::stream! {
            let mut current_data: Option<String> = None;

            info!(path = %self.path.display(), "watch the configuration file.");

            loop {
                if let Ok(data) = tokio::fs::read_to_string(&self.path).await {
                    if current_data.as_ref() != Some(&data) {
                        info!(path = %self.path.display(), "configuration file changed.");

                        current_data = Some(data.clone());
                        match serde_yaml::from_str::<Config>(&data) {
                            Ok(cfg) => yield cfg,
                            Err(err) => {
                                error!(
                                    error = %err,
                                    "invalid configuration file.",
                                );
                            }
                        }
                    }
                }

                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        })
    }
}
