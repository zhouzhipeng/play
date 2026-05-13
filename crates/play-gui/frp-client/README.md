# frp-client

`frp-client` is a tool under `play-gui` for editing and running a `rathole`
client configuration. It ships two build modes controlled by Cargo features:

| Mode     | Feature flag               | Target platforms                |
|----------|----------------------------|---------------------------------|
| Desktop  | `gui` (default)            | macOS, Windows, Linux w/ window |
| Headless | `--no-default-features`    | Headless Linux, OpenWrt, servers|

The default desktop build includes the `tls` feature for rustls and secure
websocket transports. The headless `--no-default-features` build keeps TCP,
noise, and hot-reload support, avoiding C cross-toolchain requirements on
embedded targets.

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

```text
frp-client [OPTIONS]

OPTIONS:
    -c, --config <PATH>   Path to the rathole client TOML config
                          (default: /etc/frp-client.toml on Unix)
    -h, --help            Print help
    -V, --version         Print version
```

The process runs until it receives `SIGINT` or `SIGTERM`, so it integrates
cleanly with OpenWrt `procd` or systemd.

### OpenWrt build and deploy runbook

The OpenWrt binary is normally cross-compiled on a development machine, then
copied to the router. The steps below use an ARM64 router as the concrete
example.

1. Identify the OpenWrt architecture:

   ```sh
   ssh root@192.168.10.1 'cat /etc/openwrt_release; uname -m; ls -l /lib/ld-musl-* 2>/dev/null'
   ```

   Common mappings:

   | OpenWrt arch / CPU         | Rust target                         |
   |----------------------------|-------------------------------------|
   | `aarch64_cortex-a53`       | `aarch64-unknown-linux-musl`        |
   | `aarch64_generic`          | `aarch64-unknown-linux-musl`        |
   | `arm_cortex-a7_neon-vfpv4` | `armv7-unknown-linux-musleabihf`    |
   | `mips_24kc`                | `mips-unknown-linux-musl`           |
   | `mipsel_24kc`              | `mipsel-unknown-linux-musl`         |
   | `x86_64`                   | `x86_64-unknown-linux-musl`         |

2. Install the Rust target. For an ARM64 OpenWrt router:

   ```sh
   rustup target add aarch64-unknown-linux-musl
   ```

3. Build the binary from the workspace root:

   ```sh
   cargo build -p frp-client --no-default-features --release \
       --target aarch64-unknown-linux-musl
   ```

   On Windows, if the build finishes compilation but fails with `linker "cc"
   not found`, use Rust's bundled LLD linker:

   ```powershell
   $env:CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER = "rust-lld"
   cargo build -p frp-client --no-default-features --release `
       --target aarch64-unknown-linux-musl
   ```

   The output path for this example is:

   ```text
   target/aarch64-unknown-linux-musl/release/frp-client
   ```

4. Prepare the rathole server config on the public server.

   `frp-client` uses upstream rathole TOML. If another wrapper uses a section
   named `[frp_server]`, map those fields to `[server]` for rathole itself:

   ```toml
   [server]
   bind_addr = "0.0.0.0:2333"
   default_token = "change_this_token"
   heartbeat_interval = 20

   [server.services.smb]
   bind_addr = "0.0.0.0:445"
   ```

   Windows Explorer SMB UNC paths do not support `host:port`, so use remote
   port `445` when you want to open the share as `\\ip.zhouzhipeng.com\share`.
   A non-standard port such as `11445` is still valid TCP, but Windows Explorer
   will not open it directly as `\\host:11445`.

5. Create the OpenWrt client config as `frp-client.toml`:

   ```toml
   [client]
   remote_addr = "ip.zhouzhipeng.com:2333"
   default_token = "change_this_token"
   heartbeat_timeout = 40

   [client.services.smb]
   local_addr = "192.168.10.1:445"
   ```

   The service name, `smb`, must match the server-side
   `[server.services.smb]` section. `heartbeat_timeout` should be greater than
   the server's `heartbeat_interval`.

6. Copy the binary and config to the router:

   ```sh
   scp target/aarch64-unknown-linux-musl/release/frp-client \
       root@192.168.10.1:/tmp/frp-client
   scp frp-client.toml root@192.168.10.1:/tmp/frp-client.toml
   ```

   Some OpenWrt/dropbear installations do not provide SFTP. If `scp` fails
   during SFTP negotiation, force legacy SCP mode:

   ```sh
   scp -O target/aarch64-unknown-linux-musl/release/frp-client \
       root@192.168.10.1:/tmp/frp-client
   scp -O frp-client.toml root@192.168.10.1:/tmp/frp-client.toml
   ```

7. Install the files on OpenWrt:

   ```sh
   ssh root@192.168.10.1

   cp /tmp/frp-client /usr/sbin/frp-client
   chmod 755 /usr/sbin/frp-client

   cp /tmp/frp-client.toml /etc/frp-client.toml
   chmod 600 /etc/frp-client.toml

   /usr/sbin/frp-client --version
   ```

8. Add the OpenWrt `procd` init script:

   ```sh
   cat >/etc/init.d/frp-client <<'EOF'
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
   EOF

   chmod 755 /etc/init.d/frp-client
   /etc/init.d/frp-client enable
   /etc/init.d/frp-client restart
   ```

9. Verify the router service:

   ```sh
   /etc/init.d/frp-client status
   pgrep -af frp-client
   ss -tnp | grep ':2333'
   logread -e frp-client
   ```

10. Verify the remote SMB exposure from Windows:

    ```powershell
    Test-NetConnection ip.zhouzhipeng.com -Port 445
    ```

    Then open the share with a normal UNC path:

    ```text
    \\ip.zhouzhipeng.com\share-name
    ```

### Upgrading an existing OpenWrt install

Copy the new binary to `/tmp/frp-client`, then replace and restart:

```sh
/etc/init.d/frp-client stop
cp /usr/sbin/frp-client /usr/sbin/frp-client.bak.$(date +%Y%m%d-%H%M%S)
cp /tmp/frp-client /usr/sbin/frp-client
chmod 755 /usr/sbin/frp-client
/etc/init.d/frp-client start
logread -e frp-client
```
