## Play
A playground project writen in rust for fun.

## install as linux service
```bash
curl -sSL https://raw.githubusercontent.com/zhouzhipeng/play/main/scripts/install_service.sh | sudo bash
```

## plugin development
[plugin-dev.md](docs/plugin-dev.md)


## general data api
[genera-data-api.md](doc%2Fgenera-data-api.md)

## crates
* tool : put your pre-build logic here.
* server : a http server providing APIs and static files and templates.

## local debug
the `debug` feature will activate live-reload mode for `static` and `templates` folders.
```bash
cargo debug
```



## branches

`dev2`: working for clean minimal server
`dev3`: regular developing
`master`: used for CICD

