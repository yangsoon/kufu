pub mod core;
pub mod inner;
use fuser::consts::FOPEN_DIRECT_IO;
pub use inner::*;

use fuser::Filesystem;
use fuser::{
    KernelConfig, ReplyAttr, ReplyBmap, ReplyCreate, ReplyData, ReplyDirectory, ReplyDirectoryPlus,
    ReplyEmpty, ReplyEntry, ReplyIoctl, ReplyLock, ReplyLseek, ReplyOpen, ReplyStatfs, ReplyWrite,
    ReplyXTimes, ReplyXattr, Request, TimeOrNow,
};
use kube::Client;
use libc::{ENOSYS, EPERM};
use std::ffi::OsStr;
use std::os::raw::c_int;
use std::path::Path;
use std::time::{Duration, SystemTime};
use tracing::*;

use crate::db::SledDb;
use crate::Result as KufuResult;

pub struct Fs {
    pub inner: inner::FsInner,
    pub client: Client,
    pub mount_point: String,
}

impl Fs {
    pub fn new(client: Client, store: SledDb, mount_point: String) -> Fs {
        Fs {
            inner: FsInner::new(store),
            client,
            mount_point,
        }
    }

    pub fn init(&self) -> KufuResult<()> {
        self.inner.init(self.mount_point.clone(), vec![])
    }
}

impl Filesystem for Fs {
    fn init(
        &mut self,
        _req: &Request<'_>,
        #[allow(unused_variables)] _config: &mut KernelConfig,
    ) -> Result<(), c_int> {
        Ok(())
    }

    fn destroy(&mut self) {}

    fn lookup(&mut self, _req: &Request<'_>, parent: u64, name: &OsStr, reply: ReplyEntry) {
        match self.inner.look_up(parent, name) {
            Ok(attr) => {
                debug!("look up file: attr: {:?}", attr);
                reply.entry(&Duration::new(0, 0), &attr, 0)
            }
            Err(e) => {
                if !e.to_string().contains("._.") {
                    error!("fail to lookup name: {:?} in parent err: {:?}", name, e);
                }
                reply.error(libc::ENOENT)
            }
        }
    }

    fn forget(&mut self, _req: &Request<'_>, _ino: u64, _nlookup: u64) {}

    fn getattr(&mut self, _req: &Request<'_>, ino: u64, reply: ReplyAttr) {
        match self.inner.get_attr(ino) {
            Ok(attr) => reply.attr(&Duration::new(0, 0), &attr),
            Err(e) => {
                error!("fail to get attr err: {:?}", e);
                reply.error(libc::ENOENT)
            }
        }
    }

    fn setattr(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        mode: Option<u32>,
        uid: Option<u32>,
        gid: Option<u32>,
        size: Option<u64>,
        _atime: Option<TimeOrNow>,
        _mtime: Option<TimeOrNow>,
        _ctime: Option<SystemTime>,
        fh: Option<u64>,
        _crtime: Option<SystemTime>,
        _chgtime: Option<SystemTime>,
        _bkuptime: Option<SystemTime>,
        flags: Option<u32>,
        reply: ReplyAttr,
    ) {
        debug!(
            "[Not Implemented] setattr(ino: {:#x?}, mode: {:?}, uid: {:?}, \
            gid: {:?}, size: {:?}, fh: {:?}, flags: {:?})",
            ino, mode, uid, gid, size, fh, flags
        );
        reply.error(ENOSYS);
    }

    fn readlink(&mut self, _req: &Request<'_>, ino: u64, reply: ReplyData) {
        debug!("[Not Implemented] readlink(ino: {:#x?})", ino);
        reply.error(ENOSYS);
    }

    fn mknod(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        mode: u32,
        umask: u32,
        rdev: u32,
        reply: ReplyEntry,
    ) {
        debug!(
            "[Not Implemented] mknod(parent: {:#x?}, name: {:?}, mode: {}, \
            umask: {:#x?}, rdev: {})",
            parent, name, mode, umask, rdev
        );
        reply.error(ENOSYS);
    }

    fn mkdir(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        mode: u32,
        umask: u32,
        reply: ReplyEntry,
    ) {
        debug!(
            "[Not Implemented] mkdir(parent: {:#x?}, name: {:?}, mode: {}, umask: {:#x?})",
            parent, name, mode, umask
        );
        reply.error(ENOSYS);
    }

    fn unlink(&mut self, _req: &Request<'_>, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        debug!(
            "[Not Implemented] unlink(parent: {:#x?}, name: {:?})",
            parent, name,
        );
        reply.error(ENOSYS);
    }

