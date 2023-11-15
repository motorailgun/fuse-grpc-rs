use fuser::FileType;
use fuser::{Filesystem, MountOption};
use libc::ENOENT;
use log::*;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::fs::DirEntry;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::prelude::*;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::{Duration, SystemTime};

struct GrpcFs {
    inode_map: BTreeMap<u64, PathBuf>,
}

impl GrpcFs {
    fn new() -> Self {
        let mut fs = GrpcFs {
            inode_map: BTreeMap::new(),
        };

        fs.inode_map.insert(0, PathBuf::from_str("/").unwrap());
        fs
    }

    fn append_inode(&mut self, key: u64, path: PathBuf) -> Result<(), String> {
        if key == 1 {
            return Err("inode number 1 is reserved for root directory".to_string());
        }
        self.inode_map.insert(key, path);
        Ok(())
    }
}

impl Filesystem for GrpcFs {
    fn init(
        &mut self,
        _req: &fuser::Request<'_>,
        _config: &mut fuser::KernelConfig,
    ) -> Result<(), libc::c_int> {
        Ok(())
    }

    fn getattr(&mut self, _req: &fuser::Request<'_>, inode: u64, reply: fuser::ReplyAttr) {
        if let Some(path) = self.inode_map.get(&inode) {
            match fs::metadata(path) {
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
                        reply.attr(&Duration::from_secs(1), &attr(fuser::FileType::Directory))
                    } else if dentry_metadata.is_file() {
                        reply.attr(&Duration::from_secs(1), &attr(fuser::FileType::RegularFile))
                    } else {
                        debug!("unknown file type");
                        reply.error(ENOENT);
                    }
                }
                Err(_) => {
                    debug!("failed to get metadata of {}", path.display());
                    reply.error(ENOENT);
                }
            }
        } else {
            reply.error(ENOENT);
        }
    }

    fn lookup(
        &mut self,
        _req: &fuser::Request<'_>,
        parent: u64,
        name: &std::ffi::OsStr,
        reply: fuser::ReplyEntry,
    ) {
        let default_duration = Duration::from_secs(1);
        let name = if let Some(parent_path) = self.inode_map.get(&parent) {
            Path::new(parent_path).join(name)
        } else {
            reply.error(ENOENT);
            return;
        };

        match fs::metadata(&name) {
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

                debug!("insert: inode {}, path {}", dentry_metadata.ino(), name.display());
                match self.append_inode(dentry_metadata.ino(), name) {
                    Err(err) => warn!("{}", err),
                    _ => (),
                }

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
        inode: u64,
        _fh: u64,
        offset: i64,
        mut reply: fuser::ReplyDirectory,
    ) {
        if let Some(path) = self.inode_map.get(&inode) {
            let path = Path::new(path);
            if path.is_dir() {
                let dirs = match fs::read_dir(path) {
                    Ok(dir) => dir,
                    Err(_) => {
                        debug!("failed to read directory {}", path.display());
                        reply.error(ENOENT);
                        return;
                    }
                };

                let entries: Vec<DirEntry> =
                    dirs.filter(Result::is_ok).map(|ok| ok.unwrap()).collect();
                for (i, entry) in entries.iter().enumerate().skip(offset as usize) {
                    let file_type = if entry.path().is_dir() {
                        FileType::Directory
                    } else {
                        FileType::RegularFile
                    };
                    let file_name = entry.file_name();
                    let inode = entry.ino();
                    debug!("inode: {}, file_name: {:?}", inode, file_name);

                    debug!("insert: inode {}, path {}", inode, entry.path().display());
                    match self.append_inode(inode, entry.path()) {
                        Err(err) => warn!("{}", err),
                        _ => (),
                    }

                    if reply.add(inode, (i + 1) as i64, file_type, file_name) {
                        break;
                    }
                }

                reply.ok();
            } else {
                reply.error(ENOENT);
            }
        } else {
            reply.error(ENOENT)
        }
    }

    fn open(&mut self, _req: &fuser::Request<'_>, inode: u64, flags: i32, reply: fuser::ReplyOpen) {
        match self.inode_map.get(&inode) {
            Some(path) => {
                if path.is_file() {
                    reply.opened(0, flags as u32);
                }
            },
            None => {
                reply.error(ENOENT);
            }
        }
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
    let options = vec![MountOption::RO, MountOption::FSName("hello".to_string())];
    // options.push(MountOption::AutoUnmount);
    // options.push(MountOption::AllowRoot);

    fuser::mount2(GrpcFs::new(), mountpoint, &options).unwrap();
}
