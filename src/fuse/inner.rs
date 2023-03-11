use crate::db::{FSManger, SledDb};
use crate::Result;
use fuser::{FileAttr, ReplyDirectory};
use tracing::*;

pub struct FsInner {
    pub store: SledDb,
}

impl FsInner {
    pub fn new(store: SledDb) -> FsInner {
        FsInner { store }
    }

    pub fn init(&self, mount_point: String, cluster: Vec<String>) -> Result<()> {
        let mount_point_inode = self.store.mount_dir(mount_point.clone(), 0)?;
        self.store.mount_dir("default", mount_point_inode)?;
        Ok(())
    }

    pub fn get_attr(&self, inode: u64) -> Result<FileAttr> {
        let attr = self.store.get_inode(inode)?;
        Ok(attr.into())
    }

    pub fn read_dir(&self, inode: u64, offset: i64, reply: &mut ReplyDirectory) -> Result<()> {
        let dentry = self.store.get_dentry(inode)?;
        info!("read dir: {:?}", dentry);
        for (_, entry) in dentry.entries.iter().skip(offset as usize).enumerate() {
            let (name, (kind, inode)) = entry;
            if reply.add(*inode, offset, (*kind).into(), name) {
                return Ok(());
            }
        }
        Ok(())
    }
}
