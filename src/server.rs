use std::path::{PathBuf, Path};
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

// we do not want to keep inode-to-path translation table in server-side
// as it requires too much work on handler side
// instead, we do inode-to-path translation table in client-side,
// and the path is sent over RPCs
#[derive(Debug, Default)]
pub struct GrpcFs {}

#[tonic::async_trait]
impl RpcFs for GrpcFs {
    async fn get_attr(&self, request: Request<GetAttrRequest>) -> Result<Response<GetAttrReply>, Status> {
        let path = request.into_inner().path;
        match fs::metadata(&path) {
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
                debug!("failed to get metadata of {}", path);
            }
        }

        Err(Status::new(tonic::Code::NotFound, "not found"))
    }

    async fn read_dir(&self, request: Request<ReadDirRequest>) -> Result<Response<ReadDirReply>, Status> {
        let ReadDirRequest {
            path, offset
        } = request.into_inner();

        
        let path = Path::new(&path);
        if path.is_dir() {
            let dirs = match fs::read_dir(path) {
                Ok(dir) => dir,
                Err(_) => {
                    let msg = format!("failed to read directory {}", path.display());
                    debug!("{}", msg);
                    return Err(Status::new(tonic::Code::Internal, msg));
                }
            };

            let entries: Vec<DEntry> =
                dirs
                    .filter_map(|e| e.ok())
                    .skip(offset as usize)
                    .enumerate()
                    .map(|(offset, entry)| {
                        let kind = if entry.path().is_dir() {
                            FileType::Directory
                        } else {
                            FileType::Regular
                        };

                        let file_name = entry.file_name().into_string().unwrap();
                        let inode = entry.ino();
                        debug!("inode: {}, file_name: {:?}", inode, file_name);
                    
                    rpc_fs::DEntry {
                        inode,
                        offset: offset as u64,
                        file_name,
                        kind: kind.into(),
                    }
                }).collect();

                return Ok(Response::new(ReadDirReply {
                    entries
                }))
        }
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
