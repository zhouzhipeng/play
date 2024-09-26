FROM zhouzhipeng/play-cache

WORKDIR /app
COPY . .

RUN cargo build --target x86_64-unknown-linux-gnu --release -p play-dylib-example