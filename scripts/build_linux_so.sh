#!/usr/bin/env bash

set  -eux

docker build -f scripts/dockerfiles/build_linux_dylib_example.Dockerfile -t tmpimg .
docker run --name tmpcontontainer tmpimg
docker cp tmpcontontainer:/app/target/release/libplay_dylib_example.so .