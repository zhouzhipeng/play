# Migration Summary: play-mcp to play-integration-xiaozhi

## What Was Migrated

### From `play-mcp` to `play-integration-xiaozhi`:

1. **Protocol Types** (moved to `protocol.rs`):
   - `JsonRpcRequest`
   - `JsonRpcResponse` 
   - `JsonRpcError`

2. **Configuration Types** (moved to `config.rs`):
   - `McpConfig`
   - `ClientConfig`
   - `RetryConfig`
   - Added `McpConfigBuilder` for better ergonomics

3. **Client Logic** (in `lib.rs` and `client.rs`):
   - `start_xiaozhi_client()` (was `start_mcp_client`)
   - `start_xiaozhi_client_with_tools()` (was `start_mcp_client_with_tools`)
   - `handle_server_request()`
   - `run_mcp_connection()`
   - WebSocket connection management
   - Retry logic
   - Protocol handling

## What Remains in `play-mcp`

- `Tool` trait
- `ToolRegistry` 
- `ToolMetadata`
- `define_mcp_tool!` macro
- Tool validation (`metadata_loader`)
- Tool factories and registration system

## New Structure

```
play-mcp/                        # Tool definitions only
├── src/
│   ├── lib.rs                  # Minimal, exports tools module
│   ├── tools/
│   │   ├── mod.rs              # Tool trait, registry, metadata
│   │   └── http_request.rs    # Example tool
│   └── metadata_loader.rs     # Tool validation

play-integration-xiaozhi/       # MCP client and protocol
├── src/
│   ├── lib.rs                 # Main client logic
│   ├── client.rs              # Client builder and API
│   ├── config.rs              # Configuration types
│   ├── protocol.rs            # JSON-RPC protocol types
│   └── tests.rs               # Unit tests
└── examples/
    └── xiaozhi_client.rs      # Usage example
```

## Benefits of This Separation

1. **Clear Separation of Concerns**:
   - `play-mcp`: Pure tool definitions, no network/protocol code
   - `play-integration-xiaozhi`: All client/protocol/network logic

2. **Better Modularity**:
   - Tools can be used without client dependencies
   - Client can be updated without affecting tools
   - Different clients can be created for different MCP servers

3. **Reduced Dependencies**:
   - `play-mcp` no longer needs WebSocket/networking dependencies
   - Applications only include what they need

4. **Specialized Integration**:
   - `play-integration-xiaozhi` can be optimized for Xiaozhi AI
   - Other integration crates can be created for different MCP servers

## Migration Guide for Users

### If you were using tools only:
No changes needed - continue using `play-mcp` as before.

### If you were using the MCP client:

**Before:**
```rust
use play_mcp::{start_mcp_client, McpConfig};
```

**After:**
```rust
use play_integration_xiaozhi::{start_xiaozhi_client, McpConfig};
```

Or use the new builder pattern:
```rust
use play_integration_xiaozhi::XiaozhiClient;

let client = XiaozhiClient::builder()
    .url("ws://localhost:5173/ws")
    .build();
client.start().await?;
```

## Testing

All tests pass:
- `cargo test -p play-mcp` ✓
- `cargo test -p play-integration-xiaozhi` ✓
- `cargo build --example xiaozhi_client -p play-integration-xiaozhi` ✓

## Files Changed

### Removed from play-mcp:
- `src/config.rs` (moved to play-integration-xiaozhi)
- JSON-RPC types from `src/lib.rs`
- Client implementation code

### Added to play-integration-xiaozhi:
- `src/protocol.rs` - Protocol types
- `src/config.rs` - Configuration with builder
- `src/client.rs` - Client API
- All client logic from play-mcp

### Modified:
- `play-mcp/src/lib.rs` - Removed protocol/config exports
- `play-integration-xiaozhi/src/lib.rs` - Uses local types
- Both README files updated