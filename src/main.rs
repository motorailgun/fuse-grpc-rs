use fuser::{Filesystem, MountOption};
use libc::ENOENT;
use log::*;
use std::env;

struct GrpcFs {}

impl Filesystem for GrpcFs {
    fn getattr(&mut self, _req: &fuser::Request<'_>, _ino: u64, reply: fuser::ReplyAttr) {
        reply.error(ENOENT)
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
