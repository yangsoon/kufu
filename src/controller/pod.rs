use super::Controller;
use crate::{EventHandler, EventHandlerFactory, FactoryClone, Result};
use kube::discovery::ApiCapabilities;
use kube::runtime::watcher::Event;
use kube::Client;
use kube_core::DynamicObject;

#[warn(dead_code)]
struct PodController {
    client: Client,
    caps: ApiCapabilities,
}

impl PodController {
    fn new(client: Client, caps: ApiCapabilities) -> PodController {
        PodController { client, caps }
    }
}

impl Controller for PodController {
    fn resync(&self) -> Result<()> {
        todo!()
    }
}

impl EventHandler for PodController {
    fn process(&self, e: Event<DynamicObject>) -> Result<()> {
        println!("{:?}", e);
        Ok(())
    }
}

#[derive(Clone, Copy)]
pub struct PodControllerFactory {}

impl PodControllerFactory {
    pub fn new_box() -> Box<PodControllerFactory> {
        Box::new(PodControllerFactory {})
    }
}

impl EventHandlerFactory for PodControllerFactory {
    fn build(&self, caps: ApiCapabilities, client: Client) -> Box<dyn EventHandler> {
        Box::new(PodController::new(client, caps))
    }
}

impl FactoryClone for PodControllerFactory {
    fn clone_box(&self) -> Box<dyn EventHandlerFactory> {
        Box::new(self.clone())
    }
}
