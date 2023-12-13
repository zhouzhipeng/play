FROM python as BuildImage
SHELL ["/bin/bash", "-c"]

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

WORKDIR /app
COPY . .

RUN source "$HOME/.cargo/env" && \
    cargo build && \
    export PYO3_CONFIG_FILE=$(pwd)/server/python/build/pyo3-build-config-file.txt && \
    cargo build --release


FROM debian:12-slim
WORKDIR /app

COPY --from=BuildImage /app/target/release/play .
CMD /app/play