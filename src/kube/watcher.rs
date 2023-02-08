use crate::{config::KubeConfig, error::Error, utils, Result};
use futures::{StreamExt, TryStreamExt};
use kube::{
    api::ListParams,
    client::ConfigExt,
    config::Kubeconfig,
    core::{DynamicObject, GroupVersionKind, TypeMeta},
    discovery::{self, ApiCapabilities},
    runtime::watcher,
    Api, Client, Config,
};
use std::collections::HashMap;
use tokio::task::JoinHandle;
use tower::ServiceBuilder;

trait EventHandlerFactory: FactoryClone {
    fn build(&self, caps: ApiCapabilities) -> Box<dyn EventHandler + Send>;
}

trait FactoryClone {
    fn clone_box(&self) -> Box<dyn EventHandlerFactory + Send>;
}

trait EventHandler {
    fn process(&self, e: watcher::Event<DynamicObject>) -> Result<()>;
}

struct ApiConfig {
    caps: ApiCapabilities,
    api: Api<DynamicObject>,
    gvk: GroupVersionKind,
}

pub struct Watcher<'a> {
    r: &'a Vec<TypeMeta>,
    pub client: Client,
    watch_pool: HashMap<GroupVersionKind, ApiConfig>,
    pub factorys: HashMap<GroupVersionKind, Box<dyn EventHandlerFactory + Send>>,
}

impl<'a> Watcher<'a> {
    pub async fn new(r: &'a Vec<TypeMeta>, c: &KubeConfig) -> Result<Watcher<'a>> {
        let kubeconfig = match (&c.config_path, &c.raw) {
            (_, Some(data)) => data.to_owned(),
            (Some(path), None) => Kubeconfig::read_from(path)?,
            (None, None) => {
                panic!("kubeconfig information is empty, please set config-path or raw kubeconfig data")
            }
        };
        let config_options = utils::get_current_kubeconfig_options(&kubeconfig);
        let rest_config = Config::from_custom_kubeconfig(kubeconfig, &config_options).await?;
        let service = ServiceBuilder::new()
            .layer(rest_config.base_uri_layer())
            .option_layer(rest_config.auth_layer()?)
            .service(hyper::Client::new());
        Ok(Watcher {
            r: r,
            client: Client::new(service, rest_config.default_namespace),
            factorys: HashMap::with_capacity(r.len()),
            watch_pool: HashMap::with_capacity(r.len()),
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
        if self.factorys.len() != self.r.len() {
            panic!(
                "please make sure all resource EventHandler were registered,
            kufu decide to watch {} kind k8s reousrce, but only have {} handler
            ",
                self.r.len(),
                self.factorys.len()
            )
        }
        let mut watchers = Vec::with_capacity(self.r.len());
        for (gvk, api_config) in self.watch_pool.iter() {
            let mut events = watcher(api_config.api.clone(), ListParams::default()).boxed();
            let factory = self.dispatcher(gvk);
            let caps = api_config.caps.clone();
            watchers.push(tokio::spawn(async move {
                let handler = factory.build(caps);
                while let Some(e) = events.try_next().await? {
                    handler.process(e)?;
                }
                Ok::<(), Error>(())
            }));
        }
        for w in watchers {
            w.await?;
        }
        Ok(())
    }

    fn dispatcher(&self, gvk: &GroupVersionKind) -> Box<dyn EventHandlerFactory + Send> {
        self.factorys.get(gvk).unwrap().clone_box()
    }
}

pub fn register(
    watcher: &mut Watcher,
    gvk: GroupVersionKind,
    f: Box<dyn EventHandlerFactory + Send>,
) {
    watcher.factorys.insert(gvk, f);
}
