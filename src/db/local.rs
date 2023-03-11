use super::{FSManger, Storage};
use crate::db::utils::*;
use crate::error::Error::{DentryAttrNotFount, InodeAttrNotFount};
use crate::fuse::core::{time_now, DentryAttributes, FileKind, InodeAttributes};
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

    pub fn mount_gvr(&self, cluster_obj: ClusterObject) -> Result<u64> {
        let parent_inode = self.mount_gvk(&cluster_obj)?;
        let key = get_resource_full_key(&cluster_obj);
        let value: IVec = (&cluster_obj).try_into()?;
        self.mount_file(key, parent_inode, value)
    }

    pub fn update_gvr(&self, cluster_obj: ClusterObject) -> Result<()> {
        let key = get_resource_full_key(&cluster_obj);
        let value: IVec = (&cluster_obj).try_into()?;
        self.edit_file(key, value)
    }

    pub fn mount_gvk(&self, cluster_obj: &ClusterObject) -> Result<u64> {
        let api_path = get_resource_api_key(cluster_obj);
        let parent_inode = ivec_to_u64(
            self.get_bucket(Inode)
                .get(get_parent_resource_full_key(cluster_obj))?
                .unwrap(),
        );
        self.mount_dir(api_path, parent_inode)
    }
}

impl Storage for SledDb {
    fn add(&self, cluster_obj: ClusterObject) -> Result<()> {
        if !self.has(&cluster_obj)? {
            return self.update(cluster_obj);
        }
        self.mount_gvr(cluster_obj)?;
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

impl FSManger for SledDb {
    fn mount_dir(&self, path: impl AsRef<Path>, parent_inode: u64) -> Result<u64> {
        let name = extract_name(path.as_ref());
        let key = into_string(path.as_ref());
        if self.get_bucket(RIndex).contains_key(key.clone())? {
            let inode = self.get_bucket(RIndex).get(key.clone())?.unwrap();
            return Ok(ivec_to_u64(inode));
        }
        let next_inode = handle_next_inode();
        let inode_attr: IVec = InodeAttributes::new_dict(next_inode.0).into();
        let entries = if parent_inode != 0 {
            vec![
                (".".to_string(), (FileKind::Directory, next_inode.0)),
                ("..".to_string(), (FileKind::Directory, parent_inode)),
            ]
        } else {
            vec![(".".to_string(), (FileKind::Directory, next_inode.0))]
        };
        let dentry_attr: IVec = DentryAttributes {
            parent: parent_inode,
            name: name.clone(),
            entries: entries.into_iter().collect(),
        }
        .into();
        self.join_dir(parent_inode, next_inode.0, name, FileKind::Directory)?;
        (
            self.get_bucket(RIndex),
            self.get_bucket(Inode),
            self.get_bucket(Dentry),
        )
            .transaction(|(rindx, inode, dentry)| {
                rindx.insert(key.as_bytes(), next_inode.1.clone())?;
                inode.insert(next_inode.1.clone(), inode_attr.clone())?;
                dentry.insert(next_inode.1.clone(), dentry_attr.clone())?;
                Ok(())
            })?;
        Ok(next_inode.0)
    }

    fn mount_file(&self, path: impl AsRef<Path>, parent_inode: u64, content: IVec) -> Result<u64> {
        let next_inode = handle_next_inode();
        let name = extract_name(path.as_ref());
        let key = into_string(path.as_ref());
        let inode_attr: IVec = InodeAttributes::new_file(next_inode.0, content.len() as u64).into();
        self.join_dir(parent_inode, next_inode.0, name, FileKind::File)?;
        (
            self.get_bucket(RIndex),
            self.get_bucket(Inode),
            self.get_bucket(Data),
        )
            .transaction(|(rindx, inode, data)| {
                rindx.insert(key.as_bytes(), next_inode.1.clone())?;
                inode.insert(next_inode.1.clone(), inode_attr.clone())?;
                data.insert(next_inode.1.clone(), content.clone())?;
                Ok(())
            })?;
        Ok(next_inode.0)
    }

    fn edit_file(&self, path: impl AsRef<Path>, content: IVec) -> Result<()> {
        let key = into_string(path.as_ref());
        let inode_ivec = self.get_bucket(Inode).get(key)?.unwrap();
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
            .insert(inode_ivec.clone(), content.clone())?;
        Ok(())
    }

    fn join_dir(&self, parent_inode: u64, inode: u64, name: String, kind: FileKind) -> Result<()> {
        let update_dentry_attr = |old: Option<&[u8]>| -> Option<DentryAttributes> {
            match old {
                Some(bytes) => {
                    let mut inode_attr: DentryAttributes = bytes.try_into().unwrap();
                    inode_attr.entries.insert(name.clone(), (kind, inode));
                    Some(inode_attr)
                }
                None => None,
            }
        };
        self.get_bucket(Dentry)
            .fetch_and_update(u64_to_ivec(parent_inode), update_dentry_attr)?;
        Ok(())
    }

    fn get_dentry(&self, inode: u64) -> Result<DentryAttributes> {
        let dentry_bucket = self.get_bucket(Dentry);
        let inode_key = u64_to_ivec(inode);
        if !dentry_bucket.contains_key(inode_key.clone())? {
            return Err(DentryAttrNotFount(inode));
        }
        let attr: DentryAttributes = dentry_bucket.get(inode_key)?.unwrap().try_into()?;
        Ok(attr)
    }

    fn get_inode(&self, inode: u64) -> Result<InodeAttributes> {
        let inode_bucket = self.get_bucket(Inode);
        let inode_key = u64_to_ivec(inode);
        if !inode_bucket.contains_key(inode_key.clone())? {
            return Err(InodeAttrNotFount(inode));
        }
        let attr: InodeAttributes = inode_bucket.get(inode_key)?.unwrap().try_into()?;
        Ok(attr)
    }
}
