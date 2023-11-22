pub mod server;
pub mod client;

use client::GrpcFsClient;
use server::GrpcFs;
use server::rpc_fs::rpc_fs_server::RpcFsServer;
use tonic::transport::Server;

use fuser:: MountOption;


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
                let addr = "[::1]:50051".parse()?;
                let mountpoint = String::from("/tmp/mnt");
                let options = vec![MountOption::RO, MountOption::FSName("GrpcFs".to_string())];
                fuser::mount2(GrpcFsClient::new(addr), mountpoint, &options)?;
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
