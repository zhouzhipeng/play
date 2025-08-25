FROM ghcr.io/zhouzhipeng/play-cache:latest

WORKDIR /app
COPY . .

RUN  mv .cargo/config-prod.toml .cargo/config.toml &&\
cargo dev_server && cp /app/target/release/play-server .