## Play
A playground project writen in rust for run.

## build
```bash
## run  `cargo build ` firstly because we need to generate wasm files (which will cause deadlock in --release mode)
# dev (default)
cargo clean && cargo build && cargo build --release

# prod
cargo clean &&  cargo build &&  ENV=prod  cargo build --release
```