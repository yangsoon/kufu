use crate::error;
use serde::{Deserialize, Serialize};
use sled::IVec;
use std::{
    collections::{BTreeMap, HashMap},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

type Inode = u64;

#[derive(Serialize, Deserialize, Copy, Clone, PartialEq)]
pub enum FileKind {
    File,
    Directory,
    Symlink,
}

impl From<FileKind> for fuser::FileType {
    fn from(kind: FileKind) -> Self {
        match kind {
            FileKind::File => fuser::FileType::RegularFile,
            FileKind::Directory => fuser::FileType::Directory,
            FileKind::Symlink => fuser::FileType::Symlink,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct InodeAttributes {
    pub inode: Inode,
    pub open_file_handles: u64, // Ref count of open file handles to this inode
    pub size: u64,
    pub last_accessed: (i64, u32),
    pub last_modified: (i64, u32),
    pub last_metadata_changed: (i64, u32),
    pub kind: FileKind,
    // Permissions and special mode bits
    pub mode: u16,
    pub hardlinks: u32,
    pub uid: u32,
    pub gid: u32,
    pub xattrs: BTreeMap<Vec<u8>, Vec<u8>>,
}

impl InodeAttributes {
    pub fn new_file(inode: u64, size: u64) -> InodeAttributes {
        InodeAttributes {
            inode: inode,
            open_file_handles: 0,
            size: size,
            last_accessed: time_now(),
            last_modified: time_now(),
            last_metadata_changed: time_now(),
            kind: FileKind::File,
            mode: 0o777,
            hardlinks: 0,
            uid: 0,
            gid: 0,
            xattrs: Default::default(),
        }
    }

    pub fn new_dict(inode: u64) -> InodeAttributes {
        InodeAttributes {
            inode: inode,
            open_file_handles: 0,
            size: 0,
            last_accessed: time_now(),
            last_modified: time_now(),
            last_metadata_changed: time_now(),
            kind: FileKind::Directory,
            mode: 0o777,
            hardlinks: 0,
            uid: 0,
            gid: 0,
            xattrs: Default::default(),
        }
    }
}

impl TryFrom<IVec> for InodeAttributes {
    type Error = error::Error;
    fn try_from(value: IVec) -> Result<Self, Self::Error> {
        let inode_attr: InodeAttributes = serde_yaml::from_slice(&*value)?;
        Ok(inode_attr)
    }
}

impl TryFrom<&[u8]> for InodeAttributes {
    type Error = error::Error;
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let inode_attr: InodeAttributes = serde_yaml::from_slice(&*value)?;
        Ok(inode_attr)
    }
}

impl From<InodeAttributes> for IVec {
    fn from(value: InodeAttributes) -> Self {
        let obj_data = &*serde_yaml::to_string(&value).unwrap();
        obj_data.into()
    }
}

#[derive(Serialize, Deserialize)]
pub struct DentryAttributes {
    pub entries: Vec<u64>,
}

impl From<DentryAttributes> for IVec {
    fn from(value: DentryAttributes) -> Self {
        let obj_data = &*serde_yaml::to_string(&value).unwrap();
        obj_data.into()
    }
}

impl TryFrom<&[u8]> for DentryAttributes {
    type Error = error::Error;
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let dentry_attr: DentryAttributes = serde_yaml::from_slice(&*value)?;
        Ok(dentry_attr)
    }
}

pub fn time_now() -> (i64, u32) {
    time_from_system_time(&SystemTime::now())
}

fn system_time_from_time(secs: i64, nsecs: u32) -> SystemTime {
    if secs >= 0 {
        UNIX_EPOCH + Duration::new(secs as u64, nsecs)
    } else {
        UNIX_EPOCH - Duration::new((-secs) as u64, nsecs)
    }
}

fn time_from_system_time(system_time: &SystemTime) -> (i64, u32) {
    // Convert to signed 64-bit time with epoch at 0
    match system_time.duration_since(UNIX_EPOCH) {
        Ok(duration) => (duration.as_secs() as i64, duration.subsec_nanos()),
        Err(before_epoch_error) => (
            -(before_epoch_error.duration().as_secs() as i64),
            before_epoch_error.duration().subsec_nanos(),
        ),
    }
}
