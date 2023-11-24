use fuse3::raw::prelude::*;
#[allow(unused_imports)]
use log::{warn, debug, error, info};
use rpc_fs::rpc_fs_client::RpcFsClient;
use rpc_fs::*;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use futures_util::stream;
use futures_util::stream::Iter;
use std::iter::Skip;
use std::vec::IntoIter;
use fuse3::Result;
use tokio::sync::RwLock;

pub mod rpc_fs {
    tonic::include_proto!("rpc_fs");
}

pub struct GrpcFsClient {
    inode_map: RwLock<BTreeMap<u64, PathBuf>>,
    #[allow(dead_code)]
    address: String,
    client: Option<RpcFsClient<tonic::transport::Channel>>,
}

impl GrpcFsClient {
    pub async fn new(address: String) -> Self {
        let client = RpcFsClient::connect(address.clone()).await;
        match client {
            Ok(_) => {
                info!("successfully connected to server");
            }
            Err(e) => panic!("failed to connect to server: {}", e),
        }

        let c = GrpcFsClient {
            inode_map: RwLock::new(BTreeMap::new()),
            address,
            client: Some(client.unwrap()),
        };
        c.inode_map.write().await.insert(1, PathBuf::from("/"));

        c
    }

    async fn append_inode(&self, inode: u64, path: PathBuf) {
        if inode == 1 {
            warn!("inode number 1 is reserved: path \"{}\"", path.display());
            return;
        }
        debug!("caching: inode #{}, path = {}", inode, path.display());
        self.inode_map.write().await.insert(inode, path);
    }
    
    async fn get_path(&self, inode: u64) -> Option<PathBuf> {
        if let Some(path) = self.inode_map.read().await.get(&inode) {
            return Some(path.clone());
        }
        None
    }
}

#[async_trait::async_trait]
impl Filesystem for GrpcFsClient{
    type DirEntryStream = Iter<Skip<IntoIter<Result<DirectoryEntry>>>>;
    type DirEntryPlusStream = Iter<Skip<IntoIter<Result<DirectoryEntryPlus>>>>;

    async fn init(
        &self, _req: Request
    ) -> Result<()> {
        Ok(())
    }

    async fn destroy(&self, _req: Request) {}

    async fn getattr(&self, _req: Request, inode: u64, _fh: Option<u64>, _flags: u32) -> Result<ReplyAttr> {
        debug!("getattr: inode {}", inode);
        if let Some(path) = self.get_path(inode).await {
            let mut client = self.client.clone().unwrap();
            let request = tonic::Request::new(GetAttrRequest {
                path: path.to_str().unwrap().to_string(),
            });

            let response = client.get_attr(request).await;
            match response {
                Ok(response) => {
                    let attr = response.into_inner().attributes.unwrap();
                    let kind = attr.kind;
                    let perm = 0o600; // attr.permission;
                    let nlink = attr.nlink;
                    let uid = attr.uid;
                    let gid = attr.gid;
                    let size = attr.size;
                    let blksize = attr.blksize;
                    let blocks = attr.blocks;
                    let rdev = attr.rdev;

                    return Ok(
                        ReplyAttr {
                            ttl: Duration::from_secs(1),
                            attr: FileAttr {
                                ino: inode,
                                generation: 1,
                                size,
                                blocks,
                                atime: SystemTime::UNIX_EPOCH.into(),
                                mtime: SystemTime::UNIX_EPOCH.into(),
                                ctime: SystemTime::UNIX_EPOCH.into(),
                                kind: if kind == rpc_fs::FileType::Directory.into() {
                                    fuse3::FileType::Directory
                                } else {
                                    fuse3::FileType::RegularFile
                                },
                                perm: 0o600, // perm as u16,
                                nlink,
                                uid,
                                gid,
                                rdev,
                                blksize,
                            }
                        }
                    );
                },
                Err(e) => {
                    warn!("failed to get attributes of {}: {}", path.display(), e);
                }
            }
        }
        Err(libc::ENOENT.into())
    }

