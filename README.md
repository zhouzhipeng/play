## Play
A playground project writen in rust for fun.


## crates
* prebuild : put your pre-build logic here.
* shared :  models and some common functions should be put here (should be simple)
* server : a http server providing APIs and static files and templates.
* server/python : for server side templates, we run a python interpreter inside rust.

## local debug
the `debug` feature will activate live-reload mode for `static` and `templates` folders.
```bash
cargo debug
```

## build python library
```bash
cargo python
```


## build binary
```bash
cargo dev
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
[README.md](doc/README.md)


## pressure test using wrk
```bash
#brew install wrk 
# 10 thread, 20 connections, test 10 seconds.
wrk -t 10  -c 20  -d 10  http://127.0.0.1:3000
```

## upload file to github release
```bash
gh release upload 1.0 play --clobber
```

## how to use `tool` crate
```bash
use `cargo tool` to see how many operations we have
```
> and if you have new tool binaries , define it in  `[alias]` block of `.cargo/config.toml`

## see all `cargo` alias commands.
[config.toml](.cargo%2Fconfig.toml)