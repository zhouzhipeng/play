FROM rust:bookworm

WORKDIR /app
COPY . .

RUN cargo dev_server

FROM debian:bookworm-slim
WORKDIR /app

COPY --from=0 /app/target/release/play .

ENTRYPOINT ["/app/play"]