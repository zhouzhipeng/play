## Play
A playground project writen in rust for fun.


## about workspace
* shared :  models and some common functions should be put here (should be simple)
* client : built to wasm file and be copied to server folder , finally runs in browser
* server : a http server providing APIs and static files and templates.
* server/python : for server side templates, we run a python interpreter inside rust.

## local run
```bash
cargo run
```

## build binary
```bash
## run  `cargo build ` firstly because we need to generate wasm files (which will cause deadlock in --release mode)
# dev (default)
cargo clean && cargo build && cargo build --release

# prod
cargo clean &&  cargo build &&  cargo build --release  --no-default-features --features=prod
```

## running
put the final binary `play` on your server , and just run `./play` , everything is embed in it including config files.