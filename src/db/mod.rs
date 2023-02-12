pub mod local;
pub use local::*;

use crate::ClusterObject;
use crate::Result;
use kube::core::DynamicObject;
pub trait Storage: Sync + Send {
    fn set(&self, cluster_obj: ClusterObject) -> Result<()>;
    fn get(&self, cluster_obj: ClusterObject) -> Result<Option<DynamicObject>>;
    fn delete(&self, cluster_obj: ClusterObject) -> Result<()>;
}
