FROM zhouzhipeng/play-cache

WORKDIR /app
COPY . .

RUN cargo build  --release -p play-dylib-example