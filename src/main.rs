#![allow(clippy::mutable_key_type)]

#[macro_use]
extern crate tracing;
#[macro_use]
extern crate anyhow;

use std::{path::PathBuf, time::Duration};

use structopt::StructOpt;
use tokio::task::JoinHandle;
use tokio_stream::StreamExt;

use crate::{
    api::set_global_provider,
    config::{DebouncedStream, GatewayConfig},
};

mod api;
mod config;
mod consumer_filters;
mod endpoints;
mod listeners;
mod plugins;
mod providers;

#[derive(Debug, StructOpt)]
#[structopt(name = "poem-gateway")]
struct Options {
    /// Path of the config file
    #[structopt(parse(from_os_str))]
    pub file: PathBuf,
}

fn init_tracing() {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();
}

#[tokio::main]
async fn main() {
    let options: Options = Options::from_args();
    init_tracing();

    let gateway_config = {
        let file = match std::fs::File::open(&options.file) {
            Ok(file) => file,
            Err(err) => {
                error!(error = %err, "failed to open configuration file.");
                return;
            }
        };
        match serde_yaml::from_reader::<_, GatewayConfig>(file) {
            Ok(gateway_config) => gateway_config,
            Err(err) => {
                error!(error = %err, "failed to parse configuration file.");
                return;
            }
        }
    };

    let config_provider = match gateway_config
        .provider
        .create(options.file.parent().unwrap())
        .await
    {
        Ok(config_provider) => config_provider,
        Err(err) => {
            error!(error = %err, "failed to create configuration provider");
            return;
        }
    };
    set_global_provider(config_provider.clone());

    // create admin server
    let fut = match gateway_config.admin.start_server().await {
        Ok(fut) => fut,
        Err(err) => {
            error!(error = %err, "failed to create static server.");
            return;
        }
    };
    tokio::spawn(fut);

    // create proxy server
    let mut current_server_handle: Option<JoinHandle<()>> = None;

    loop {
        let mut watcher_stream =
            DebouncedStream::new(config_provider.watch(), Duration::from_secs(2));

        while let Some(res) = watcher_stream.next().await {
            let cfg = match res {
                Ok(cfg) => cfg,
                Err(err) => {
                    error!(error = %err, "watcher error");
                    break;
                }
            };

            if let Some(handle) = current_server_handle.take() {
                info!("reload configuration.");
                handle.abort();
                tokio::time::sleep(Duration::from_secs(1)).await;
            }

            let fut = match cfg.start_server().await {
                Ok(server) => server,
                Err(err) => {
                    error!(error = %err, "failed to create dynamic server.");
                    continue;
                }
            };

            let handle = tokio::spawn(async move {
                let _ = fut.await;
            });
            current_server_handle = Some(handle);
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
