#[macro_use]
extern crate lazy_static;
use std::collections::HashMap;
use tokio::sync::Mutex;

use ::kube::{
    core::DynamicObject, core::GroupVersionKind, discovery::ApiCapabilities, runtime::watcher,
};

pub mod args;
pub mod config;
pub mod controller;
pub mod error;
pub mod kube;
pub mod utils;

pub type Result<T> = std::result::Result<T, error::Error>;

pub trait EventHandlerFactory: FactoryClone + Send + Sync {
    fn build(&self, caps: ApiCapabilities) -> Box<dyn EventHandler + Send>;
}

pub trait FactoryClone {
    fn clone_box(&self) -> Box<dyn EventHandlerFactory + Send>;
}

pub trait EventHandler {
    fn process(&self, e: watcher::Event<DynamicObject>) -> Result<()>;
}

lazy_static! {
    pub static ref SCHEMA: Mutex<HashMap<GroupVersionKind, Box<dyn EventHandlerFactory>>> =
        Mutex::new(HashMap::new());
}

pub async fn register(gvk: GroupVersionKind, f: Box<dyn EventHandlerFactory>) {
    SCHEMA.lock().await.insert(gvk, f);
}
