## Play
A playground project writen in rust for fun.

## install as linux service
```bash
curl -sSL https://raw.githubusercontent.com/zhouzhipeng/play/main/scripts/install_service.sh | sudo bash
```

## plugin development
[plugin-dev.md](docs/plugin-dev.md)


## general data api v2
* [English Doc](docs/api-v2-doc-en.md)
* [中文文档](docs/api-v2-doc-cn.md)

## crates
* tool : put your pre-build logic here.
* server : a http server providing APIs and static files and templates.

## local debug
the `debug` feature will activate live-reload mode for `static` and `templates` folders.
```bash
cargo debug
```


