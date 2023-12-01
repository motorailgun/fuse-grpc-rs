FROM rust:1.74-slim-bookworm

WORKDIR /build
COPY . .

RUN apt update && apt upgrade -y && \
    apt install -y protobuf-compiler libprotobuf-dev libfuse-dev && \
    apt clean && \
    rm -rf /var/lib/apt/lists/*

RUN cargo install --path .

WORKDIR /sandbox
RUN rm -rf /build
RUN dd if=/dev/urandom of=./test.img bs=128M count=1

ENV SERVER_ADDRESS=0.0.0.0:50051 RUST_LOG=info

ENTRYPOINT [ "fuse-grpc-rs", "server" ]
