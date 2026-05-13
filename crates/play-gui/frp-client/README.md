# frp-client

`frp-client` is a tool under `play-gui` for editing and running a `rathole`
client configuration. It ships two build modes controlled by the `gui` cargo
feature:

| Mode     | Feature flag               | Target platforms                |
|----------|----------------------------|---------------------------------|
| Desktop  | `gui` (default)            | macOS, Windows, Linux w/ window |
| Headless | `--no-default-features`    | Headless Linux, OpenWrt, servers|

It uses the same TOML format as upstream `rathole`.

## Desktop (GUI) mode

```bash
cargo run -p frp-client
```

Startup behavior:

- When `play-gui` is launched from the OS login startup path and `frp-client`
  is the configured default tool, the client starts automatically after
  opening.
- When running `frp-client` standalone, pass `--auto-start` to start the
  client immediately.

## Headless mode (OpenWrt, Linux servers)

Build without default features to drop the `eframe` / `egui` / `directories`
dependency and produce a small CLI binary:

```bash
cargo build -p frp-client --no-default-features --release
```

Usage:

```
frp-client [OPTIONS]

OPTIONS:
    -c, --config <PATH>   Path to the rathole client TOML config
                          (default: /etc/frp-client.toml on Unix)
    -h, --help            Print help
    -V, --version         Print version
```

The process runs until it receives `SIGINT` or `SIGTERM`, so it integrates
cleanly with OpenWrt `procd` or systemd.

### Cross-compiling for OpenWrt

OpenWrt routers typically use a musl-libc target. Pick the triple that
matches your device's CPU, e.g.:

- `aarch64-unknown-linux-musl`  — most modern ARM64 routers
- `armv7-unknown-linux-musleabihf` — 32-bit ARM (e.g. many Xiaomi / GL.iNet)
- `mips-unknown-linux-musl` / `mipsel-unknown-linux-musl` — older MIPS SoCs

Example with [`cross`](https://github.com/cross-rs/cross):

```bash
rustup target add aarch64-unknown-linux-musl
cross build -p frp-client --no-default-features --release \
    --target aarch64-unknown-linux-musl
```

The resulting binary is a self-contained static executable at
`target/aarch64-unknown-linux-musl/release/frp-client`. Copy it to the device
(e.g. `/usr/sbin/frp-client`) together with a config file
(`/etc/frp-client.toml`).

### Example OpenWrt `procd` init script

Save as `/etc/init.d/frp-client`, `chmod +x`, then `/etc/init.d/frp-client enable`:

```sh
#!/bin/sh /etc/rc.common
USE_PROCD=1
START=95
STOP=10

PROG=/usr/sbin/frp-client
CONF=/etc/frp-client.toml

start_service() {
    procd_open_instance
    procd_set_param command "$PROG" --config "$CONF"
    procd_set_param respawn 3600 5 0
    procd_set_param stdout 1
    procd_set_param stderr 1
    procd_close_instance
}
```

Then:

```sh
/etc/init.d/frp-client start
logread -e frp-client
```
