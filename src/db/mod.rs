pub mod manager;
pub mod storage;
pub mod utils;
pub use storage::*;

use crate::fuse::core::DentryAttributes;
use crate::fuse::core::FileKind;
use crate::fuse::core::InodeAttributes;
use crate::ClusterObject;
use crate::Result;
use kube::core::DynamicObject;
use sled::{IVec, Tree};
use std::path::Path;

#[derive(Clone, Hash, PartialEq, Eq)]
pub enum Bucket {
    RIndex,
    Inode,
    Dentry,
    Data,
}

pub trait Storage: Sync + Send {
    fn add(&self, cluster_obj: ClusterObject) -> Result<()>;
    fn get(&self, cluster_obj: ClusterObject) -> Result<Option<DynamicObject>>;
    fn update(&self, cluster_obj: ClusterObject) -> Result<()>;
    fn delete(&self, cluster_obj: ClusterObject) -> Result<()>;
    fn get_bucket(&self, name: Bucket) -> &Tree;
    fn has(&self, cluster_obj: &ClusterObject) -> Result<bool>;
}

pub trait FSManger: Sync + Send {
    fn mount_dir(&self, path: impl AsRef<Path>, parent_inode: u64) -> Result<u64>;
    fn mount_file(&self, path: impl AsRef<Path>, parent_inode: u64, content: IVec) -> Result<u64>;
    fn edit_file(&self, path: impl AsRef<Path>, content: IVec) -> Result<()>;
    fn join_dir(&self, parent_inode: u64, inode: u64, name: String, kind: FileKind) -> Result<()>;
    fn get_dentry(&self, inode: u64) -> Result<DentryAttributes>;
    fn get_inode_attr(&self, inode: u64) -> Result<InodeAttributes>;
    fn update_inode(&self, inode: u64, attr: InodeAttributes) -> Result<()>;
    fn get_inode(&self, key: String) -> Result<u64>;
    fn get_data(&self, inode: u64) -> Result<IVec>;
}
