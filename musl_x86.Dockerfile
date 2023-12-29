FROM --platform=linux/amd64  messense/rust-musl-cross:x86_64-musl

USER root

WORKDIR /app
COPY . .

RUN cargo dev

RUN musl-strip target/x86_64-unknown-linux-musl/release/play

FROM --platform=linux/amd64  gcr.io/distroless/static-debian11

COPY --from=0 /app/target/x86_64-unknown-linux-musl/release/play .

ENTRYPOINT ["/play"]