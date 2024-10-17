FROM rust:1.81.0-bookworm


WORKDIR /app
COPY . .

# rm to prevent real build is failed but still can copy file.
RUN cargo dev_server && rm -rf target/release/play
