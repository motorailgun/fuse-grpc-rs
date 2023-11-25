pub mod client;
pub mod server;

use client::GrpcFsClient;
use server::rpc_fs::rpc_fs_server::RpcFsServer;
use server::GrpcFs;
use tonic::transport::Server;

use fuse3::raw::prelude::*;
use fuse3::MountOptions;

fn usage(exe_name: &str) {
    println!("usage: {exe_name} [subcommand] <options...>");
    println!("");
    println!("subcommands:");
    println!("    server");
    println!("    client");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    env_logger::init();

    if let Some(subcommand) = args.get(1) {
        match &**subcommand {
            "server" => {
                let addr = "[::1]:50051".parse()?;
                let grpc_fs = GrpcFs::default();

                Server::builder()
                    .add_service(RpcFsServer::new(grpc_fs))
                    .serve(addr)
                    .await?;
            }
            "client" => {
                let addr = String::from("http://[::1]:50051");
                let mountpoint = String::from("/tmp/mnt");
                let mut options = MountOptions::default();
                options.read_only(true).fs_name("GrpcFs"); //force_readdir_plus(true);
                Session::new(options)
                    .mount_with_unprivileged(GrpcFsClient::new(addr).await, mountpoint)
                    .await?
                    .await?;
            }
            _ => {
                usage(&args[0]);
            }
        }
    } else {
        usage(&args[0]);
    }

    Ok(())
}
