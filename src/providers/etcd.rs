use std::{collections::HashMap, marker::PhantomData, path::Path, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use etcd_client::{
    Client, Compare, CompareOp, EventType, GetOptions, KeyValue, KvClient, Txn, TxnOp, WatchOptions,
};
use futures_util::{stream::BoxStream, StreamExt};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::config::{
    ConfigProvider, ConfigProviderConfig, ConsumerConfig, ListenerConfig, PluginConfig,
    ProxyConfig, ResourcesOperation, RouteConfig, ServiceConfig, ServiceNotFoundError, ServiceRef,
};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Config {
    endpoints: Vec<String>,
    #[serde(default)]
    prefix: String,
}

#[typetag::serde(name = "etcd")]
#[async_trait::async_trait]
impl ConfigProviderConfig for Config {
    async fn create(&self, _config_root: &Path) -> Result<Arc<dyn ConfigProvider>> {
        let etcd_cli = Client::connect(&self.endpoints, None).await?;
        Ok(Arc::new(EtcdProvider {
            endpoints: self.endpoints.clone(),
            etcd_cli,
            prefix: self.prefix.clone(),
        }))
    }
}

struct EtcdProvider {
    endpoints: Vec<String>,
    etcd_cli: Client,
    prefix: String,
}

impl EtcdProvider {
    fn create_operation<T: Serialize + DeserializeOwned + Send + Sync + 'static>(
        &self,
        name: &str,
    ) -> Box<dyn ResourcesOperation<T>> {
        Box::new(EtcdResourcesOperation {
            etcd_cli: self.etcd_cli.clone(),
            prefix: self.prefix.clone(),
            resource_prefix: format!("{}/resources/{}/", self.prefix, name),
            _mark: Default::default(),
        })
    }
}

impl ConfigProvider for EtcdProvider {
    fn watch(&self) -> BoxStream<'_, Result<ProxyConfig>> {
        let mut kv_cli = self.etcd_cli.kv_client();
        let prefix = self.prefix.clone();

        info!(endpoints = ?self.endpoints, "watch the configuration from etcd.");

        Box::pin(async_stream::try_stream! {
            let resources_prefix = format!("{}/resources", prefix);
            let mut values = ConfigValues::default();
            let resp = kv_cli
                .get(resources_prefix.clone(), Some(GetOptions::new().with_prefix()))
                .await?;
            let header = resp.header().expect("ResponseHeader");
            let revision = header.revision();

            for kv in resp.kvs() {
                values.add(kv, &prefix)?;
            }
            let cfg = values.generate_config()?;
            yield cfg;

            let mut watch_cli = self.etcd_cli.watch_client();
            let opts = WatchOptions::new().with_start_revision(revision + 1).with_prefix();
            let (_, mut stream) = watch_cli.watch(resources_prefix, Some(opts)).await?;

            while let Some(resp) = stream.next().await.transpose()? {
                for event in resp.events() {
                    if event.event_type() == EventType::Put {
                        values.add(event.kv().unwrap(), &prefix)?;
                    } else if event.event_type() == EventType::Delete {
                        values.remove(event.kv().unwrap(), &prefix)?;
                    }
                }

                info!("configuration file changed.");
                let cfg = values.generate_config()?;
                yield cfg;
            }
        })
    }

    fn listeners(&self) -> Result<Box<dyn ResourcesOperation<ListenerConfig>>> {
        Ok(self.create_operation("listeners"))
    }

    fn consumers(&self) -> Result<Box<dyn ResourcesOperation<ConsumerConfig>>> {
        Ok(self.create_operation("consumers"))
    }

    fn routes(&self) -> Result<Box<dyn ResourcesOperation<RouteConfig>>> {
        Ok(Box::new(RouteOperation(EtcdResourcesOperation {
            etcd_cli: self.etcd_cli.clone(),
            prefix: self.prefix.clone(),
            resource_prefix: format!("{}/resources/{}/", self.prefix, "routes"),
            _mark: Default::default(),
        })))
    }

    fn services(&self) -> Result<Box<dyn ResourcesOperation<ServiceConfig>>> {
        Ok(self.create_operation("services"))
    }

    fn global_plugins(&self) -> Result<Box<dyn ResourcesOperation<Box<dyn PluginConfig>>>> {
        Ok(self.create_operation("globalPlugins"))
    }
}

