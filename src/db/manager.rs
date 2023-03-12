use super::Bucket::*;
use super::{FSManger, SledDb, Storage};
use crate::db::utils::*;
use crate::error::Error::{ClusterObjectDataNotFound, DentryAttrNotFound, InodeAttrNotFound};
use crate::fuse::core::{time_now, DentryAttributes, FileKind, InodeAttributes};
use crate::Result;
use sled::IVec;
use sled::Transactional;
use std::path::Path;

impl FSManger for SledDb {
    fn mount_dir(&self, path: impl AsRef<Path>, parent_inode: u64) -> Result<u64> {
        let name = extract_name(path.as_ref());
        let key = into_string(path.as_ref());

        if self.get_bucket(RIndex).contains_key(key.clone())? {
            let inode = self.get_bucket(RIndex).get(key.clone())?.unwrap();
            return Ok(ivec_to_u64(&inode));
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
        let name = extract_name(path.as_ref());
        let key = into_string(path.as_ref());
        let next_inode = if self.get_bucket(RIndex).contains_key(key.clone())? {
            let inode = self.get_bucket(RIndex).get(key.clone())?.unwrap();
            (ivec_to_u64(&inode), inode)
        } else {
            handle_next_inode()
        };
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
        if parent_inode == 0 {
            return Ok(());
        }
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
            return Err(DentryAttrNotFound(inode));
        }
        let attr: DentryAttributes = dentry_bucket.get(inode_key)?.unwrap().try_into()?;
        Ok(attr)
    }

    fn get_inode_attr(&self, inode: u64) -> Result<InodeAttributes> {
        let inode_bucket = self.get_bucket(Inode);
        let inode_key = u64_to_ivec(inode);
        if !inode_bucket.contains_key(inode_key.clone())? {
            return Err(InodeAttrNotFound(inode));
        }
        let attr: InodeAttributes = inode_bucket.get(inode_key)?.unwrap().try_into()?;
        Ok(attr)
    }

    fn update_inode(&self, inode: u64, attr: InodeAttributes) -> Result<()> {
        let value: IVec = attr.try_into()?;
        self.get_bucket(Inode).insert(u64_to_ivec(inode), value)?;
        Ok(())
    }

    fn get_inode(&self, key: String) -> Result<u64> {
        let inode = self.get_bucket(RIndex).get(key)?.unwrap();
        Ok(ivec_to_u64(&inode))
    }

    fn get_data(&self, inode: u64) -> Result<IVec> {
        let data_bucket = self.get_bucket(Data);
        let obj_key = u64_to_ivec(inode);
        if !data_bucket.contains_key(&obj_key)? {
            return Err(ClusterObjectDataNotFound(inode));
        }
        let data = data_bucket.get(&obj_key)?.unwrap();
        return Ok(data);
    }
}
