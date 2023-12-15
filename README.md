## Play
A playground project writen in rust for fun.


## about workspace
* shared :  models and some common functions should be put here (should be simple)
* client : built to wasm file and be copied to server folder , finally runs in browser
* server : a http server providing APIs and static files and templates.
* server/python : for server side templates, we run a python interpreter inside rust.

## local debug
the `debug` feature will activate live-reload mode for `static` and `templates` folders.
```bash
cargo run  --features=debug
```

## build python library
```bash
# set env
PYO3_CONFIG_FILE=/Users/zhouzhipeng/RustroverProjects/play/server/python/build/pyo3-build-config-file.txt
```


## build binary
```bash
## run  `cargo build ` firstly because we need to generate wasm files (which will cause deadlock in --release mode)
# dev (default)
cargo clean && cargo build && PYO3_CONFIG_FILE=$(pwd)/server/python/build/pyo3-build-config-file.txt   cargo build --release

# prod
cargo clean &&  cargo build &&  cargo build --release  --no-default-features --features=prod
```

## running
put the final binary `play` on your server , and just run `./play` , everything is embed in it including config files.


## run a redis cluster locally
```bash
docker run -e "IP=0.0.0.0" -p 7000-7005:7000-7005 grokzen/redis-cluster:latest
```

## known issues
* `output_dir` generation will encounter concurrency problem when running `cargo test`
 ,to prevent that, u need to `cargo run` firstly to generate `output_dir`  and then run test.
* need to delete `server/output_dir` manually when config or python files changed.


## how to write  benchmarks
see `my_benchmark.rs` in benches folder. register your new file in Cargo.toml like below

```toml

[[bench]]
name = "my_benchmark"
harness = false
```

after that, run `cargo bench` then check the report html in `target/criterion/report/index.html`

## check docs before developing
[README.md](server/doc/README.md)


## pressure test using wrk
```bash
#brew install wrk 
# 10 thread, 20 connections, test 10 seconds.
wrk -t 10  -c 20  -d 10  http://127.0.0.1:3000
```