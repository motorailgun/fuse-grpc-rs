use fuser::{Filesystem, MountOption};
use libc::ENOENT;
use log::*;
use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::prelude::*;
use std::time::{Duration, SystemTime};

struct GrpcFs {}

impl Filesystem for GrpcFs {
    fn init(
        &mut self,
        _req: &fuser::Request<'_>,
        _config: &mut fuser::KernelConfig,
    ) -> Result<(), libc::c_int> {
        Ok(())
    }

    /* fn getattr(&mut self, _req: &fuser::Request<'_>, _ino: u64, reply: fuser::ReplyAttr) {
        reply.error(ENOENT)
    } */

    fn lookup(
        &mut self,
        _req: &fuser::Request<'_>,
        _parent: u64,
        name: &std::ffi::OsStr,
        reply: fuser::ReplyEntry,
    ) {
        let default_duration = Duration::from_secs(1);

        match fs::metadata(name) {
            Ok(dentry_metadata) => {
                let attr = |kind| fuser::FileAttr {
                    ino: dentry_metadata.ino(),
                    size: dentry_metadata.size(),
                    blocks: dentry_metadata.blocks(),
                    atime: SystemTime::UNIX_EPOCH,
                    mtime: SystemTime::UNIX_EPOCH,
                    ctime: SystemTime::UNIX_EPOCH,
                    crtime: SystemTime::UNIX_EPOCH,
                    kind: kind,
                    perm: dentry_metadata.permissions().mode() as u16,
                    nlink: dentry_metadata.nlink() as u32,
                    uid: dentry_metadata.uid(),
                    gid: dentry_metadata.gid(),
                    rdev: dentry_metadata.rdev() as u32,
                    blksize: dentry_metadata.blksize() as u32,
                    flags: 0, // as I don't use this thing on macOS
                };

                if dentry_metadata.is_dir() {
                    reply.entry(&default_duration, &attr(fuser::FileType::Directory), 0)
                } else if dentry_metadata.is_file() {
                    reply.entry(&default_duration, &attr(fuser::FileType::RegularFile), 0)
                } else {
                    reply.error(ENOENT);
                }
            }
            Err(_) => {
                reply.error(ENOENT);
            }
        };
    }

    fn readdir(
        &mut self,
        _req: &fuser::Request<'_>,
        _ino: u64,
        _fh: u64,
        _ffset: i64,
        reply: fuser::ReplyDirectory,
    ) {
        reply.error(ENOENT)
    }

    fn open(&mut self, _req: &fuser::Request<'_>, _ino: u64, _flags: i32, reply: fuser::ReplyOpen) {
        reply.error(ENOENT)
    }

    fn read(
        &mut self,
        _req: &fuser::Request<'_>,
        _ino: u64,
        _fh: u64,
        _offset: i64,
        _size: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: fuser::ReplyData,
    ) {
        reply.error(ENOENT)
    }
}

fn main() {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    info!("given command line arguments are: {}", args.join(" "));

    let mountpoint = String::from("/tmp/mnt");
    let mut options = vec![MountOption::RO, MountOption::FSName("hello".to_string())];
    // options.push(MountOption::AutoUnmount);
    // options.push(MountOption::AllowRoot);

    fuser::mount2(GrpcFs {}, mountpoint, &options).unwrap();
}
