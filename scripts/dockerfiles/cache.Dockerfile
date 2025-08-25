FROM rust:1.89.0-bookworm

# install basic packages
RUN apt update && apt install -y lua5.4

WORKDIR /app
COPY . .

# rm to prevent real build is failed but still can copy file.
RUN mv .cargo/config-prod.toml .cargo/config.toml &&\
cargo dev_server && rm -rf target/release/play-server
