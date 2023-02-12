pub mod local;
pub use local::*;

use crate::Result;
use crate::{config::Mount, ClusterObject};
use kube::core::DynamicObject;

pub struct StoreOption {
    mount: Option<Mount>,
}

pub trait Storage: Sync + Send {
    fn set(&self, cluster_obj: &ClusterObject) -> Result<()>;
    fn get(&self, cluster_obj: &ClusterObject) -> Result<Option<DynamicObject>>;
    fn delete(&self, cluster_obj: &ClusterObject) -> Result<()>;
}
