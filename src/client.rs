use fuser::Filesystem;
use log::{warn, debug, error, info};
use rpc_fs::rpc_fs_client::RpcFsClient;
use rpc_fs::*;
use std::collections::BTreeMap;
use std::path::PathBuf;
use futures::executor;
use std::time::{Duration, SystemTime};

pub mod rpc_fs {
    tonic::include_proto!("rpc_fs");
}

pub struct GrpcFsClient {
    address: String,
    inode_map: BTreeMap<u64, PathBuf>,
    client: Option<RpcFsClient<tonic::transport::Channel>>,
}

impl GrpcFsClient {
    pub fn new(address: String) -> Self {
        let mut c = GrpcFsClient {
            address,
            inode_map: BTreeMap::new(),
            client: None,
        };
        c.inode_map.insert(1, PathBuf::from("/"));
        c
    }

    fn append_inode(&mut self, inode: u64, path: PathBuf) {
        if inode == 1 {
            warn!("inode number 1 is reserved: path \"{}\"", path.display());
            return;
        }
        self.inode_map.insert(inode, path);
    }
}

impl Filesystem for GrpcFsClient{
    fn init(
        &mut self,
        _req: &fuser::Request<'_>,
        _config: &mut fuser::KernelConfig,
    ) -> Result<(), libc::c_int> {
        let client = executor::block_on(RpcFsClient::connect(self.address.clone()));
        match client {
            Ok(cl) => {
                info!("successfully connected to server");
                self.client = Some(cl);
                return Ok(());
            }
            Err(e) => error!("failed to connect to server: {}", e),
        }
        Err(libc::EIO)
    }

    fn getattr(&mut self, _req: &fuser::Request<'_>, inode: u64, reply: fuser::ReplyAttr) {
        if let Some(path) = self.inode_map.get(&inode) {
            let client = self.client.as_mut().unwrap();
            let request = tonic::Request::new(GetAttrRequest {
                path: path.to_str().unwrap().to_string(),
            });

            let response = executor::block_on(client.get_attr(request));
            match response {
                Ok(response) => {
                    let attr = response.into_inner().attributes.unwrap();
                    let kind = attr.kind;
                    let perm = attr.permission;
                    let nlink = attr.nlink;
                    let uid = attr.uid;
                    let gid = attr.gid;
                    let size = attr.size;
                    let blksize = attr.blksize;
                    let blocks = attr.blocks;
                    let rdev = attr.rdev;

                    reply.attr(&Duration::new(1, 0), &fuser::FileAttr {
                        ino: inode,
                        size,
                        blocks,
                        atime: SystemTime::UNIX_EPOCH,
                        mtime: SystemTime::UNIX_EPOCH,
                        ctime: SystemTime::UNIX_EPOCH,
                        crtime: SystemTime::UNIX_EPOCH + Duration::from_secs(0),
                        kind: if kind == FileType::Directory.into() {
                            fuser::FileType::Directory
                        } else {
                            fuser::FileType::RegularFile
                        },
                        perm: perm as u16,
                        nlink,
                        uid,
                        gid,
                        rdev,
                        blksize,
                        flags: 0,
                    });
                    return;
                }
                Err(e) => {
                    warn!("failed to get attributes of {}: {}", path.display(), e);
                    reply.error(libc::ENOENT);
                }
            }
        }
    }

