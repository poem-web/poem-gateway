#[macro_use]
extern crate tracing;
#[macro_use]
extern crate anyhow;

mod config;
mod consumer_filters;
mod listeners;
mod plugins;
mod service_targets;

use std::{path::PathBuf, time::Duration};

use structopt::StructOpt;
use tokio::task::JoinHandle;
use tokio_stream::StreamExt;

use crate::config::{providers::FileProvider, ConfigProvider};

#[derive(Debug, StructOpt)]
#[structopt(name = "poem-gateway")]
struct Options {
    /// Path of the config file
    #[structopt(parse(from_os_str))]
    pub config: PathBuf,
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

    info!(
        path = %options.config.display(),
        "load configuration file.",
    );

    let mut current_server_handle: Option<JoinHandle<()>> = None;
    let config_provider = FileProvider::new(&options.config);
    let mut watcher_stream = config_provider.watch();

    while let Some(cfg) = watcher_stream.next().await {
        if let Some(handle) = current_server_handle.take() {
            info!(path = %options.config.display(), "reload configuration file.");
            handle.abort();
            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        let server = match cfg.create_server().await {
            Ok(server) => server,
            Err(err) => {
                error!(error = %err, "failed to create the server.");
                continue;
            }
        };
        let ep = match cfg.create_endpoint().await {
            Ok(ep) => ep,
            Err(err) => {
                error!(error = %err, "failed to initialize the server.");
                continue;
            }
        };

        let handle = tokio::spawn(async move {
            if let Err(err) = server.run(ep).await {
                error!(error = %err, "server error");
            }
        });
        current_server_handle = Some(handle);
    }
}
