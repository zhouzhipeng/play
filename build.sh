#!/bin/bash

usage() { echo "Usage: $0 < dev | dev_embed | prod | prod_embed | all >" 1>&2; exit 1; }

# unless there are 1 argument, print the "usage" and exit
[ ! $# -ge 1 ] && usage

# Functions
dev() {
    set -eux

    cargo clean
    cargo build
    cargo build --release
}

dev_embed() {
    set -eux

    cargo clean
    cargo build  --features=use_embed_python
    export PYO3_CONFIG_FILE=$(pwd)/server/python/build/pyo3-build-config-file.txt
    cargo build --release --features=use_embed_python

}

prod() {
    set -eux

    cargo clean
    cargo build  --no-default-features --features=prod
    cargo build  --release  --no-default-features --features=prod
}

prod_embed() {
    set -eux

    cargo clean
    cargo build  --no-default-features --features=prod,use_embed_python
    export PYO3_CONFIG_FILE=$(pwd)/server/python/build/pyo3-build-config-file.txt
    cargo build  --release  --no-default-features --features=prod,use_embed_python
}

all(){
  dev
  dev_embed
  prod
  prod_embed
}

# Execution
for i in "$@"
do

    case "$i" in

        dev)
            dev &
            ;;

        dev_embed)
            dev_embed &
            ;;

        prod)
            prod &
            ;;

        prod_embed)
            prod_embed &
            ;;
        all)
            all &
            ;;

        *)
            usage
            ;;
    esac
done
wait