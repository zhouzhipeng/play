FROM ghcr.io/zhouzhipeng/play-cache:latest

WORKDIR /app
COPY . .

RUN cargo dev_server

FROM debian:bookworm-slim
WORKDIR /app

COPY --from=0 /app/target/release/play-server .

ENTRYPOINT ["/app/play-server"]