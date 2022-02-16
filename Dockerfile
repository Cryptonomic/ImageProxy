FROM rustlang/rust:nightly-bullseye AS builder
WORKDIR /opt/img_proxy
COPY Cargo.lock .
COPY Cargo.toml .
COPY build.rs .
RUN mkdir -p ./src
COPY docker/stub.rs ./src/main.rs
RUN cargo build --release
COPY . .
RUN cargo build --release

FROM debian:bullseye
WORKDIR /opt/img_proxy
RUN apt-get update && apt-get upgrade -y && apt-get install ca-certificates -y && rm -rf /var/lib/apt/lists/*
COPY --from=builder /opt/img_proxy/target/release/nft_image_proxy /opt/img_proxy/nft_image_proxy
RUN mkdir -p /opt/img_proxy/sql
COPY sql/ /opt/img_proxy/sql
# Remember to change the below variable in `.cargo/config.toml`
ENV _RJEM_MALLOC_CONF="background_thread:true,tcache:false,dirty_decay_ms:0,muzzy_decay_ms:0,abort_conf:true"
CMD sleep 5 && /opt/img_proxy/nft_image_proxy
