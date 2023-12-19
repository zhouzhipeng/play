FROM python:3.11 as BuildImage
SHELL ["/bin/bash", "-c"]

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

WORKDIR /app
COPY . .

RUN source "$HOME/.cargo/env" && \
    ./build.sh dev


FROM python:3.11-slim
WORKDIR /app

COPY --from=BuildImage /app/target/release/play .
CMD /app/play