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
struct NamespaceController {
    store: Arc<Box<dyn Storage>>,
    meta: ClusterObjectMeta,
}

impl NamespaceController {
    fn new(meta: ClusterObjectMeta, store: Arc<Box<dyn Storage>>) -> NamespaceController {
        NamespaceController { meta, store }
    }
    fn to_cluster_obj<'a>(&'a self, o: &'a DynamicObject) -> ClusterObject {
        ClusterObject {
            meta: &self.meta,
            obj: o,
        }
    }
    fn on_apply(&self, o: DynamicObject) -> Result<()> {
        info!(
            "namespace apply event: {:#?}/{:#?}",
            &o.metadata.namespace, &o.metadata.name
        );
        self.store.add(self.to_cluster_obj(&o))
    }
    fn on_delete(&self, o: DynamicObject) -> Result<()> {
        info!(
            "namespace delete event: {:#?}/{:#?}",
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

impl Controller for NamespaceController {
    fn resync(&self) -> Result<()> {
        todo!()
    }
}

#[async_trait]
impl EventHandler for NamespaceController {
    async fn process(&self, e: Event<DynamicObject>) -> Result<()> {
        match e {
            Applied(o) => self.on_apply(o),
            Deleted(o) => self.on_delete(o),
            Restarted(o) => self.on_resync(o),
        }
    }
}

#[derive(Clone, Copy)]
pub struct NamespaceControllerFactory;

impl NamespaceControllerFactory {
    pub fn new_box() -> Box<NamespaceControllerFactory> {
        Box::new(NamespaceControllerFactory)
    }
}

impl EventHandlerFactory for NamespaceControllerFactory {
    fn build(
        &self,
        meta: ClusterObjectMeta,
        client: Client,
        store: Arc<Box<dyn Storage>>,
    ) -> Box<dyn EventHandler> {
        Box::new(NamespaceController::new(meta, store))
    }
}

impl FactoryClone for NamespaceControllerFactory {
    fn clone_box(&self) -> Box<dyn EventHandlerFactory> {
        Box::new(self.clone())
    }
}
