use crate::db::Bucket;
use crate::{ClusterObject, Result, INODE_NUM};
use kube::discovery::Scope;
use sled::{Db, IVec};
use std::path::Path;
use std::sync::atomic::Ordering;

pub type SledInode = (u64, IVec);

pub fn u64_to_ivec(number: u64) -> IVec {
    IVec::from(number.to_be_bytes().to_vec())
}

pub fn ivec_to_u64(value: IVec) -> u64 {
    assert_eq!(value.len(), 8);
    let raw = &*value;
    u64::from_be_bytes(raw[0..8].try_into().unwrap())
}

pub fn handle_next_inode() -> SledInode {
    let inode_c = INODE_NUM.fetch_add(1, Ordering::SeqCst);
    (inode_c, u64_to_ivec(inode_c))
}

pub fn get_resource_full_key(cluster_obj: &ClusterObject) -> String {
    let kind = &cluster_obj.meta.gvk.kind;
    let name = cluster_obj.obj.metadata.name.as_ref().unwrap();
    let cluster = &cluster_obj.meta.cluster;
    let namespace = cluster_obj.obj.metadata.namespace.as_ref();
    match cluster_obj.meta.caps.scope {
        Scope::Namespaced => {
            format!(
                "{}/namespace/{}/{}/{}",
                cluster,
                namespace.unwrap(),
                kind,
                name
            )
        }
        Scope::Cluster => {
            format!("{}/{}/{}", cluster, kind, name)
        }
    }
}

pub fn get_resource_api_key(cluster_obj: &ClusterObject) -> String {
    let kind = &cluster_obj.meta.gvk.kind;
    let cluster = &cluster_obj.meta.cluster;
    match cluster_obj.meta.caps.scope {
        Scope::Namespaced => {
            let namespace = cluster_obj.obj.metadata.namespace.as_ref().unwrap();
            format!("{}/namespace/{}/{}", cluster, namespace, kind)
        }
        Scope::Cluster => {
            format!("{}/{}", cluster, kind)
        }
    }
}

pub fn get_parent_resource_full_key(cluster_obj: &ClusterObject) -> String {
    let cluster = &cluster_obj.meta.cluster;
    match cluster_obj.meta.caps.scope {
        Scope::Namespaced => {
            let namespace = cluster_obj.obj.metadata.namespace.as_ref().unwrap();
            format!("{}/namespace/{}", cluster, namespace)
        }
        Scope::Cluster => {
            format!("{}", cluster)
        }
    }
}

pub fn clean_one_time_buckets(db: &Db) -> Result<()> {
    db.drop_tree(Bucket::RIndex)?;
    db.drop_tree(Bucket::Inode)?;
    db.drop_tree(Bucket::Dentry)?;
    Ok(())
}

pub fn extract_name(path: &Path) -> String {
    let file_name = path.file_name().unwrap();
    file_name.to_os_string().into_string().unwrap()
}

pub fn into_string(path: &Path) -> String {
    path.to_str().unwrap().to_string()
}
