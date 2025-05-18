# Play Terminal

A web-based terminal implementation using Rust Axum 0.6, designed to be easily integrated into existing Axum web applications.

## Features

- 🖥️ Clean, modern terminal interface with XTerm.js
- 📱 Mobile-friendly responsive design
- 🌈 Multiple themes (dark, light, solarized)
- ⚙️ User-configurable settings (font size, theme, shell)
- 💾 Persistent settings using localStorage
- 🔄 Automatic reconnection
- 📦 Easy integration into existing Axum projects

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
play_terminal = "0.1.0"
```

## Usage

### Basic Example

```rust
use axum::{
    routing::get,
    Router,
};
use play_terminal::WebTerminal;
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    // Create a web terminal with default configuration
    let web_terminal = WebTerminal::default();

    // Create the application router
    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        // Nest the web terminal router under the /terminal path
        .nest("/terminal", web_terminal.router());

    // Run the server
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
```

### Custom Configuration

```rust
use play_terminal::{WebTerminal, config::TerminalConfig};

// Create a terminal with custom configuration
let config = TerminalConfig::default()
    .with_max_terminals(8)
    .with_timeout(300) // 5 minutes
    .with_base_path("/console");

let web_terminal = WebTerminal::new(config);
```

### Using Custom Static Assets

```rust
use play_terminal::web_terminal_with_assets;

// Create a terminal with custom static assets
let web_terminal = web_terminal_with_assets("./my-custom-terminal-ui")?;
```

## Configuration Options

| Option | Description | Default |
|--------|-------------|---------|
| `assets_dir` | Path to custom static assets | None (embedded assets) |
| `max_terminals_per_user` | Maximum number of terminals per user | 4 |
| `terminal_timeout_secs` | Timeout for inactive terminals (seconds) | 600 (10 minutes) |
| `base_path` | Base path for the terminal API | "/terminal" |
| `use_embedded_assets` | Whether to use embedded assets | true |

## Terminal Interface

The terminal interface provides the following features:

- **Terminal window** with standard input/output
- **Control buttons** for fullscreen, resize, settings, and close
- **Settings panel** for font size, theme, and shell configuration
- **Status bar** showing connection status and terminal dimensions

## Browser Compatibility

The web terminal should work in any modern browser that supports WebSockets and the Fetch API:

- Chrome/Edge (latest)
- Firefox (latest)
- Safari (latest)
- Mobile browsers (iOS Safari, Android Chrome)

## Security Considerations

This terminal provides shell access to the server. Consider the following security precautions:

- **Authentication**: Always implement proper authentication before allowing access to the terminal
- **Restricted shells**: Consider using restricted shells for untrusted users
- **HTTPS**: Always use HTTPS in production to protect WebSocket connections
- **Resource limits**: Use the timeout and max terminals settings to prevent resource exhaustion

## License

This project is licensed under either of:

- MIT License
- Apache License, Version 2.0

at your option.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.