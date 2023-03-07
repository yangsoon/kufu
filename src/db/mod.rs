pub mod local;
pub mod utils;
pub use local::*;
use sled::Tree;

use crate::ClusterObject;
use crate::Result;
use kube::core::DynamicObject;
pub trait Storage: Sync + Send {
    fn add(&self, cluster_obj: ClusterObject) -> Result<()>;
    fn get(&self, cluster_obj: ClusterObject) -> Result<Option<DynamicObject>>;
    fn update(&self, cluster_obj: ClusterObject) -> Result<()>;
    fn delete(&self, cluster_obj: ClusterObject) -> Result<()>;
    fn get_bucket(&self, name: Bucket) -> Option<&Tree>;
    fn has(&self, cluster_obj: &ClusterObject) -> Result<bool>;
}
