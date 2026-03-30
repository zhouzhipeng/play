# Play Workspace

`play` is now a Rust workspace, not just a single server crate.

The repo is centered around `play-server`, plus a growing set of desktop tools, MCP integration crates, shared libraries, dynamic-loading support, and utility crates.

## Current Workspace Layout

```text
play/
├── crates/
│   ├── play-server/                         # Main HTTP server and static assets
│   ├── play-gui/                            # Pure egui toolbox shell
│   │   ├── src/
│   │   ├── assets/
│   │   ├── curl-helper/                     # Embedded desktop curl manager tool
│   │   └── frp-client/                      # Embedded desktop FRP client tool
│   ├── play-mcp/                            # MCP tool registry and tool definitions
│   ├── play-integration/
│   │   └── play-integration-xiaozhi/        # Xiaozhi MCP client integration
│   ├── play-terminal/                       # Web terminal support
│   ├── play-db/                             # Database helpers
│   ├── play-shared/                         # Shared types, constants, helpers
│   ├── play-lua/                            # Lua integration
│   ├── play-redis/                          # Redis integration
│   ├── play-https/                          # HTTPS support
│   ├── play-macros/                         # Proc macros
│   ├── play-dylib/
│   │   ├── play-dylib-abi/                  # ABI shared by host/plugins
│   │   ├── play-dylib-loader/               # Dynamic plugin loader
│   │   └── play-dylib-example/              # Example dylib plugin
│   └── play-utils/
│       ├── play-utils-sql-util/
│       ├── play-utils-common-crypt/
│       ├── play-utils-blockchain/
│       ├── play-utils-data-api/
│       └── play-utils-strings/
├── docs/
├── scripts/
├── Cargo.toml
└── README.md
```

## Key Crates

- `play-server`
  Main backend service. Provides the data APIs, file APIs, admin/static pages, plugin loading hooks, MCP HTTP endpoint support, and optional embedded FRP server support behind the `frp-server` feature.

- `play-gui`
  Pure `egui` desktop toolbox shell. It hosts local tools in-process from library crates instead of spawning child processes.

- `curl-helper`
  Desktop tool nested under `crates/play-gui/curl-helper`. Used to organize and run curl commands locally.

- `frp-client`
  Desktop tool nested under `crates/play-gui/frp-client`. Used to edit and run a `rathole` client config from the toolbox.

- `play-mcp`
  MCP tool registry and tool-definition crate. Still used by `play-server` and `play-integration-xiaozhi`.

- `play-integration-xiaozhi`
  Xiaozhi-side MCP client integration. Uses `play-mcp::ToolRegistry` to expose tools remotely.

- `play-dylib-loader` and `play-dylib-abi`
  Runtime plugin loading infrastructure for server-side extensions.

- `play-shared`
  Shared constants, helpers, and cross-crate common code.

## Development Commands

### Build the whole workspace

```bash
cargo build
```

### Run the main server

```bash
cargo run -p play-server
```

### Run the main server with embedded FRP server support compiled in

```bash
cargo run -p play-server --features frp-server
```

### Run the desktop toolbox

```bash
cargo run -p play-gui
```

### Run the curl desktop tool directly

```bash
cargo run -p curl-helper
```

### Run the FRP desktop tool directly

```bash
cargo run -p frp-client
```

### Release build aliases

Defined in `.cargo/config.toml`:

```bash
cargo dev_server   # release build for play-server with server features
cargo dev_gui      # release build for play-gui and its embedded tool libraries
```

## FRP Usage

### 1. Enable the embedded FRP server

Put the FRP server settings directly in `DATA_DIR/config.toml`:

```toml
[frp_server]
enabled = true
bind_addr = "0.0.0.0:2333"
default_token = "change_this_token"
heartbeat_interval = 30

[frp_server.services.demo_http]
bind_addr = "0.0.0.0:8081"
```

Then start `play-server` with the feature enabled:

```bash
cargo run -p play-server --features frp-server
```

This means:

- `2333` is the FRP control port the client connects to.
- `8081` is the public port exposed by the FRP server.

### 2. Configure and start the FRP client

Open `play-gui`, launch `FRP Client`, then edit the client config.

Example client config:

```toml
[client]
remote_addr = "127.0.0.1:2333"
default_token = "change_this_token"

[client.services.demo_http]
local_addr = "127.0.0.1:3000"
```

This forwards the local service at `127.0.0.1:3000` through the FRP connection so it becomes reachable from the FRP server on port `8081`.

You can also run the client tool directly:

```bash
cargo run -p frp-client
```

### 3. Verify the tunnel

With the example configs above:

- local service: `127.0.0.1:3000`
- FRP server port: `127.0.0.1:2333`
- exposed tunnel port: `127.0.0.1:8081`

Once the client is connected, requests to `http://127.0.0.1:8081` should reach the local service behind the FRP client.

## Workspace Notes

- The default workspace member is `crates/play-server`.
- `play-ui` has been removed. Desktop entry now lives in `play-gui`.
- `play-gui` no longer spawns tool subprocesses. It opens embedded tools from linked libraries in the same `eframe` process.
- `play-gui` keeps the toolbox window open and opens tools in separate native windows.
- `play-gui` does not start or manage `play-server`.
- FRP server support is optional. Enable the `frp-server` cargo feature and configure `[frp_server]` in the root `config.toml`.
- FRP server settings now live entirely inside the main `config.toml`; there is no user-managed `frp/server.toml`.
- MCP support is still active in the workspace through `play-mcp` and the server's `/mcp` controller.

## Documentation

- [Quick Development Guide](docs/quick_dev.md)
- [API v4 English](docs/api-v4-doc-en.md)
- [API v4 中文文档](docs/api-v4-doc-cn.md)

## Deployment

### Linux service install

```bash
bash <(curl -Ls https://raw.githubusercontent.com/zhouzhipeng/play/main/scripts/install_service.sh)
```

### Docker

```bash
docker build -t play .
docker run -p 8080:8080 play
```
