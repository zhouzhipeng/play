FROM rust:1.85.0-bookworm

# install basic packages
RUN apt update && apt install -y lua5.4 tesseract-ocr tesseract-ocr-chi-sim

WORKDIR /app
COPY . .

# rm to prevent real build is failed but still can copy file.
RUN cargo dev_server && rm -rf target/release/play
