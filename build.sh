#!/bin/bash

usage() { echo "Usage: $0 < dev | dev_embed | prod | prod_embed | all >" 1>&2; exit 1; }

# unless there are 1 argument, print the "usage" and exit
[ ! $# -ge 1 ] && usage

pre_build(){
  set -eux

  cd prebuild
  cargo run
  cd ..

  export PYO3_CONFIG_FILE=$(pwd)/server/python/build/pyo3-build-config-file.txt

}

# Functions
dev() {
    set -eux
    cargo clean
    cargo build --release
}

dev_embed() {
    set -eux

    pre_build

    cargo clean
    cargo build --release --features=use_embed_python

}

prod() {
    set -eux

    cargo clean
    cargo build  --release  --no-default-features --features=use_mysql
}

prod_embed() {
    set -eux

    pre_build

    cargo clean
    cargo build  --release  --no-default-features --features=use_mysql,use_embed_python
}

all(){
  set -eux

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