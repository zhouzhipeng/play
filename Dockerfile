FROM rust:bookworm

WORKDIR /app
COPY . .

# 由于GitHub Actions已缓存target目录，我们将其复制进来以利用增量编译
COPY target target
COPY ~/.cargo/registry ~/.cargo/registry

RUN cargo dev_server

FROM debian:bookworm-slim
WORKDIR /app

COPY --from=0 /app/target/release/play .

ENTRYPOINT ["/app/play"]