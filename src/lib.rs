#[macro_use]
extern crate lazy_static;
use ::kube::{
    core::DynamicObject, core::GroupVersionKind, discovery::ApiCapabilities, runtime::watcher,
    Client,
};
use controller::PodControllerFactory;
use std::collections::HashMap;
use std::sync::Mutex;

pub mod args;
pub mod config;
pub mod controller;
pub mod error;
pub mod kube;
pub mod utils;

pub type Result<T> = std::result::Result<T, error::Error>;

pub trait EventHandlerFactory: FactoryClone + Send + Sync {
    fn build(&self, caps: ApiCapabilities, client: Client) -> Box<dyn EventHandler>;
}

pub trait FactoryClone {
    fn clone_box(&self) -> Box<dyn EventHandlerFactory>;
}

pub trait EventHandler: Send {
    fn process(&self, e: watcher::Event<DynamicObject>) -> Result<()>;
}

lazy_static! {
    pub static ref SCHEMA: Mutex<HashMap<GroupVersionKind, Box<dyn EventHandlerFactory>>> = {
        let mut schema = HashMap::new();
        register(
            &mut schema,
            GroupVersionKind::gvk("", "v1", "Pod"),
            PodControllerFactory::new_box(),
        );
        // TODO: add more controller
        Mutex::new(schema)
    };
}

pub fn register(
    schema: &mut HashMap<GroupVersionKind, Box<dyn EventHandlerFactory>>,
    gvk: GroupVersionKind,
    f: Box<dyn EventHandlerFactory>,
) {
    schema.insert(gvk, f);
}
