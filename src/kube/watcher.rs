use crate::{
    config::KubeConfig, db::Storage, error::Error, ClusterObjectMeta, EventHandlerFactory, Result,
    SCHEMA,
};
use futures::{StreamExt, TryStreamExt};
use kube::{
    api::ListParams,
    config::{KubeConfigOptions, Kubeconfig},
    core::{DynamicObject, GroupVersionKind, TypeMeta},
    discovery::{self, ApiCapabilities},
    runtime::watcher,
    Api, Client, Config,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::task::JoinHandle;

struct ApiConfig {
    caps: ApiCapabilities,
    api: Api<DynamicObject>,
    gvk: GroupVersionKind,
}

pub struct Watcher {
    r: Vec<TypeMeta>,
    pub client: Client,
    watch_pool: HashMap<GroupVersionKind, ApiConfig>,
    store: Arc<Box<dyn Storage>>,
}

impl Watcher {
    pub async fn new(r: Vec<TypeMeta>, c: &KubeConfig, store: Box<dyn Storage>) -> Result<Watcher> {
        let kubeconfig = match (&c.config_path, &c.raw) {
            (_, Some(data)) => data.to_owned(),
            (Some(path), None) => Kubeconfig::read_from(path)?,
            (None, None) => {
                panic!("kubeconfig information is empty, please set config-path or raw kubeconfig data")
            }
        };
        let rest_config =
            Config::from_custom_kubeconfig(kubeconfig, &KubeConfigOptions::default()).await?;
        let client = Client::try_from(rest_config)?;
        let pool_cap = r.len();
        Ok(Watcher {
            r,
            client,
            watch_pool: HashMap::with_capacity(pool_cap),
            store: Arc::new(store),
        })
    }

    pub async fn build_api_pool(&mut self) -> Result<()> {
        let tasks: Vec<JoinHandle<Result<ApiConfig>>> = self
            .r
            .iter()
            .map(|r| r.to_owned())
            .map(|r| {
                let client = self.client.clone();
                tokio::spawn(async {
                    let gvk: GroupVersionKind = r.try_into()?;
                    let (ar, caps) = discovery::pinned_kind(&client.clone(), &gvk).await?;
                    let api = Api::<DynamicObject>::all_with(client, &ar);
                    Ok::<ApiConfig, Error>(ApiConfig { caps, api, gvk })
                })
            })
            .collect();
        for task in tasks {
            let api_config = task.await??;
            self.watch_pool.insert(api_config.gvk.clone(), api_config);
        }
        Ok(())
    }

    pub async fn watch(&self) -> Result<()> {
        if SCHEMA.lock().unwrap().len() != self.r.len() {
            panic!(
                "please make sure all resource EventHandler were registered,
            kufu decide to watch {} kind k8s reousrce, but only have {} handler
            ",
                self.r.len(),
                SCHEMA.lock().unwrap().len()
            )
        }
        let mut watchers = Vec::with_capacity(self.r.len());
        for (gvk, api_config) in self.watch_pool.iter() {
            let mut events = watcher(api_config.api.clone(), ListParams::default()).boxed();
            let factory = self.dispatcher(gvk).await;
            let object_meta = ClusterObjectMeta::new(
                "default".to_string(),
                gvk.to_owned(),
                api_config.caps.clone(),
            );
            let client = self.client.clone();
            let s = Arc::clone(&self.store);

            // use queue for raise concurrency https://docs.rs/kube/latest/kube/runtime/utils/struct.StreamBackoff.html
            watchers.push(tokio::spawn(async move {
                let handler = factory.build(object_meta, client, s);
                while let Some(e) = events.try_next().await? {
                    handler.process(e).await?;
                }
                Ok::<(), Error>(())
            }));
        }

        #[allow(unused_must_use)]
        for w in watchers {
            w.await?;
        }
        Ok(())
    }

    async fn dispatcher(&self, gvk: &GroupVersionKind) -> Box<dyn EventHandlerFactory> {
        SCHEMA.lock().unwrap().get(gvk).unwrap().clone_box()
    }
}
