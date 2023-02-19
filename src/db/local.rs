use super::Storage;
use crate::db::utils::*;
use crate::fuse::core::{time_now, InodeAttributes};
use crate::{ClusterObject, Result};
use kube::core::DynamicObject;
use sled::Transactional;
use sled::{Db, IVec, Tree};
use std::{collections::HashMap, path::Path};
use Bucket::*;

#[derive(Clone, Hash, PartialEq, Eq)]
pub enum Bucket {
    RIndex,
    Inode,
    Dentry,
    Data,
}

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

    fn get_bucket(&self, name: Bucket) -> &Tree {
        self.buckets.get(&name).unwrap()
    }

    pub fn create_file(&self, cluster_obj: ClusterObject) -> Result<()> {
        let path = get_resource_full_key(&cluster_obj);
        let next_inode = handle_next_inode();
        let value: IVec = (&cluster_obj).try_into()?;
        let inode_attr: IVec = InodeAttributes::new(next_inode.0, value.len() as u64).try_into()?;
        (
            self.get_bucket(RIndex),
            self.get_bucket(Inode),
            self.get_bucket(Data),
        )
            .transaction(|(rindx, inode, data)| {
                rindx.insert(path.as_bytes(), next_inode.1.clone())?;
                inode.insert(next_inode.1.clone(), inode_attr.clone())?;
                data.insert(next_inode.1.clone(), value.clone())?;
                Ok(())
            })?;
        Ok(())
    }

    // TODO!: keep thread safety
    pub fn update_file(&self, cluster_obj: ClusterObject) -> Result<()> {
        let path = get_resource_full_key(&cluster_obj);
        let value: IVec = (&cluster_obj).try_into()?;
        let inode_ivec = self.get_bucket(Inode).get(path)?.unwrap();
        let inode_c: u64 = ivec_to_u64(inode_ivec.clone());
        let mut inode_attr: InodeAttributes = self
            .get_bucket(Inode)
            .get(u64_to_ivec(inode_c))?
            .unwrap()
            .try_into()
            .unwrap();
        inode_attr.last_modified = time_now();
        inode_attr.last_metadata_changed = time_now();
        let inode_attr_ivec: IVec = inode_attr.try_into()?;

        (self.get_bucket(Inode), self.get_bucket(Data)).transaction(|(inode, data)| {
            inode.insert(inode_ivec.clone(), inode_attr_ivec.clone())?;
            data.insert(inode_ivec.clone(), value.clone())?;
            Ok(())
        })?;

        Ok(())
    }

    pub fn is_file_exist(&self, path: &str) -> Result<bool> {
        let rindex = self.get_bucket(RIndex);
        let exist = rindex.contains_key(path)?;
        Ok(exist)
    }
}

impl Storage for SledDb {
    fn add(&self, cluster_obj: ClusterObject) -> Result<()> {
        if !self.is_file_exist(&get_resource_full_key(&cluster_obj))? {
            return self.update(cluster_obj);
        }
        self.create_file(cluster_obj)?;
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

    fn get_bucket(&self, name: Bucket) -> Option<&Tree> {
        self.buckets.get(&name)
    }
}
