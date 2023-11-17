use std::collections::BTreeMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::{Duration, SystemTime};
use std::fs;
use std::fs::DirEntry;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::prelude::*;
use log::*;

use tonic::{transport::Server, Request, Response, Status};

use rpc_fs::rpc_fs_server::{RpcFs, RpcFsServer};
use rpc_fs::*;

pub mod rpc_fs {
    tonic::include_proto!("rpc_fs");
}

#[derive(Debug, Default)]
pub struct GrpcFs {
    inode_map: BTreeMap<u64, PathBuf>,
}

impl GrpcFs {
    fn new() -> Self {
        let mut fs = GrpcFs {
            inode_map: BTreeMap::new(),
        };

        fs.inode_map.insert(1, PathBuf::from_str("/").unwrap());
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

#[tonic::async_trait]
impl RpcFs for GrpcFs {
    async fn get_attr(&self, request: Request<GetAttrRequest>) -> Result<Response<GetAttrReply>, Status> {
        let inode = request.into_inner().inode;
        if let Some(path) = self.inode_map.get(&inode) {
            match fs::metadata(path) {
                Ok(dentry_metadata) => {
                    let reply = |kind| GetAttrReply {
                        attributes: Some(Attr {
                            inode: dentry_metadata.ino(),
                            size: dentry_metadata.size(),
                            blocks: dentry_metadata.blocks(),
                            kind: kind,
                            permission: dentry_metadata.permissions().mode(),
                            nlink: dentry_metadata.nlink() as u32,
                            uid: dentry_metadata.uid(),
                            gid: dentry_metadata.gid(),
                            rdev: dentry_metadata.rdev() as u32,
                            blksize: dentry_metadata.blksize() as u32,
                        })
                    };

                    return Ok(Response::new(if dentry_metadata.is_dir() {
                        reply(FileType::Directory.into())
                    } else {
                        reply(FileType::Regular.into())
                    }));
                }
                Err(_) => {
                    debug!("failed to get metadata of {}", path.display());
                }
            }
        }

        Err(Status::new(tonic::Code::NotFound, "not found"))
    }

    async fn read_dir(&self, request: Request<ReadDirRequest>) -> Result<Response<ReadDirReply>, Status> {
        let request = request.into_inner();
        Err(Status::new(tonic::Code::NotFound, "not found"))
    }
        
    async fn open(&self, request: Request<OpenRequest>) -> Result<Response<OpenReply>, Status> {
        let request = request.into_inner();
        Err(Status::new(tonic::Code::NotFound, "not found"))
    }

    async fn read(&self, request: Request<ReadRequest>) -> Result<Response<ReadReply>, Status> {
        let request = request.into_inner();
        Err(Status::new(tonic::Code::NotFound, "not found"))
    }
}
