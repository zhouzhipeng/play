# play-integration-xiaozhi

Xiaozhi AI MCP (Model Context Protocol) integration client for the Play framework.

## Overview

This crate provides a specialized MCP client for integrating with Xiaozhi AI services. It builds on top of `play-mcp` to provide:

- WebSocket-based MCP communication with Xiaozhi
- Automatic tool registration and management
- Retry logic and connection resilience
- Session management
- Easy-to-use builder pattern API

## Features

- **Full MCP Support**: Implements the complete MCP specification for tool communication
- **Tool Integration**: Automatically registers and exposes tools from `play-mcp`
- **Resilient Connection**: Built-in retry logic with configurable parameters
- **Flexible Configuration**: Builder pattern for easy customization
- **Async/Await**: Fully async implementation using Tokio

## Usage

### Quick Start

```rust
use play_integration_xiaozhi::quick_start;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Connect to Xiaozhi with default settings
    quick_start("ws://localhost:5173/ws").await?;
    Ok(())
}
```

### Custom Configuration

```rust
use play_integration_xiaozhi::XiaozhiClient;
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = XiaozhiClient::builder()
        .name("my-client")
        .version("1.0.0")
        .description("My custom Xiaozhi client")
        .url("ws://xiaozhi.example.com/ws")
        .tool_prefix("myapp:")  // Prefix all tool names
        .retry(true)
        .max_attempts(10)
        .retry_interval(Duration::from_secs(5))
        .build();
    
    client.start().await?;
    Ok(())
}
```

### With Custom Tools

```rust
use play_integration_xiaozhi::XiaozhiClient;
use play_mcp::tools::ToolRegistry;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut registry = ToolRegistry::new();
    // Add your custom tools to registry
    // registry.register(my_custom_tool);
    
    let client = XiaozhiClient::builder()
        .with_tools(registry)
        .build();
    
    client.start().await?;
    Ok(())
}
```

## Configuration Options

- **name**: Client name reported to Xiaozhi
- **version**: Client version string
- **description**: Human-readable description
- **url**: WebSocket URL for Xiaozhi server
- **tool_prefix**: Optional prefix for all tool names
- **retry**: Enable/disable automatic reconnection
- **max_attempts**: Maximum number of retry attempts (0 = unlimited)
- **retry_interval**: Time to wait between reconnection attempts

## Protocol Flow

1. Client connects to Xiaozhi via WebSocket
2. Xiaozhi sends `initialize` request
3. Client responds with capabilities and protocol version
4. Xiaozhi requests tool list via `tools/list`
5. Client provides available tools
6. Xiaozhi can then call tools via `tools/call`
7. Client executes tools and returns results

## Dependencies

This crate depends on:
- `play-mcp`: Core MCP types and tool definitions
- `tokio`: Async runtime
- `tokio-tungstenite`: WebSocket implementation
- `anyhow`: Error handling
- `serde`/`serde_json`: JSON serialization
- `tracing`: Logging

## Examples

See the `examples/` directory for complete working examples:
- `xiaozhi_client.rs`: Basic client with various configuration options

Run examples with:
```bash
cargo run --example xiaozhi_client
```

## License

Same as the Play framework.