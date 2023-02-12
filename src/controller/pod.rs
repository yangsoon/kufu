use super::Controller;
use crate::{
    db::Storage, ClusterObject, ClusterObjectMeta, EventHandler, EventHandlerFactory, FactoryClone,
    Result,
};
use async_trait::async_trait;
use kube::{
    core::DynamicObject,
    runtime::watcher::Event,
    runtime::watcher::Event::{Applied, Deleted, Restarted},
    Client,
};
use std::sync::Arc;
use tracing::info;

#[allow(dead_code)]
struct PodController {
    client: Client,
    store: Arc<Box<dyn Storage>>,
    meta: ClusterObjectMeta,
}

impl PodController {
    fn new(client: Client, meta: ClusterObjectMeta, store: Arc<Box<dyn Storage>>) -> PodController {
        PodController {
            client,
            meta,
            store,
        }
    }
    fn to_cluster_obj<'a>(&'a self, o: &'a DynamicObject) -> ClusterObject {
        ClusterObject {
            meta: &self.meta,
            obj: o,
        }
    }
    fn on_apply(&self, o: DynamicObject) -> Result<()> {
        info!(
            "pod apply  event: {:#?}/{:#?}",
            &o.metadata.namespace, &o.metadata.name
        );
        self.store.set(self.to_cluster_obj(&o))
    }
    fn on_delete(&self, o: DynamicObject) -> Result<()> {
        info!(
            "pod delete event: {:#?}/{:#?}",
            &o.metadata.namespace, &o.metadata.name
        );
        self.store.delete(self.to_cluster_obj(&o))
    }
    fn on_resync(&self, objs: Vec<DynamicObject>) -> Result<()> {
        for o in objs {
            self.on_apply(o)?;
        }
        Ok(())
    }
}

impl Controller for PodController {
    fn resync(&self) -> Result<()> {
        todo!()
    }
}

#[async_trait]
impl EventHandler for PodController {
    async fn process(&self, e: Event<DynamicObject>) -> Result<()> {
        match e {
            Applied(o) => self.on_apply(o),
            Deleted(o) => self.on_delete(o),
            Restarted(o) => self.on_resync(o),
        }
    }
}

#[derive(Clone, Copy)]
pub struct PodControllerFactory;

impl PodControllerFactory {
    pub fn new_box() -> Box<PodControllerFactory> {
        Box::new(PodControllerFactory)
    }
}

impl EventHandlerFactory for PodControllerFactory {
    fn build(
        &self,
        meta: ClusterObjectMeta,
        client: Client,
        store: Arc<Box<dyn Storage>>,
    ) -> Box<dyn EventHandler> {
        Box::new(PodController::new(client, meta, store))
    }
}

impl FactoryClone for PodControllerFactory {
    fn clone_box(&self) -> Box<dyn EventHandlerFactory> {
        Box::new(self.clone())
    }
}
