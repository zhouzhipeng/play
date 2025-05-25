#!/usr/bin/env bash

set  -eux

trap 'mv ~/.cargo/config.toml.bak ~/.cargo/config.toml; exit 0' EXIT

# disable local `config.toml` temporarily
mv ~/.cargo/config.toml ~/.cargo/config.toml.bak

cargo clean
cargo update

cargo build --all-features
cargo dev_ui
cargo dev_server


echo "check ok."
