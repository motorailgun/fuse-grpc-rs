# fuse-grpc-rs
## Usage
Run:
```bash
$ cargo run [server | client]
```
Note: currently mountpoint and listen/connection address is hard-corded, which are `/tmp/mnt` and `[::1]:50050`, respectively.

## Acknowledgement
Thanks to

- [Sherlock-Holo / fuse3](https://github.com/Sherlock-Holo/fuse3/tree/master) for async fuse3 crate
- [hyperium / tonic](https://github.com/hyperium/tonic) for gRPC crate
- [tokio](https://github.com/tokio-rs) for async runtime, prost, and more

and more...
