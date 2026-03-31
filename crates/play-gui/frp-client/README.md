# frp-client

`frp-client` is a desktop tool under `play-gui` for editing and running a `rathole` client configuration.

It can run standalone with `cargo run -p frp-client`, and it is also embedded directly inside `play-gui`.

It uses the same TOML format as upstream `rathole`.

Startup behavior:

- When `play-gui` is launched from the OS login startup path and `frp-client` is the configured default tool, the client starts automatically after opening.
- When running `frp-client` standalone, pass `--auto-start` to start the client immediately.
