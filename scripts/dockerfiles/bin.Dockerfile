FROM ghcr.io/zhouzhipeng/play-cache:latest

WORKDIR /app
COPY . .

RUN cargo dev_server
