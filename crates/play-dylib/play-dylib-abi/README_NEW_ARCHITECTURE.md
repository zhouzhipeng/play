# Play Dylib ABI - New Architecture

## Overview

The new architecture eliminates complex FFI memory management by using a simple request ID pattern. Instead of passing serialized data through FFI boundaries, plugins now:

1. Receive only a `request_id` (int64)
2. Fetch request data via HTTP GET
3. Process the request
4. Push response back via HTTP POST

## Architecture Benefits

### Memory Safety
- **No memory leaks**: No manual memory management required
- **No pointer passing**: Only simple int64 values cross FFI boundary
- **Automatic cleanup**: Request/response data automatically cleaned up after processing
- **No ABI compatibility issues**: Simple types ensure cross-language compatibility

### Robustness
- **Plugin isolation**: Plugin crashes don't affect host memory
- **Timeout protection**: Automatic cleanup on timeout
- **Easier debugging**: HTTP endpoints allow direct inspection of data

## Implementation Guide

### Rust Plugin

```rust
use play_dylib_abi::*;
use serde_json::json;

async fn handle_request_impl(request_id: i64) -> anyhow::Result<()> {
    // Get host URL
    let host_url = std::env::var("HOST").unwrap_or_else(|_| "http://127.0.0.1:3000".to_string());
    
    // Fetch request
    let request = HttpRequest::fetch_from_host(request_id, &host_url).await?;
    
    // Process request
    let response = HttpResponse::json(&json!({
        "message": "Hello from Rust plugin",
        "request_id": request_id
    }));
    
    // Push response
    response.push_to_host(request_id, &host_url).await?;
    
    Ok(())
}

// Use the macro to generate the export
async_request_handler!(handle_request_impl);
```

### Go Plugin

```go
func HandleRequest(requestID int64) error {
    hostURL := GetHostURL()
    
    // Fetch request
    request, err := FetchRequestFromHost(requestID, hostURL)
    if err != nil {
        return err
    }
    
    // Process request
    response := NewHttpResponse()
    response.StatusCode = 200
    response.Body = []byte("Hello from Go plugin")
    
    // Push response
    return response.PushToHost(requestID, hostURL)
}
```

## API Endpoints

### Get Request Info
```
GET /admin/get-request-info?request_id={request_id}
```
Returns the HttpRequest JSON for the given request_id.

### Push Response Info
```
POST /admin/push-response-info?request_id={request_id}
Body: HttpResponse JSON
```
Stores the response for the given request_id.

## Data Flow

```
1. Client Request → Host Server
2. Host generates unique request_id
3. Host stores request in memory (DashMap)
4. Host calls plugin.handle_request(request_id)
5. Plugin fetches request via HTTP GET
6. Plugin processes business logic
7. Plugin pushes response via HTTP POST
8. Host retrieves response from memory
9. Host returns response to client
10. Host cleans up request/response from memory
```

## Building Plugins

### Rust
```bash
cargo build --release
```

### Go
```bash
# macOS
go build -buildmode=c-shared -o plugin.dylib plugin.go

# Linux
go build -buildmode=c-shared -o plugin.so plugin.go
```

## Configuration

Set the HOST environment variable:
```bash
export HOST=http://127.0.0.1:3000
```

## Migration from Old Architecture

### Old Pattern
```rust
fn handle_request(request: HttpRequest) -> HttpResponse {
    // Direct processing
}
```

### New Pattern
```rust
async fn handle_request(request_id: i64) -> Result<()> {
    // Fetch, process, push
}
```

## Error Handling

The new architecture provides better error handling:
- Network errors during fetch/push are properly propagated
- Timeouts are enforced at the host level
- Plugin panics don't corrupt host memory

## Performance Considerations

While the new architecture adds HTTP overhead, it provides:
- Better reliability and safety
- Support for distributed deployment
- Language-agnostic plugin development
- Easier debugging and monitoring

For high-performance scenarios, consider:
- Batching requests
- Using connection pooling
- Implementing caching where appropriate