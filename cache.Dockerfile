FROM rust:bookworm

WORKDIR /app
COPY . .

RUN cargo dev_server
