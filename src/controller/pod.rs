use super::Controller;
use crate::{db::Storage, EventHandler, EventHandlerFactory, FactoryClone, Result};
use kube::{
    core::DynamicObject,
    discovery::ApiCapabilities,
    runtime::watcher::Event,
    runtime::watcher::Event::{Applied, Deleted, Restarted},
    Client,
};
use std::sync::Arc;

#[warn(dead_code)]
struct PodController {
    client: Client,
    caps: ApiCapabilities,
    store: Arc<Box<dyn Storage>>,
}

impl PodController {
    fn new(client: Client, caps: ApiCapabilities, store: Arc<Box<dyn Storage>>) -> PodController {
        PodController {
            client,
            caps,
            store,
        }
    }

    fn on_add(&self, store: &Box<dyn Storage>, o: DynamicObject) -> Result<()> {
        println!("add: {}", o.metadata.name.unwrap());
        Ok(())
    }
    fn on_delete(&self, store: &Box<dyn Storage>, o: DynamicObject) -> Result<()> {
        println!("delete: {}", o.metadata.name.unwrap());
        Ok(())
    }
    fn on_resync(&self, store: &Box<dyn Storage>, objs: Vec<DynamicObject>) -> Result<()> {
        println!("sync: {:?}", objs);
        Ok(())
    }
}

impl Controller for PodController {
    fn resync(&self) -> Result<()> {
        todo!()
    }
}

impl EventHandler for PodController {
    fn process(&self, e: Event<DynamicObject>) -> Result<()> {
        match e {
            Applied(o) => self.on_add(self.store.as_ref(), o),
            Deleted(o) => self.on_delete(self.store.as_ref(), o),
            Restarted(o) => self.on_resync(self.store.as_ref(), o),
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
        caps: ApiCapabilities,
        client: Client,
        store: Arc<Box<dyn Storage>>,
    ) -> Box<dyn EventHandler> {
        Box::new(PodController::new(client, caps, store))
    }
}

impl FactoryClone for PodControllerFactory {
    fn clone_box(&self) -> Box<dyn EventHandlerFactory> {
        Box::new(self.clone())
    }
}
