## Play
A playground project writen in rust for fun.

## general data api
[genera-data-api.md](doc%2Fgenera-data-api.md)

## crates
* tool : put your pre-build logic here.
* libs :  a bunch of useful crates.
* server : a http server providing APIs and static files and templates.

## local debug
the `debug` feature will activate live-reload mode for `static` and `templates` folders.
```bash
cargo debug
```


## check docs before developing
[README.md](docs/README.md)


## build rust app into static linux executables
https://github.com/rust-cross/rust-musl-cross


## ports
```bash
3000 :  api server
25 : mail receive server
```

## install as linux service
```bash
curl -sSL https://raw.githubusercontent.com/zhouzhipeng/play/main/scripts/install_service.sh | sudo bash
```


## branches

`dev2`: working for clean minimal server
`dev3`: regular developing

## add patch for local dev
`vim ~/.cargo/config.toml`
```toml
[patch."https://github.com/zhouzhipeng/rust-utils"]
rust-utils = { path = "/Users/zhouzhipeng/RustroverProjects/rust-utils"}
```
