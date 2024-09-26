#!/usr/bin/env bash

set  -eux

rustup target add x86_64-unknown-linux-gnu

cargo build --target x86_64-unknown-linux-gnu --release -p play-dylib-example