    async fn lookup(&self, _req: Request, parent: u64, name: &std::ffi::OsStr) -> Result<ReplyEntry> {
        debug!("lookup: parent {}, name {}", parent, name.to_string_lossy());
        if let Some(parent_path) = self.get_path(parent).await {
            let path = parent_path.join(name);
            let mut client = self.client.clone().unwrap();
            let request = tonic::Request::new(GetAttrRequest {
                path: path.to_str().unwrap().to_string(),
            });

            let response = client.get_attr(request).await;
            match response {
                Ok(response) => {
                    let attr = response.into_inner().attributes.unwrap();
                    let inode = attr.inode;
                    let kind = attr.kind;
                    let perm = 0o600;
                    let nlink = attr.nlink;
                    let uid = attr.uid;
                    let gid = attr.gid;
                    let size = attr.size;
                    let blksize = attr.blksize;
                    let blocks = attr.blocks;
                    let rdev = attr.rdev;
                    
                    return dbg!(Ok(
                        ReplyEntry {
                            ttl: Duration::from_secs(1),
                            attr: FileAttr {
                                ino: inode,
                                generation: 1,
                                size,
                                blocks,
                                atime: SystemTime::UNIX_EPOCH.into(),
                                mtime: SystemTime::UNIX_EPOCH.into(),
                                ctime: SystemTime::UNIX_EPOCH.into(),
                                kind: if kind == rpc_fs::FileType::Directory.into() {
                                    fuse3::FileType::Directory
                                } else {
                                    fuse3::FileType::RegularFile
                                },
                                perm: 0o600, //perm as u16,
                                nlink,
                                uid,
                                gid,
                                rdev,
                                blksize,
                            },
                            generation: 1,
                        }
                    ));
                }
                Err(_e) => {
                    // TODO: check if this is just 404, or other errors
                    info!("lookup: not found for path: {}", path.display());
                }
            }
        }
        Err(libc::ENOENT.into())
    }

    async fn readdir(
            &self,
            _req: Request,
            inode: u64,
            _fh: u64,
            offset: i64,
        ) -> Result<ReplyDirectory<Self::DirEntryStream>> {
        debug!("readdir: inode {}, offset {}", inode, offset);
        if let Some(path) = self.get_path(inode).await {
            let path = path.clone();
            let mut client = self.client.clone().unwrap();
            let request = tonic::Request::new(ReadDirRequest {
                path: path.to_str().unwrap().to_string(),
                offset,
            });

            let response = client.read_dir(request).await;
            match response {
                Ok(response) => {
                    let entries: Vec<_> = response.into_inner().entries.into_iter().map(move |entry| {
                        let DEntry {
                            kind, inode, offset,
                            file_name: name,
                        } = entry;

                        let inode = if name == "." || name == ".." {1} else {inode};
                        futures::executor::block_on(self.append_inode(inode, path.join(&name)));

                        Ok(DirectoryEntry {
                            inode,
                            offset: offset as i64,
                            kind: {if kind == rpc_fs::FileType::Directory.into() {
                                fuse3::FileType::Directory
                            } else {
                                fuse3::FileType::RegularFile
                            }},
                            name: name.into(),
                        })
                    }).collect();
                    Ok(ReplyDirectory {
                        entries: stream::iter(entries.into_iter().skip(offset as usize)),
                    })
                },
                Err(e) => {
                    warn!("failed to read directory {}: {}", path.display(), e);
                    Err(libc::ENOENT.into())
                },
            }
        } else {
            Err(libc::ENOENT.into())
        }
    }

