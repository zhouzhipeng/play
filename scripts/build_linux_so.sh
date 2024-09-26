#!/usr/bin/env bash

set  -eux

docker build -f scripts/dockerfiles/build_linux_dylib_example.Dockerfile -t tmpimg .
docker run --name tmp tmpimg
docker cp tmp:/app/target/release/libplay_dylib_example.so .
docker rm -f tmp
# test
docker run tmpimg cargo test --lib tests::test_load_and_run_in_docker -p play-dylib-loader