#[derive(Default)]
struct ConfigValues {
    listeners: HashMap<String, Vec<u8>>,
    consumers: HashMap<String, Vec<u8>>,
    routes: HashMap<String, Vec<u8>>,
    services: HashMap<String, Vec<u8>>,
    global_plugins: HashMap<String, Vec<u8>>,
}

impl ConfigValues {
    fn add(&mut self, kv: &KeyValue, prefix: &str) -> Result<()> {
        let key = kv
            .key_str()
            .context("invalid key")?
            .strip_prefix(&format!("{}/resources/", prefix))
            .unwrap();

        if let Some(id) = key.strip_prefix("listeners/") {
            let id = id.parse().context("invalid listener id")?;
            self.listeners.insert(id, kv.value().to_vec());
        } else if let Some(id) = key.strip_prefix("consumers/") {
            let id = id.parse().context("invalid consumer id")?;
            self.consumers.insert(id, kv.value().to_vec());
        } else if let Some(id) = key.strip_prefix("routes/") {
            let id = id.parse().context("invalid route id")?;
            self.routes.insert(id, kv.value().to_vec());
        } else if let Some(id) = key.strip_prefix("services/") {
            let id = id.parse().context("invalid service id")?;
            self.services.insert(id, kv.value().to_vec());
        } else if let Some(id) = key.strip_prefix("globalPlugins/") {
            let id = id.parse().context("invalid global plugin id")?;
            self.global_plugins.insert(id, kv.value().to_vec());
        }

        Ok(())
    }

    fn remove(&mut self, kv: &KeyValue, prefix: &str) -> Result<()> {
        let key = kv
            .key_str()
            .context("invalid key")?
            .strip_prefix(&format!("{}/resources/", prefix))
            .unwrap();

        if let Some(id) = key.strip_prefix("listeners/") {
            self.listeners.remove(id);
        } else if let Some(id) = key.strip_prefix("consumers/") {
            self.consumers.remove(id);
        } else if let Some(id) = key.strip_prefix("routes/") {
            self.routes.remove(id);
        } else if let Some(id) = key.strip_prefix("services/") {
            self.services.remove(id);
        } else if let Some(id) = key.strip_prefix("globalPlugins/") {
            self.global_plugins.remove(id);
        }

        Ok(())
    }

    fn generate_config(&self) -> Result<ProxyConfig> {
        let mut cfg = ProxyConfig::default();

        for data in self.listeners.values() {
            cfg.listeners.push(serde_json::from_slice(data)?);
        }

        for data in self.consumers.values() {
            cfg.consumers.push(serde_json::from_slice(data)?);
        }

        for data in self.routes.values() {
            cfg.routes.push(serde_json::from_slice(data)?);
        }

        for (id, data) in &self.services {
            let mut service_cfg: ServiceConfig = serde_json::from_slice(data)?;
            service_cfg.name = Some(id.clone());
            cfg.services.push(service_cfg);
        }

        for data in self.global_plugins.values() {
            cfg.global_plugins.push(serde_json::from_slice(data)?);
        }

        Ok(cfg)
    }
}

