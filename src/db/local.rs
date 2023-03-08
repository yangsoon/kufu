use super::Storage;
use crate::db::utils::*;
use crate::fuse::core::{time_now, DentryAttributes, InodeAttributes};
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
        let parent_inode = self.create_api_dict(&cluster_obj)?;
        let path = get_resource_full_key(&cluster_obj);
        let next_inode = handle_next_inode();
        let value: IVec = (&cluster_obj).try_into()?;
        let inode_attr: IVec = InodeAttributes::new_file(next_inode.0, value.len() as u64).into();
        let update_dentry_attr = |old: Option<&[u8]>| -> Option<DentryAttributes> {
            match old {
                Some(bytes) => {
                    let mut inode_attr: DentryAttributes = bytes.try_into().unwrap();
                    inode_attr.entries.push(next_inode.0);
                    Some(inode_attr)
                }
                None => Some(DentryAttributes {
                    entries: vec![next_inode.0],
                    parent: parent_inode,
                }),
            }
        };
        self.get_bucket(Dentry)
            .fetch_and_update(u64_to_ivec(parent_inode), update_dentry_attr)?;
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

    // TODO!: keep in mind thread safety
    pub fn update_file(&self, cluster_obj: ClusterObject) -> Result<()> {
        let path = get_resource_full_key(&cluster_obj);
        let value: IVec = (&cluster_obj).try_into()?;
        let inode_ivec = self.get_bucket(Inode).get(path)?.unwrap();

        fn update_inode_attr(old: Option<&[u8]>) -> Option<InodeAttributes> {
            match old {
                Some(bytes) => {
                    let mut inode_attr: InodeAttributes = bytes.try_into().unwrap();
                    inode_attr.last_modified = time_now();
                    inode_attr.last_metadata_changed = time_now();
                    Some(inode_attr)
                }
                None => None,
            }
        }
        self.get_bucket(Inode)
            .fetch_and_update(inode_ivec.clone(), update_inode_attr)?;
        self.get_bucket(Data)
            .insert(inode_ivec.clone(), value.clone())?;
        Ok(())
    }

    pub fn create_api_dict(&self, cluster_obj: &ClusterObject) -> Result<u64> {
        let api_path = get_resource_api_key(cluster_obj);
        if self.get_bucket(RIndex).contains_key(api_path.clone())? {
            let parent_inode = self.get_bucket(RIndex).get(api_path.clone())?.unwrap();
            return Ok(ivec_to_u64(parent_inode));
        }
        let next_inode = handle_next_inode();
        let inode_attr: IVec = InodeAttributes::new_dict(next_inode.0).into();
        (self.get_bucket(RIndex), self.get_bucket(Inode)).transaction(|(rindx, inode)| {
            rindx.insert(api_path.as_bytes(), next_inode.1.clone())?;
            inode.insert(next_inode.1.clone(), inode_attr.clone())?;
            Ok(())
        })?;
        Ok(next_inode.0)
    }
}

impl Storage for SledDb {
    fn add(&self, cluster_obj: ClusterObject) -> Result<()> {
        if !self.has(&cluster_obj)? {
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

    fn has(&self, cluster_obj: &ClusterObject) -> Result<bool> {
        let exist = self
            .get_bucket(RIndex)
            .contains_key(&get_resource_full_key(cluster_obj))?;
        Ok(exist)
    }
}
