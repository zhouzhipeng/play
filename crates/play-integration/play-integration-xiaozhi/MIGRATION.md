# Migration Guide: From play-mcp to play-integration-xiaozhi

## Overview

The MCP client functionality has been extracted from `play-mcp` into a dedicated crate `play-integration-xiaozhi` for better separation of concerns and specialized Xiaozhi AI integration.

## What Changed

### play-mcp (Before)
- Contained both MCP tool definitions AND client implementation
- Mixed concerns between tool management and network communication
- Generic MCP client for any server

### New Architecture

#### play-mcp (Core)
- Focus on MCP protocol types and tool definitions
- Exports: `JsonRpcRequest`, `JsonRpcResponse`, `JsonRpcError`
- Exports: `McpConfig`, `ClientConfig`, `RetryConfig`
- Exports: `ToolRegistry` and tool management
- Provides `define_mcp_tool!` macro

#### play-integration-xiaozhi (Client)
- Specialized client for Xiaozhi AI integration
- Handles WebSocket connection and protocol flow
- Provides builder pattern for easy configuration
- Includes retry logic and session management

## Migration Steps

### 1. Update Dependencies

Replace in your `Cargo.toml`:
```toml
# Old
play-mcp = { path = "../play-mcp" }

# New (if you need the client)
play-mcp = { path = "../play-mcp" }  # Still needed for tools
play-integration-xiaozhi = { path = "../play-integration-xiaozhi" }
```

### 2. Update Imports

```rust
// Old
use play_mcp::{start_mcp_client, start_mcp_client_with_tools};

// New
use play_integration_xiaozhi::{start_xiaozhi_client, start_xiaozhi_client_with_tools};
// Or use the builder pattern:
use play_integration_xiaozhi::XiaozhiClient;
```

### 3. Update Function Calls

```rust
// Old
play_mcp::start_mcp_client(&config).await?;

// New - Option 1: Direct replacement
play_integration_xiaozhi::start_xiaozhi_client(&config).await?;

// New - Option 2: Quick start
play_integration_xiaozhi::quick_start("ws://localhost:5173/ws").await?;

// New - Option 3: Builder pattern (recommended)
let client = XiaozhiClient::builder()
    .url("ws://localhost:5173/ws")
    .name("my-client")
    .build();
client.start().await?;
```

## Benefits of the New Architecture

1. **Separation of Concerns**: Tools and client logic are now separate
2. **Specialized Integration**: Xiaozhi-specific features and optimizations
3. **Better API**: Builder pattern for cleaner configuration
4. **Maintainability**: Easier to update client without affecting tools
5. **Flexibility**: Can create other integration crates for different MCP servers

## Examples

See `/crates/play-integration-xiaozhi/examples/` for complete examples:
- `xiaozhi_client.rs`: Shows various configuration options

## Compatibility

- The MCP protocol implementation remains unchanged
- All existing tools continue to work
- Configuration format (`McpConfig`) is preserved
- Only the namespace changed from `play_mcp` to `play_integration_xiaozhi`