async fn take_id(mut kv_cli: KvClient, prefix: String) -> Result<String> {
    let key = format!("{}/auto_increment_id", prefix);

    loop {
        let resp = kv_cli.get(key.clone(), None).await?;

        if resp.kvs().is_empty() {
            let compares = vec![Compare::create_revision(key.clone(), CompareOp::Equal, 0)];
            let and_then = vec![TxnOp::put(key.clone(), "1", None)];
            let resp = kv_cli
                .txn(Txn::new().when(compares).and_then(and_then))
                .await?;
            if resp.succeeded() {
                return Ok("1".to_string());
            }
        } else {
            let version = resp.kvs()[0].version();
            let prev_id = resp.kvs()[0].value_str()?.parse::<u64>()?;

            let resp = kv_cli
                .txn(
                    Txn::new()
                        .when(vec![Compare::version(
                            key.clone(),
                            CompareOp::Equal,
                            version,
                        )])
                        .and_then(vec![TxnOp::put(
                            key.clone(),
                            format!("{}", prev_id + 1),
                            None,
                        )]),
                )
                .await?;
            if resp.succeeded() {
                return Ok((prev_id + 1).to_string());
            }
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

struct EtcdResourcesOperation<T> {
    etcd_cli: Client,
    prefix: String,
    resource_prefix: String,
    _mark: PhantomData<T>,
}

#[async_trait::async_trait]
impl<T: Serialize + DeserializeOwned + Send + Sync + 'static> ResourcesOperation<T>
    for EtcdResourcesOperation<T>
{
    async fn get_all(&self) -> Result<Vec<(String, T)>> {
        let mut kv_cli = self.etcd_cli.kv_client();
        let resp = kv_cli
            .get(
                self.resource_prefix.clone(),
                Some(GetOptions::new().with_prefix()),
            )
            .await?;
        let mut items = Vec::new();

        for kv in resp.kvs() {
            let id = kv.key_str()?.parse()?;
            let item = serde_json::from_str(kv.value_str()?)?;
            items.push((id, item));
        }

        Ok(items)
    }

    async fn get(&self, id: &str) -> Result<Option<T>> {
        let mut kv_cli = self.etcd_cli.kv_client();
        let resp = kv_cli
            .get(format!("{}{}", self.resource_prefix, id), None)
            .await?;
        if resp.kvs().is_empty() {
            return Ok(None);
        }
        let item = serde_json::from_str(resp.kvs()[0].value_str()?)?;
        Ok(Some(item))
    }

    async fn create(&self, config: T) -> Result<String> {
        let mut kv_cli = self.etcd_cli.kv_client();
        let id = take_id(kv_cli.clone(), self.prefix.clone()).await?;
        let data = serde_json::to_vec(&config)?;
        kv_cli
            .put(format!("{}{}", self.resource_prefix, id), data, None)
            .await?;
        Ok(id)
    }

    async fn delete(&self, id: &str) -> Result<bool> {
        let mut kv_cli = self.etcd_cli.kv_client();
        let resp = kv_cli
            .delete(format!("{}{}", self.resource_prefix, id), None)
            .await?;
        Ok(resp.deleted() > 0)
    }

    async fn update(&self, id: &str, config: T) -> Result<()> {
        let mut kv_cli = self.etcd_cli.kv_client();
        let key = format!("{}{}", self.resource_prefix, id);
        let data = serde_json::to_vec(&config)?;
        let resp = kv_cli
            .txn(
                Txn::new()
                    .when(vec![Compare::version(key.clone(), CompareOp::Greater, 0)])
                    .and_then(vec![TxnOp::put(key, data, None)]),
            )
            .await?;
        ensure!(resp.succeeded(), "resource `{}` is not found.", id);
        Ok(())
    }
}

struct RouteOperation(EtcdResourcesOperation<RouteConfig>);

#[async_trait::async_trait]
impl ResourcesOperation<RouteConfig> for RouteOperation {
    async fn get_all(&self) -> Result<Vec<(String, RouteConfig)>> {
        self.0.get_all().await
    }

    async fn get(&self, id: &str) -> Result<Option<RouteConfig>> {
        self.0.get(id).await
    }

    async fn create(&self, config: RouteConfig) -> Result<String> {
        match &config.service_ref {
            ServiceRef::Reference(service_id) => {
                let mut kv_cli = self.0.etcd_cli.kv_client();
                let service_key =
                    format!("{}/resources/{}/{}", self.0.prefix, "services", service_id);
                let id = take_id(kv_cli.clone(), self.0.prefix.clone()).await?;
                let data = serde_json::to_vec(&config)?;

                let txn = Txn::new()
                    .when(vec![Compare::version(service_key, CompareOp::Greater, 0)])
                    .and_then(vec![TxnOp::put(
                        format!("{}{}", self.0.resource_prefix, id),
                        data,
                        None,
                    )]);
                let resp = kv_cli.txn(txn).await?;
                ensure!(resp.succeeded(), ServiceNotFoundError(service_id.clone()));
                Ok(id)
            }
            ServiceRef::Inline(_) => self.0.create(config).await,
        }
    }

    async fn delete(&self, id: &str) -> Result<bool> {
        self.0.delete(id).await
    }

    async fn update(&self, id: &str, config: RouteConfig) -> Result<()> {
        self.0.update(id, config).await
    }
}
