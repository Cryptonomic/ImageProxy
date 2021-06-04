FROM rust:1.52.1-bullseye AS builder
WORKDIR /opt/img_proxy
COPY Cargo.lock .
COPY Cargo.toml .
COPY build.rs .
RUN mkdir -p ./src
COPY docker/stub.rs ./src/main.rs
RUN cargo build --release
COPY . .
RUN cargo build --release

FROM node:16-buster AS dashboard-builder
WORKDIR /opt/dashboard
COPY dashboard-ui/package.* .
RUN npm i
COPY dashboard-ui/ ./
RUN npm run build

FROM debian:bullseye
WORKDIR /opt/img_proxy
RUN apt-get update && apt-get upgrade -y && apt-get install ca-certificates -y && rm -rf /var/lib/apt/lists/*
COPY --from=builder /opt/img_proxy/target/release/nft_image_proxy /opt/img_proxy/nft_image_proxy
COPY --from=dashboard-builder /opt/dashboard/build/ /opt/img_proxy/ui
COPY proxy.conf /opt/img_proxy/
COPY log4rs.yml /opt/img_proxy/
CMD /opt/img_proxy/nft_image_proxy