    async fn readdirplus(&self, _req: Request, parent: u64, _fh: u64, offset: u64, _lock_owner: u64) -> Result<ReplyDirectoryPlus<Self::DirEntryPlusStream>> {
        debug!("readdirplus: parent {}, offset {}", parent, offset);
        if let Some(path) = self.get_path(parent).await {
            let mut client = self.client.clone().unwrap();
            let request = tonic::Request::new(ReadDirRequest {
                path: path.to_str().unwrap().to_string(),
                offset: offset.try_into().unwrap(), // blame if someone put minus-value into offset
            });

            let response = client.read_dir_plus(request).await;
            match response {
                Ok(response) => {
                    let entries: Vec<_> = response.into_inner().entries.into_iter().map(move |entry| {
                        let DEntryPlus {
                            kind, inode, offset, name, attr,
                        } = entry;

                        if attr == None {
                            warn!("empty attr on readdirplus!");
                            return Err(libc::ENOENT.into());
                        }
                        let attr = attr.unwrap();
                        let inode = if name == "." || name == ".." {1} else {inode};
                        futures::executor::block_on(self.append_inode(inode, path.join(&name)));

                        Ok(DirectoryEntryPlus {
                            inode,
                            offset: offset as i64,
                            kind: {if kind == rpc_fs::FileType::Directory.into() {
                                fuse3::FileType::Directory
                            } else {
                                fuse3::FileType::RegularFile
                            }},
                            name: name.into(),
                            generation: 1,
                            entry_ttl: Duration::from_secs(1),
                            attr_ttl: Duration::from_secs(1),
                            attr: FileAttr {
                                ino: inode,
                                generation: 1,
                                size: attr.size,
                                blocks: attr.blocks,
                                atime: SystemTime::UNIX_EPOCH.into(),
                                mtime: SystemTime::UNIX_EPOCH.into(),
                                ctime: SystemTime::UNIX_EPOCH.into(),
                                kind: if kind == rpc_fs::FileType::Directory.into() {
                                    fuse3::FileType::Directory
                                } else {
                                    fuse3::FileType::RegularFile
                                },
                                perm: 0o600, // attr.permission as u16,
                                nlink: attr.nlink,
                                uid: attr.uid,
                                gid: attr.gid,
                                rdev: attr.rdev,
                                blksize: attr.blksize,
                            },
                        })
                    }).collect();
                    Ok(ReplyDirectoryPlus {
                        entries: stream::iter(entries.into_iter().skip(offset as usize)),
                    })
                },
                Err(_) => {
                    warn!("error on readdirplus grpc request!");
                    Err(libc::ENOENT.into())
                }
            }
        } else {
            Err(libc::ENOENT.into())
        }
    }

    async fn open(&self, _req: Request, inode: u64, flags: u32) -> Result<ReplyOpen> {
        debug!("open: inode {}", inode);
        match self.get_path(inode).await {
            Some(path) => {
                let mut client = self.client.clone().unwrap();
                let request = tonic::Request::new(OpenRequest {
                    path: path.to_str().unwrap().to_string(),
                    flags,
                });
                let response = client.open(request).await;
                match response {
                    Ok(response) => {
                        let fd = response.into_inner().fd;
                        Ok(ReplyOpen{fh: fd as u64, flags})
                    },
                    Err(e) => {
                        warn!("failed to open {}: {}", path.display(), e);
                        Err(libc::ENOENT.into())
                    }
                }
            }
            None => {
                Err(libc::ENOENT.into())
            }
        }
    }

    async fn read(
            &self,
            _req: Request,
            ino: u64,
            _fh: u64,
            offset: u64,
            size: u32,
        ) -> Result<ReplyData> {
        debug!("read: inode {}, offset {}, size {}", ino, offset, size);
        if let Some(path) = self.get_path(ino).await {
            let mut client = self.client.clone().unwrap();
            let request = tonic::Request::new(ReadRequest {
                path: path.to_str().unwrap().to_string(),
                offset: offset.try_into().unwrap(),
                size: size.into(),
            });
            let response = client.read(request).await;
            match response {
                Ok(response) => {
                    let data = response.into_inner().data;
                    Ok(ReplyData{data: bytes::Bytes::copy_from_slice(&data)})
                }
                Err(e) => {
                    warn!("failed to read {}: {}", path.display(), e);
                    Err(libc::ENOENT.into())
                }
            }
        } else {
            Err(libc::ENOENT.into())
        }
    }

    async fn statfs(&self, _req: Request, _inode: u64) -> Result<ReplyStatFs> {
        warn!("statfs isn't implemented yet");
        Err(libc::ENOSYS.into())
    }
}
