#!/usr/bin/env bash
set -eux

# for mac
export WASI_SDK_PATH=/Users/zhouzhipeng/Downloads/wasi-sdk-24.0-arm64-macos
export CC="${WASI_SDK_PATH}/bin/clang --sysroot=${WASI_SDK_PATH}/share/wasi-sysroot"
RUSTFLAGS="--cfg wasmedge --cfg tokio_unstable"  cargo build --target wasm32-wasi  --release -p play-wasm-example
