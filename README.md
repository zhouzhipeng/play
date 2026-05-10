# Play Workspace

`play` is a Rust workspace built around `play-server`. The repository also includes desktop GUI tools, web terminal support, MCP integration, dynamic plugin loading, and shared utility crates.

## Workspace Structure

```text
play/
|- .cargo/
|- crates/
|  |- play-server/                         Main HTTP server, controllers, templates, and static assets
|  |- play-gui/                            eframe desktop toolbox shell
|  |  |- assets/
|  |  |- curl-helper/                      Standalone + embedded desktop curl tool
|  |  `- frp-client/                       Standalone + embedded desktop FRP client
|  |- play-terminal/                       Web terminal backend and static assets
|  |- play-db/                             Database helpers and asset/document storage helpers
|  |- play-shared/                         Shared models, constants, logging, and helpers
|  |- play-mcp/                            MCP registry, metadata loader, and tool definitions
|  |- play-integration/
|  |  `- play-integration-xiaozhi/         Xiaozhi integration crate
|  |- play-lua/                            Lua runtime and template integration
|  |- play-redis/                          Redis connection and pubsub helpers
|  |- play-https/                          HTTPS support crate
|  |- play-macros/                         Procedural macros
|  |- play-dylib/
|  |  |- play-dylib-abi/                   ABI shared by host and plugins
|  |  |- play-dylib-loader/                Runtime plugin loader
|  |  `- play-dylib-example/               Example dynamic plugin
|  `- play-utils/
|     |- play-utils-sql-util/              SQL helpers
|     |- play-utils-common-crypt/          Common cryptography helpers
|     |- play-utils-blockchain/            Bitcoin and Ethereum helpers
|     |- play-utils-data-api/              General data API helpers
|     `- play-utils-strings/               String utilities
|- docs/                                   API, plugin, and development docs
|- scripts/                                Build, cache, install, and packaging helpers
|- third_party/
|  `- rathole/                             Vendored FRP dependency
|- Cargo.toml                              Workspace manifest
`- README.md
```

## Crate Map

### Applications

| Crate | Path | Purpose |
| --- | --- | --- |
| `play-server` | `crates/play-server` | Main backend service with controllers, APIs, templates, static pages, and optional FRP/IKEv2 support |
| `play-gui` | `crates/play-gui` | Desktop toolbox shell built with `eframe` |
| `curl-helper` | `crates/play-gui/curl-helper` | Desktop curl request organizer and runner |
| `frp-client` | `crates/play-gui/frp-client` | Desktop FRP client powered by the vendored `rathole` crate |

### Core Libraries

| Crate | Path | Purpose |
| --- | --- | --- |
| `play-terminal` | `crates/play-terminal` | Web terminal backend and session management |
| `play-db` | `crates/play-db` | SQLite-oriented helpers for assets, docs, and KV storage |
| `play-shared` | `crates/play-shared` | Shared models, constants, logging setup, and cross-crate helpers |
| `play-mcp` | `crates/play-mcp` | MCP registry, metadata loading, and tool definitions |
| `play-lua` | `crates/play-lua` | Lua runtime integration |
| `play-redis` | `crates/play-redis` | Redis connection, pubsub, and error helpers |
| `play-https` | `crates/play-https` | HTTPS support helpers |
| `play-macros` | `crates/play-macros` | Procedural macros used across the workspace |

### Plugins and Integrations

| Crate | Path | Purpose |
| --- | --- | --- |
| `play-integration-xiaozhi` | `crates/play-integration/play-integration-xiaozhi` | Xiaozhi integration built on top of `play-mcp` |
| `play-dylib-abi` | `crates/play-dylib/play-dylib-abi` | Stable ABI definitions for host/plugin communication |
| `play-dylib-loader` | `crates/play-dylib/play-dylib-loader` | Dynamic plugin loading runtime |
| `play-dylib-example` | `crates/play-dylib/play-dylib-example` | Example plugin implementation |

### Utility Libraries

| Crate | Path | Purpose |
| --- | --- | --- |
| `play-utils-sql-util` | `crates/play-utils/play-utils-sql-util` | SQL helpers |
| `play-utils-common-crypt` | `crates/play-utils/play-utils-common-crypt` | Common cryptography helpers |
| `play-utils-blockchain` | `crates/play-utils/play-utils-blockchain` | Bitcoin and Ethereum helpers |
| `play-utils-data-api` | `crates/play-utils/play-utils-data-api` | General data API helpers |
| `play-utils-strings` | `crates/play-utils/play-utils-strings` | String utilities |

## Cargo Features

`play-server` currently exposes these notable features:

- `server`: enables the full server bundle, including plugin loading, Lua, Redis, FRP server support, and IKEv2 support.
- `frp-server`: enables embedded FRP support through the vendored `third_party/rathole` crate.
- `ikev2-server`: enables IKEv2 runtime integration.
- `debug`: convenience development feature that currently enables `play-lua` and `frp-server`.
- `use_mysql`: enables MySQL support in `sqlx` and SQL parser support in `play-shared`.

## Common Commands

### Build

```bash
cargo build
cargo build --workspace
```

### Run the main server

```bash
cargo run -p play-server
```

### Run the main server with optional features

```bash
cargo run -p play-server --features server
cargo run -p play-server --features frp-server
cargo run -p play-server --features ikev2-server
```

### Run the desktop tools

```bash
cargo run -p play-gui
cargo run -p curl-helper
cargo run -p frp-client
```

### Release build aliases

Defined in `.cargo/config.toml`:

```bash
cargo dev_server   # build --locked --package play-server --release --features=server
cargo dev_gui      # build --locked --package play-gui --release
```

## Workspace Notes

- The default workspace member is `crates/play-server`.
- `play-gui` hosts `curl-helper` and `frp-client` in-process, but both tools remain runnable as standalone binaries.
- `play-server` templates live under `crates/play-server/src/controller/templates/`.
- `play-server` static assets live under `crates/play-server/static/`.
- FRP support in both server and client code is backed by `third_party/rathole`.

## Documentation

- [Documentation Index](docs/README.md)
- [Quick Development Guide](docs/quick_dev.md)
- [Plugin Development](docs/plugin-dev.md)
- [General Data API](docs/general-data-api.md)
- [API v4 English](docs/api-v4-doc-en.md)
- [API v4 Chinese](docs/api-v4-doc-cn.md)
- [Dynamic Library Usage](docs/play-dylib-usage-cn.md)

## Deployment and Packaging

### Linux service install

The service install helper downloads the latest published Linux binary from GitHub releases and registers it as a `systemd` service:

```bash
bash <(curl -Ls https://raw.githubusercontent.com/zhouzhipeng/play/main/scripts/install_service.sh)
```

### Container and packaging scripts

Container and packaging recipes live under `scripts/dockerfiles/`. For example, `scripts/dockerfiles/bin.Dockerfile` builds a release `play-server` binary inside the cache image.
