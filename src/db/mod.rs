use kube::core::DynamicObject;

use crate::{config::Mount, ClusterObject};
pub mod local;
use crate::Result;

pub struct StoreOption {
    mouth: Option<Mount>,
}

pub trait Store {
    fn set(&self, cluster_obj: &ClusterObject) -> Result<()>;
    fn get(&self, cluster_obj: &ClusterObject) -> Result<Option<DynamicObject>>;
}
