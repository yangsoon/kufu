use super::Bucket::*;
use super::{Bucket, FSManger, Storage};
use crate::db::utils::*;
use crate::error::Error::MockParentDirError;
use crate::{ClusterObject, Result};
use kube::core::DynamicObject;
use kube::discovery::Scope::*;
use sled::{Db, IVec, Tree};
use std::{collections::HashMap, path::Path};

impl AsRef<[u8]> for Bucket {
    fn as_ref(&self) -> &[u8] {
        match self {
            RIndex => "reverse-index".as_bytes(),
            Inode => "inode".as_bytes(),
            Dentry => "dentry".as_bytes(),
            Data => "data".as_bytes(),
        }
    }
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct SledDb {
    db: Db,
    buckets: HashMap<Bucket, Tree>,
}

#[allow(dead_code)]
impl SledDb {
    pub fn new(path: impl AsRef<Path>) -> Result<SledDb> {
        let db = sled::open(path)?;
        clean_one_time_buckets(&db)?;
        Ok(SledDb {
            db: db.clone(),
            buckets: HashMap::from([
                (Bucket::RIndex, db.open_tree(Bucket::RIndex)?),
                (Bucket::Inode, db.open_tree(Bucket::Inode)?),
                (Bucket::Dentry, db.open_tree(Bucket::Dentry)?),
                (Bucket::Data, db.open_tree(Bucket::Data)?),
            ]),
        })
    }

    pub fn mount_gvr(&self, cluster_obj: &ClusterObject) -> Result<u64> {
        let parent_inode = self.mount_gvk(cluster_obj)?;
        let key = get_resource_full_key(cluster_obj);
        let value: IVec = (cluster_obj).try_into()?;
        let file_key = format!("{}.yaml", &key);
        match cluster_obj.scope() {
            Namespaced => self.mount_file(&file_key, parent_inode, value),
            Cluster => {
                self.mount_dir(&key, parent_inode)?;
                self.mount_file(&file_key, parent_inode, value)
            }
        }
    }

    pub fn update_gvr(&self, cluster_obj: ClusterObject) -> Result<()> {
        let key = get_resource_full_key(&cluster_obj);
        let value: IVec = (&cluster_obj).try_into()?;
        self.edit_file(key, value)
    }

    pub fn mount_gvk(&self, cluster_obj: &ClusterObject) -> Result<u64> {
        let api_path = get_resource_api_key(cluster_obj);
        let parent_path = get_parent_resource_full_key(cluster_obj);
        let parent_inode = if self.get_bucket(RIndex).contains_key(parent_path.clone())? {
            let p_inode = self
                .get_bucket(RIndex)
                .get(get_parent_resource_full_key(cluster_obj))?
                .unwrap();
            ivec_to_u64(&p_inode)
        } else {
            match cluster_obj.scope() {
                Namespaced => {
                    // TODO: replace default to cluster
                    self.mount_dir(
                        parent_path.clone(),
                        self.get_inode("default/namespace".to_string())?,
                    )?
                }
                Cluster => return Err(MockParentDirError(api_path.clone())),
            }
        };
        self.mount_dir(api_path, parent_inode)
    }
}

impl Storage for SledDb {
    fn add(&self, cluster_obj: ClusterObject) -> Result<()> {
        if self.has(&cluster_obj)? {
            return self.update(cluster_obj);
        }
        self.mount_gvr(&cluster_obj)?;
        Ok(())
    }

    fn update(&self, cluster_obj: ClusterObject) -> Result<()> {
        let key = get_resource_full_key(&cluster_obj);
        let value: IVec = (&cluster_obj).try_into()?;
        self.get_bucket(Data).insert(key, value)?;
        Ok(())
    }

    fn get(&self, cluster_obj: ClusterObject) -> Result<Option<DynamicObject>> {
        let key = get_resource_full_key(&cluster_obj);
        let value = self.buckets.get(&Data).unwrap().get(key)?;
        match value {
            Some(v) => Ok(Some(serde_yaml::from_slice(v.as_ref())?)),
            None => Ok(None),
        }
    }

    fn delete(&self, cluster_obj: ClusterObject) -> Result<()> {
        let key = get_resource_full_key(&cluster_obj);
        self.get_bucket(Data).remove(key)?;
        Ok(())
    }

    fn get_bucket(&self, name: Bucket) -> &Tree {
        self.buckets.get(&name).unwrap()
    }

    fn has(&self, cluster_obj: &ClusterObject) -> Result<bool> {
        let exist = self
            .get_bucket(RIndex)
            .contains_key(&get_resource_full_key(cluster_obj))?;
        Ok(exist)
    }
}
