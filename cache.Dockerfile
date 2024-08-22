FROM rust:1.78.0-bookworm


WORKDIR /app
COPY . .

RUN cargo dev_server
