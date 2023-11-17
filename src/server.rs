use tonic::{transport::Server, Request, Response, Status};

use rpc_fs::rpc_fs_server::{RpcFs, RpcFsServer};
use rpc_fs::*;

pub mod rpc_fs {
    tonic::include_proto!("rpc_fs");
}

#[derive(Debug, Default)]
pub struct GrpcFs {}

#[tonic::async_trait]
impl RpcFs for GrpcFs {
    async fn get_attr(&self, request: Request<GetAttrRequest>) -> Result<Response<GetAttrReply>, Status> {
        let request = request.into_inner();
        Err(Status::new(tonic::Code::NotFound, "not found"))
    }

    async fn read_dir(&self, request: Request<ReadDirRequest>) -> Result<Response<ReadDirReply>, Status> {
        Err(Status::new(tonic::Code::NotFound, "not found"))
    }
        
    async fn open(&self, request: Request<OpenRequest>) -> Result<Response<OpenReply>, Status> {
        Err(Status::new(tonic::Code::NotFound, "not found"))
    }

    async fn read(&self, request: Request<ReadRequest>) -> Result<Response<ReadReply>, Status> {
        Err(Status::new(tonic::Code::NotFound, "not found"))
    }
}