    fn lookup(&mut self, _req: &fuser::Request<'_>, parent: u64, name: &std::ffi::OsStr, reply: fuser::ReplyEntry) {
        if let Some(parent_path) = self.inode_map.get(&parent) {
            let path = parent_path.join(name);
            let client = self.client.as_mut().unwrap();
            let request = tonic::Request::new(GetAttrRequest {
                path: path.to_str().unwrap().to_string(),
            });

            let response = executor::block_on(client.get_attr(request));
            match response {
                Ok(response) => {
                    let attr = response.into_inner().attributes.unwrap();
                    let inode = attr.inode;
                    let kind = attr.kind;
                    let perm = attr.permission;
                    let nlink = attr.nlink;
                    let uid = attr.uid;
                    let gid = attr.gid;
                    let size = attr.size;
                    let blksize = attr.blksize;
                    let blocks = attr.blocks;
                    let rdev = attr.rdev;
                    
                    self.append_inode(inode, path);
                    reply.entry(&Duration::new(1, 0), &fuser::FileAttr {
                        ino: inode,
                        size,
                        blocks,
                        atime: SystemTime::UNIX_EPOCH,
                        mtime: SystemTime::UNIX_EPOCH,
                        ctime: SystemTime::UNIX_EPOCH,
                        crtime: SystemTime::UNIX_EPOCH + Duration::from_secs(0),
                        kind: if kind == FileType::Directory.into() {
                            fuser::FileType::Directory
                        } else {
                            fuser::FileType::RegularFile
                        },
                        perm: perm as u16,
                        nlink,
                        uid,
                        gid,
                        rdev,
                        blksize,
                        flags: 0,
                    }, 0);
                    return;
                }
                Err(_e) => {
                    // TODO: check if this is just 404, or other errors
                    info!("lookup: not found for path: {}", path.display());
                    reply.error(libc::ENOENT);
                }
            }
        }
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
            let path = path.clone();
            let client = self.client.as_mut().unwrap();
            let request = tonic::Request::new(ReadDirRequest {
                path: path.to_str().unwrap().to_string(),
                offset,
            });

            let response = executor::block_on(client.read_dir(request));
            match response {
                Ok(response) => {
                    let entries = response.into_inner().entries;
                    for entry in entries {
                        let kind = entry.kind;
                        let inode = entry.inode;
                        let name = entry.file_name;
                        let offset = entry.offset;

                        self.append_inode(inode, path.join(&name));

                        if reply.add(
                            inode,
                            offset as i64,
                            if kind == FileType::Directory.into() {
                                fuser::FileType::Directory
                            } else {
                                fuser::FileType::RegularFile
                            },
                            &name,
                        ) {
                            break;
                        }
                    }
                    reply.ok();
                }
                Err(e) => {
                    warn!("failed to read directory {}: {}", path.display(), e);
                    reply.error(libc::EIO);
                }
            }
        } else {
            reply.error(libc::ENOENT);
        }
    }

    fn open(&mut self, _req: &fuser::Request<'_>, inode: u64, flags: i32, reply: fuser::ReplyOpen) {
        match self.inode_map.get(&inode) {
            Some(path) => {
                let client = self.client.as_mut().unwrap();
                let request = tonic::Request::new(OpenRequest {
                    path: path.to_str().unwrap().to_string(),
                    flags,
                });
                let response = executor::block_on(client.open(request));
                match response {
                    Ok(response) => {
                        let fd = response.into_inner().fd;
                        reply.opened(fd as u64, flags.try_into().unwrap());
                    }
                    Err(e) => {
                        warn!("failed to open {}: {}", path.display(), e);
                        reply.error(libc::ENOENT);
                    }
                }
            }
            None => {
                reply.error(libc::ENOENT);
            }
        }
    }

    fn read(
            &mut self,
            _req: &fuser::Request<'_>,
            ino: u64,
            _fh: u64,
            offset: i64,
            size: u32,
            _flags: i32,
            _lock_owner: Option<u64>,
            reply: fuser::ReplyData,
        ) {
        if let Some(path) = self.inode_map.get(&ino) {
            let client = self.client.as_mut().unwrap();
            let request = tonic::Request::new(ReadRequest {
                path: path.to_str().unwrap().to_string(),
                offset: offset.try_into().unwrap(),
                size: size.into(),
            });
            let response = executor::block_on(client.read(request));
            match response {
                Ok(response) => {
                    let data = response.into_inner().data;
                    reply.data(&data);
                }
                Err(e) => {
                    warn!("failed to read {}: {}", path.display(), e);
                    reply.error(libc::ENOENT);
                }
            }
        } else {
            reply.error(libc::ENOENT);
        }
    }
}
