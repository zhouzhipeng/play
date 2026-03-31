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
  Main backend service. Provides the data APIs, file APIs, admin/static pages, plugin loading hooks, MCP HTTP endpoint support, and optional embedded FRP server and IKEv2 server support behind feature flags.

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

### Run the main server with embedded IKEv2 support compiled in

```bash
cargo run -p play-server --features ikev2-server
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

## IKEv2 Usage

### 1. Enable the embedded IKEv2 server

Put the IKEv2 settings in `DATA_DIR/config.toml`:

```toml
[ikev2_server]
enabled = true
auto_install_dependencies = true
local_id = "vpn.example.com"
pool = "10.10.10.0/24"
dns_servers = ["1.1.1.1", "8.8.8.8"]

[ikev2_server.eap_users]
demo = "change_this_password"
```

Then start `play-server` with the feature enabled:

```bash
cargo run -p play-server --features ikev2-server
```

On first start, if the configured IKEv2 certificate files are missing, `play-server` now generates them automatically under `DATA_DIR/certs/ikev2/`:

- `ca-cert.pem`
- `ca-key.pem`
- `server-cert.pem`
- `server-key.pem`

You do not need to create the default cert files by hand. The generated CA certificate is the one you distribute to clients so they can trust the VPN server certificate.

On Debian Bookworm, `play-server` can also install the required strongSwan runtime packages automatically when IKEv2 is enabled and the binaries are missing. With the default `auto_install_dependencies = true`, you do not need to preinstall `charon-systemd` or `swanctl` yourself.

### 2. Runtime requirements

- The embedded IKEv2 runtime is currently Linux-only.
- `play-server` must be built with the `ikev2-server` feature.
- On Debian Bookworm, `play-server` will automatically run `apt-get update` and install the required strongSwan packages if they are missing.
- The host still needs outbound network access for the first automatic install, and the `play-server` process needs enough privilege to run `apt-get` and bind UDP `500` and `4500`.
- After the packages are present, `play-server` tries `charon-systemd` first, then falls back to `charon`, and uses `swanctl` to load the generated config.
- If your distro installs the binaries in a non-standard location or under a different name, set `ikev2_server.daemon_bin` and `ikev2_server.swanctl_bin` explicitly in `config.toml`.
- During startup, `play-server` also best-effort disables system strongSwan services so they do not compete for the same UDP ports.

### 3. What gets started

When enabled, `play-server` generates a temporary strongSwan runtime directory, writes `strongswan.conf` and `swanctl.conf`, starts the strongSwan IKE daemon (`charon-systemd` or `charon`), then loads the generated connection through `swanctl`.

The default connection name is `play-ikev2`, the default UDP ports are `500` and `4500`, and the default address pool is `10.10.10.0/24`.

## Workspace Notes

- The default workspace member is `crates/play-server`.
- `play-ui` has been removed. Desktop entry now lives in `play-gui`.
- `play-gui` no longer spawns tool subprocesses. It opens embedded tools from linked libraries in the same `eframe` process.
- `play-gui` keeps the toolbox window open and opens tools in separate native windows.
- `play-gui` does not start or manage `play-server`.
- FRP server support is optional. Enable the `frp-server` cargo feature and configure `[frp_server]` in the root `config.toml`.
- FRP server settings now live entirely inside the main `config.toml`; there is no user-managed `frp/server.toml`.
- IKEv2 server support is optional. Enable the `ikev2-server` cargo feature and configure `[ikev2_server]` in the root `config.toml`.
- With the default certificate paths, `play-server` auto-generates the IKEv2 CA/server certificate bundle when the server starts and the files are missing.
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
