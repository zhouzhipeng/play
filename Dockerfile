FROM python:3.11 as BuildImage
SHELL ["/bin/bash", "-c"]

# install openssl 1.1.1
RUN wget https://www.openssl.org/source/openssl-1.1.1t.tar.gz
RUN tar -zxf openssl-1.1.1t.tar.gz
RUN cd openssl-1.1.1t && ./config && make install && ldconfig

# install rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y


WORKDIR /app
COPY . .

RUN source "$HOME/.cargo/env" && \
    cargo dev_embed


FROM debian:12-slim
WORKDIR /app

COPY --from=BuildImage /app/target/release/play .
CMD /app/play