    fn rmdir(&mut self, _req: &Request<'_>, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        debug!(
            "[Not Implemented] rmdir(parent: {:#x?}, name: {:?})",
            parent, name,
        );
        reply.error(ENOSYS);
    }

    fn symlink(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        link: &Path,
        reply: ReplyEntry,
    ) {
        debug!(
            "[Not Implemented] symlink(parent: {:#x?}, name: {:?}, link: {:?})",
            parent, name, link,
        );
        reply.error(EPERM);
    }

    fn rename(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        newparent: u64,
        newname: &OsStr,
        flags: u32,
        reply: ReplyEmpty,
    ) {
        debug!(
            "[Not Implemented] rename(parent: {:#x?}, name: {:?}, newparent: {:#x?}, \
            newname: {:?}, flags: {})",
            parent, name, newparent, newname, flags,
        );
        reply.error(ENOSYS);
    }

    fn link(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        newparent: u64,
        newname: &OsStr,
        reply: ReplyEntry,
    ) {
        debug!(
            "[Not Implemented] link(ino: {:#x?}, newparent: {:#x?}, newname: {:?})",
            ino, newparent, newname
        );
        reply.error(EPERM);
    }

    fn open(&mut self, _req: &Request<'_>, ino: u64, flags: i32, reply: ReplyOpen) {
        debug!("call open inode: {:?}", ino);
        let (_, read, write) = match flags & libc::O_ACCMODE {
            libc::O_RDONLY => {
                // Behavior is undefined, but most filesystems return EACCES
                if flags & libc::O_TRUNC != 0 {
                    reply.error(libc::EACCES);
                    return;
                }
                (libc::R_OK, true, false)
            }
            libc::O_WRONLY => (libc::W_OK, false, true),
            libc::O_RDWR => (libc::R_OK | libc::W_OK, true, true),
            // Exactly one access mode flag must be specified
            _ => {
                reply.error(libc::EINVAL);
                return;
            }
        };
        match self.inner.open_dir(ino, read, write) {
            Ok(fh) => reply.opened(fh, FOPEN_DIRECT_IO),
            Err(e) => {
                error!("fail to open dir, err: {:?}", e);
                reply.error(ENOSYS);
            }
        }
    }

    fn read(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        fh: u64,
        offset: i64,
        size: u32,
        flags: i32,
        lock_owner: Option<u64>,
        mut reply: ReplyData,
    ) {
        match self.inner.read(ino, offset, size) {
            Ok(data) => reply.data(&data),
            Err(e) => {
                error!("fail to read err: {:?}", e);
                reply.error(ENOSYS);
            }
        }
    }

    fn write(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        fh: u64,
        offset: i64,
        data: &[u8],
        write_flags: u32,
        flags: i32,
        lock_owner: Option<u64>,
        reply: ReplyWrite,
    ) {
        debug!(
            "[Not Implemented] write(ino: {:#x?}, fh: {}, offset: {}, data.len(): {}, \
            write_flags: {:#x?}, flags: {:#x?}, lock_owner: {:?})",
            ino,
            fh,
            offset,
            data.len(),
            write_flags,
            flags,
            lock_owner
        );
        reply.error(ENOSYS);
    }

    fn flush(&mut self, _req: &Request<'_>, ino: u64, fh: u64, lock_owner: u64, reply: ReplyEmpty) {
        debug!(
            "[Not Implemented] flush(ino: {:#x?}, fh: {}, lock_owner: {:?})",
            ino, fh, lock_owner
        );
        reply.error(ENOSYS);
    }

    fn release(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _fh: u64,
        _flags: i32,
        _lock_owner: Option<u64>,
        _flush: bool,
        reply: ReplyEmpty,
    ) {
        reply.ok();
    }

    fn fsync(&mut self, _req: &Request<'_>, ino: u64, fh: u64, datasync: bool, reply: ReplyEmpty) {
        debug!(
            "[Not Implemented] fsync(ino: {:#x?}, fh: {}, datasync: {})",
            ino, fh, datasync
        );
        reply.error(ENOSYS);
    }

    fn opendir(&mut self, _req: &Request<'_>, ino: u64, flags: i32, reply: ReplyOpen) {
        debug!("call opendir inode: {:?}", ino);
        let (_, read, write) = match flags & libc::O_ACCMODE {
            libc::O_RDONLY => {
                // Behavior is undefined, but most filesystems return EACCES
                if flags & libc::O_TRUNC != 0 {
                    reply.error(libc::EACCES);
                    return;
                }
                (libc::R_OK, true, false)
            }
            libc::O_WRONLY => (libc::W_OK, false, true),
            libc::O_RDWR => (libc::R_OK | libc::W_OK, true, true),
            // Exactly one access mode flag must be specified
            _ => {
                reply.error(libc::EINVAL);
                return;
            }
        };
        match self.inner.open_dir(ino, read, write) {
            Ok(fh) => reply.opened(fh, FOPEN_DIRECT_IO),
            Err(e) => {
                error!("fail to open dir, err: {:?}", e);
                reply.error(ENOSYS);
            }
        }
    }

    fn readdir(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        match self.inner.read_dir(ino, offset, &mut reply) {
            Ok(()) => {
                info!("success call read_dir, inode: {:?}", ino);
                reply.ok();
            }
            Err(e) => {
                error!("fail to read dir, err: {:?}", e);
                reply.error(ENOSYS);
            }
        }
    }

    fn readdirplus(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        fh: u64,
        offset: i64,
        reply: ReplyDirectoryPlus,
    ) {
        debug!(
            "[Not Implemented] readdirplus(ino: {:#x?}, fh: {}, offset: {})",
            ino, fh, offset
        );
        reply.error(ENOSYS);
    }

    fn releasedir(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _fh: u64,
        _flags: i32,
        reply: ReplyEmpty,
    ) {
        reply.ok();
    }

    fn fsyncdir(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        fh: u64,
        datasync: bool,
        reply: ReplyEmpty,
    ) {
        debug!(
            "[Not Implemented] fsyncdir(ino: {:#x?}, fh: {}, datasync: {})",
            ino, fh, datasync
        );
        reply.error(ENOSYS);
    }

    fn statfs(&mut self, _req: &Request<'_>, _ino: u64, reply: ReplyStatfs) {
        reply.statfs(0, 0, 0, 0, 0, 512, 255, 0);
    }

    fn setxattr(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        name: &OsStr,
        _value: &[u8],
        flags: i32,
        position: u32,
        reply: ReplyEmpty,
    ) {
        debug!(
            "[Not Implemented] setxattr(ino: {:#x?}, name: {:?}, flags: {:#x?}, position: {})",
            ino, name, flags, position
        );
        reply.error(ENOSYS);
    }

    fn getxattr(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        name: &OsStr,
        size: u32,
        reply: ReplyXattr,
    ) {
        debug!(
            "[Not Implemented] getxattr(ino: {:#x?}, name: {:?}, size: {})",
            ino, name, size
        );
        reply.error(ENOSYS);
    }

    fn listxattr(&mut self, _req: &Request<'_>, ino: u64, size: u32, reply: ReplyXattr) {
        debug!(
            "[Not Implemented] listxattr(ino: {:#x?}, size: {})",
            ino, size
        );
        reply.error(ENOSYS);
    }

    fn removexattr(&mut self, _req: &Request<'_>, ino: u64, name: &OsStr, reply: ReplyEmpty) {
        debug!(
            "[Not Implemented] removexattr(ino: {:#x?}, name: {:?})",
            ino, name
        );
        reply.error(ENOSYS);
    }

    fn access(&mut self, _req: &Request<'_>, ino: u64, mask: i32, reply: ReplyEmpty) {
        debug!("[Not Implemented] access(ino: {:#x?}, mask: {})", ino, mask);
        reply.error(ENOSYS);
    }

    fn create(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        mode: u32,
        umask: u32,
        flags: i32,
        reply: ReplyCreate,
    ) {
        debug!(
            "[Not Implemented] create(parent: {:#x?}, name: {:?}, mode: {}, umask: {:#x?}, \
            flags: {:#x?})",
            parent, name, mode, umask, flags
        );
        reply.error(ENOSYS);
    }

    fn getlk(
        &mut self,
        _req: &fuser::Request<'_>,
        ino: u64,
        fh: u64,
        lock_owner: u64,
        start: u64,
        end: u64,
        typ: i32,
        pid: u32,
        reply: ReplyLock,
    ) {
        debug!(
            "[Not Implemented] getlk(ino: {:#x?}, fh: {}, lock_owner: {}, start: {}, \
            end: {}, typ: {}, pid: {})",
            ino, fh, lock_owner, start, end, typ, pid
        );
        reply.error(ENOSYS);
    }

    fn setlk(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        fh: u64,
        lock_owner: u64,
        start: u64,
        end: u64,
        typ: i32,
        pid: u32,
        sleep: bool,
        reply: ReplyEmpty,
    ) {
        debug!(
            "[Not Implemented] setlk(ino: {:#x?}, fh: {}, lock_owner: {}, start: {}, \
            end: {}, typ: {}, pid: {}, sleep: {})",
            ino, fh, lock_owner, start, end, typ, pid, sleep
        );
        reply.error(ENOSYS);
    }

    fn bmap(&mut self, _req: &Request<'_>, ino: u64, blocksize: u32, idx: u64, reply: ReplyBmap) {
        debug!(
            "[Not Implemented] bmap(ino: {:#x?}, blocksize: {}, idx: {})",
            ino, blocksize, idx,
        );
        reply.error(ENOSYS);
    }

    fn ioctl(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        fh: u64,
        flags: u32,
        cmd: u32,
        in_data: &[u8],
        out_size: u32,
        reply: ReplyIoctl,
    ) {
        debug!(
            "[Not Implemented] ioctl(ino: {:#x?}, fh: {}, flags: {}, cmd: {}, \
            in_data.len(): {}, out_size: {})",
            ino,
            fh,
            flags,
            cmd,
            in_data.len(),
            out_size,
        );
        reply.error(ENOSYS);
    }

    fn fallocate(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        fh: u64,
        offset: i64,
        length: i64,
        mode: i32,
        reply: ReplyEmpty,
    ) {
        debug!(
            "[Not Implemented] fallocate(ino: {:#x?}, fh: {}, offset: {}, \
            length: {}, mode: {})",
            ino, fh, offset, length, mode
        );
        reply.error(ENOSYS);
    }

    fn lseek(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        fh: u64,
        offset: i64,
        whence: i32,
        reply: ReplyLseek,
    ) {
        debug!(
            "[Not Implemented] lseek(ino: {:#x?}, fh: {}, offset: {}, whence: {})",
            ino, fh, offset, whence
        );
        reply.error(ENOSYS);
    }

    fn copy_file_range(
        &mut self,
        _req: &Request<'_>,
        ino_in: u64,
        fh_in: u64,
        offset_in: i64,
        ino_out: u64,
        fh_out: u64,
        offset_out: i64,
        len: u64,
        flags: u32,
        reply: ReplyWrite,
    ) {
        debug!(
            "[Not Implemented] copy_file_range(ino_in: {:#x?}, fh_in: {}, \
            offset_in: {}, ino_out: {:#x?}, fh_out: {}, offset_out: {}, \
            len: {}, flags: {})",
            ino_in, fh_in, offset_in, ino_out, fh_out, offset_out, len, flags
        );
        reply.error(ENOSYS);
    }

    fn setvolname(&mut self, _req: &Request<'_>, name: &OsStr, reply: ReplyEmpty) {
        debug!("[Not Implemented] setvolname(name: {:?})", name);
        reply.error(ENOSYS);
    }

    fn exchange(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        newparent: u64,
        newname: &OsStr,
        options: u64,
        reply: ReplyEmpty,
    ) {
        debug!(
            "[Not Implemented] exchange(parent: {:#x?}, name: {:?}, newparent: {:#x?}, \
            newname: {:?}, options: {})",
            parent, name, newparent, newname, options
        );
        reply.error(ENOSYS);
    }

    fn getxtimes(&mut self, _req: &Request<'_>, ino: u64, reply: ReplyXTimes) {
        debug!("[Not Implemented] getxtimes(ino: {:#x?})", ino);
        reply.error(ENOSYS);
    }
}
