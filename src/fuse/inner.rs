use crate::db::{FSManger, SledDb};
use crate::error::Error::ChildEntryNotFound;
use crate::{Result, FILE_HANDLE_NUM};
use fuser::{FileAttr, ReplyDirectory};
use std::cmp::min;
use std::ffi::{OsStr, OsString};
use std::str::FromStr;
use tracing::info;

const FILE_HANDLE_READ_BIT: u64 = 1 << 63;
const FILE_HANDLE_WRITE_BIT: u64 = 1 << 62;

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

    pub fn look_up(&self, parent: u64, name: &OsStr) -> Result<FileAttr> {
        let name = name.to_str().unwrap();
        let dentry = self.store.get_dentry(parent)?;
        let entry = dentry.entries.get(name);
        let inode = match entry {
            Some(item) => item.1,
            None => return Err(ChildEntryNotFound(dentry.name, name.to_string())),
        };
        return self.get_attr(inode);
    }

    pub fn get_attr(&self, inode: u64) -> Result<FileAttr> {
        let attr = self.store.get_inode_attr(inode)?;
        Ok(attr.into())
    }

    pub fn read_dir(&self, inode: u64, offset: i64, reply: &mut ReplyDirectory) -> Result<()> {
        let dentry = self.store.get_dentry(inode)?;
        info!("success call read_dir, dir: {:?}", dentry);
        for (index, entry) in dentry.entries.iter().skip(offset as usize).enumerate() {
            let (name, (kind, inode)) = entry;
            info!(
                "reply add offset:{:?}, name: {:?} kind: {:?}",
                offset,
                OsString::from_str(name)?.as_os_str(),
                kind,
            );
            if reply.add(
                *inode,
                offset + index as i64 + 1,
                (*kind).into(),
                OsString::from_str(name)?.as_os_str(),
            ) {
                break;
            }
        }
        Ok(())
    }

    pub fn open_dir(&self, inode: u64, read: bool, write: bool) -> Result<u64> {
        let mut inode_attr = self.store.get_inode_attr(inode)?;
        inode_attr.open_file_handles += 1;
        self.store.update_inode(inode, inode_attr)?;
        let mut fh = next_file_handle();
        if read {
            fh |= FILE_HANDLE_READ_BIT;
        }
        if write {
            fh |= FILE_HANDLE_WRITE_BIT;
        }
        Ok(fh)
    }

    pub fn read(&self, inode: u64, offset: i64, size: u32) -> Result<Vec<u8>> {
        info!("read inode: {}, offset :{} size: {}", inode, offset, size);
        let ivec = self.store.get_data(inode)?;
        let data = &*ivec;
        let read_size = min(size, data.len().saturating_sub(offset as usize) as u32);
        let start = offset as usize;
        let end = start + read_size as usize;
        let buffer = &data[start..end];
        Ok(buffer.to_vec())
    }
}

fn next_file_handle() -> u64 {
    FILE_HANDLE_NUM.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
}
