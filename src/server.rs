use log::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::prelude::*;
use std::path::Path;

use tonic::{Request, Response, Status};

use rpc_fs::rpc_fs_server::RpcFs;
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
    async fn get_attr(
        &self,
        request: Request<GetAttrRequest>,
    ) -> Result<Response<GetAttrReply>, Status> {
        debug!("grpc: get_attr");
        let path = request.into_inner().path;
        match fs::metadata(&path) {
            Ok(dentry_metadata) => {
                let reply = |kind| GetAttrReply {
                    attributes: Some(Attr {
                        inode: dentry_metadata.ino(),
                        size: dentry_metadata.size(),
                        blocks: dentry_metadata.blocks(),
                        kind,
                        permission: dentry_metadata.permissions().mode(),
                        nlink: dentry_metadata.nlink() as u32,
                        uid: dentry_metadata.uid(),
                        gid: dentry_metadata.gid(),
                        rdev: dentry_metadata.rdev() as u32,
                        blksize: dentry_metadata.blksize() as u32,
                    }),
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

    async fn look_up(
        &self,
        request: Request<LookUpRequest>,
    ) -> Result<Response<LookUpReply>, Status> {
        debug!("grpc: lookup");
        let path = request.into_inner().path;
        match fs::metadata(&path) {
            Ok(dentry_metadata) => {
                let reply = |kind| LookUpReply {
                    attributes: Some(Attr {
                        inode: dentry_metadata.ino(),
                        size: dentry_metadata.size(),
                        blocks: dentry_metadata.blocks(),
                        kind,
                        permission: dentry_metadata.permissions().mode(),
                        nlink: dentry_metadata.nlink() as u32,
                        uid: dentry_metadata.uid(),
                        gid: dentry_metadata.gid(),
                        rdev: dentry_metadata.rdev() as u32,
                        blksize: dentry_metadata.blksize() as u32,
                    }),
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

    async fn read_dir(
        &self,
        request: Request<ReadDirRequest>,
    ) -> Result<Response<ReadDirReply>, Status> {
        debug!("grpc: read_dir");
        let ReadDirRequest { path, offset } = request.into_inner();

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

            let entries: Vec<DEntry> = dirs
                .filter_map(|e| e.ok())
                .skip(offset as usize)
                .enumerate()
                .map(|(idx, entry)| {
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
                        offset: idx as u64 + 1,
                        file_name,
                        kind: kind.into(),
                    }
                })
                .collect();

            return Ok(Response::new(ReadDirReply { entries }));
        }
        Err(Status::new(tonic::Code::NotFound, "not found"))
    }

    async fn read_dir_plus(
        &self,
        request: Request<ReadDirRequest>,
    ) -> Result<Response<ReadDirPlusReply>, Status> {
        debug!("grpc: read_dir_plus");
        let ReadDirRequest { path, offset } = request.into_inner();

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

            let entries: Vec<DEntryPlus> = dirs
                .filter_map(|e| e.ok())
                .skip(offset as usize)
                .enumerate()
                .map(|(idx, entry)| {
                    let kind = if entry.path().is_dir() {
                        FileType::Directory
                    } else {
                        FileType::Regular
                    };

                    let file_name = entry.file_name().into_string().unwrap();
                    let inode = entry.ino();
                    debug!("inode: {}, file_name: {:?}", inode, file_name);

                    let dentry_metadata = fs::metadata(path.join(&file_name)).unwrap();
                    let attrs = rpc_fs::Attr {
                        inode: dentry_metadata.ino(),
                        size: dentry_metadata.size(),
                        blocks: dentry_metadata.blocks(),
                        kind: kind.into(),
                        permission: dentry_metadata.permissions().mode(),
                        nlink: dentry_metadata.nlink() as u32,
                        uid: dentry_metadata.uid(),
                        gid: dentry_metadata.gid(),
                        rdev: dentry_metadata.rdev() as u32,
                        blksize: dentry_metadata.blksize() as u32,
                    };

                    rpc_fs::DEntryPlus {
                        inode,
                        offset: idx as u64 + 1,
                        name: file_name,
                        kind: kind.into(),
                        attr: Some(attrs),
                    }
                })
                .collect();

            return Ok(Response::new(ReadDirPlusReply { entries }));
        }
        Err(Status::new(tonic::Code::NotFound, "not found"))
    }

    async fn open(&self, request: Request<OpenRequest>) -> Result<Response<OpenReply>, Status> {
        debug!("grpc: open");
        let OpenRequest { path, .. } = request.into_inner();
        let path = Path::new(&path);
        if path.exists() {
            return Ok(Response::new(OpenReply { fd: 0 }));
        }
        Err(Status::new(tonic::Code::NotFound, "not found"))
    }

    async fn read(&self, request: Request<ReadRequest>) -> Result<Response<ReadReply>, Status> {
        debug!("grpc: read");
        let ReadRequest { path, offset, size } = request.into_inner();
        let path = Path::new(&path);

        if path.is_file() {
            if let Ok(file) = fs::File::open(path) {
                let mut buffer = vec![0; size as usize];
                if let Ok(_) = file.read_at(&mut buffer, offset) {
                    return Ok(Response::new(ReadReply { data: buffer }));
                }
            }
        }

        Err(Status::new(tonic::Code::NotFound, "not found"))
    }
}
