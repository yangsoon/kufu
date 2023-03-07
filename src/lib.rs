pub mod args;
pub mod config;
pub mod controller;
pub mod db;
pub mod error;
pub mod fuse;
pub mod kube;

#[macro_use]
extern crate lazy_static;
use ::kube::{
    core::DynamicObject, core::GroupVersionKind, discovery::ApiCapabilities, runtime::watcher,
    Client,
};
use async_trait::async_trait;
use controller::{NamespaceControllerFactory, PodControllerFactory};
use db::Storage;
use sled::IVec;
use std::collections::HashMap;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};

pub type Result<T> = std::result::Result<T, error::Error>;

pub struct ClusterObject<'a> {
    pub meta: &'a ClusterObjectMeta,
    pub obj: &'a DynamicObject,
}

#[derive(Clone)]
pub struct ClusterObjectMeta {
    pub cluster: String,
    pub gvk: GroupVersionKind,
    pub caps: ApiCapabilities,
}

impl ClusterObjectMeta {
    pub fn new(cluster: String, gvk: GroupVersionKind, caps: ApiCapabilities) -> ClusterObjectMeta {
        ClusterObjectMeta { cluster, gvk, caps }
    }
}

impl<'a> TryFrom<&ClusterObject<'a>> for IVec {
    type Error = error::Error;

    fn try_from(value: &ClusterObject<'a>) -> std::result::Result<Self, Self::Error> {
        let obj_data = &*serde_yaml::to_string(value.obj)?;
        Ok(obj_data.into())
    }
}

pub trait EventHandlerFactory: FactoryClone + Send + Sync {
    fn build(
        &self,
        meta: ClusterObjectMeta,
        client: Client,
        store: Arc<Box<dyn Storage>>,
    ) -> Box<dyn EventHandler>;
}

pub trait FactoryClone {
    fn clone_box(&self) -> Box<dyn EventHandlerFactory>;
}

#[async_trait]
pub trait EventHandler: Send + Sync {
    async fn process(&self, e: watcher::Event<DynamicObject>) -> Result<()>;
}

lazy_static! {
    pub static ref INODE_NUM: AtomicU64 = AtomicU64::new(0);
    pub static ref SCHEMA: Mutex<HashMap<GroupVersionKind, Box<dyn EventHandlerFactory>>> = {
        let mut schema = HashMap::new();
        register(
            &mut schema,
            GroupVersionKind::gvk("", "v1", "Pod"),
            PodControllerFactory::new_box(),
        );

        register(
            &mut schema,
            GroupVersionKind::gvk("", "v1", "Namespace"),
            NamespaceControllerFactory::new_box(),
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
