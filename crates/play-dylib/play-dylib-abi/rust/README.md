# Rust Plugin Development

Rust plugins use the native ABI implementation from the parent crate.

## Quick Start

### 1. Create a New Plugin Project

```bash
cargo new --lib my_plugin
cd my_plugin
```

### 2. Configure Cargo.toml

```toml
[package]
name = "my_plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]  # Important: must be cdylib for plugins

[dependencies]
play-dylib-abi = { path = "../path/to/play-dylib-abi" }
anyhow = { version = "1.0" }
tokio = { version = "1", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = { version = "1.0" }
reqwest = { version = "0.12", features = ["json"] }
```

### 3. Implement Your Plugin

```rust
use play_dylib_abi::*;
use serde_json::json;

// Your handler function
async fn handle_request_impl(request_id: i64) -> anyhow::Result<()> {
    println!("Handling request: {}", request_id);
    
    // Get host URL from environment
    let host_url = std::env::var("HOST")
        .unwrap_or_else(|_| "http://127.0.0.1:3000".to_string());
    
    // Fetch request from host
    let request = HttpRequest::fetch_from_host(request_id, &host_url).await?;
    println!("Request: {:?}", request);
    
    // Process request based on URL
    let response = match request.url.as_str() {
        "/plugin/hello" => {
            HttpResponse::text("Hello from Rust plugin!")
        }
        "/plugin/json" => {
            HttpResponse::json(&json!({
                "message": "JSON response",
                "request_id": request_id,
                "method": format!("{:?}", request.method)
            }))
        }
        "/plugin/echo" => {
            HttpResponse::text(&request.body)
        }
        _ => {
            HttpResponse::page_404()
        }
    };
    
    // Push response back to host
    response.push_to_host(request_id, &host_url).await?;
    println!("Response sent for request: {}", request_id);
    
    Ok(())
}

// Use the macro to generate the export
async_request_handler!(handle_request_impl);
```

### 4. Build the Plugin

```bash
cargo build --release
```

The plugin will be at `target/release/libmy_plugin.dylib` (macOS) or `.so` (Linux) or `.dll` (Windows).

### 5. Configure in Play Server

```toml
[[plugin_config]]
name = "my_plugin"
file_path = "/path/to/target/release/libmy_plugin.so"
url_prefix = "/plugin"
disable = false
```

## API Reference

### Macros

#### `async_request_handler!(handler_fn)`
Generates the FFI export for async handlers.

#### `request_handler!(handler_fn)`
Generates the FFI export for sync handlers (not recommended).

#### `async_run!(server_fn)`
For long-running server plugins.

### Request/Response Types

#### HttpRequest
```rust
pub struct HttpRequest {
    pub method: HttpMethod,
    pub headers: HashMap<String, String>,
    pub query: String,
    pub url: String,
    pub body: String,
    pub context: HostContext,
}

impl HttpRequest {
    // Fetch from host
    pub async fn fetch_from_host(request_id: i64, host_url: &str) -> Result<Self>
    
    // Parse helpers
    pub fn parse_query<T: DeserializeOwned>(&self) -> Result<T>
    pub fn parse_body_form<T: DeserializeOwned>(&self) -> Result<T>
    pub fn parse_body_json<T: DeserializeOwned>(&self) -> Result<T>
    
    // URL matching
    pub fn get_suffix_url(&self) -> String
    pub fn match_suffix(&self, suffix: &str) -> bool
}
```

#### HttpResponse
```rust
pub struct HttpResponse {
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    pub status_code: u16,
    pub error: Option<String>,
}

impl HttpResponse {
    // Constructors
    pub fn text(body: &str) -> Self
    pub fn html(body: &str) -> Self
    pub fn json<T: Serialize>(body: &T) -> Self
    pub fn bytes(body: &[u8], content_type: &str) -> Self
    pub fn page_404() -> Self
    
    // Send to host
    pub async fn push_to_host(&self, request_id: i64, host_url: &str) -> Result<()>
}
```

## Advanced Examples

### Database Access
```rust
use sqlx::SqlitePool;

async fn handle_request_impl(request_id: i64) -> anyhow::Result<()> {
    let host_url = std::env::var("HOST")?;
    let request = HttpRequest::fetch_from_host(request_id, &host_url).await?;
    
    // Connect to database
    let pool = SqlitePool::connect("sqlite:data.db").await?;
    
    // Query data
    let result: (i32,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(&pool)
        .await?;
    
    let response = HttpResponse::json(&json!({
        "user_count": result.0
    }));
    
    response.push_to_host(request_id, &host_url).await?;
    Ok(())
}
```

### File Upload Processing
```rust
async fn handle_request_impl(request_id: i64) -> anyhow::Result<()> {
    let host_url = std::env::var("HOST")?;
    let request = HttpRequest::fetch_from_host(request_id, &host_url).await?;
    
    if request.match_suffix("/upload") {
        // Parse multipart data
        let boundary = extract_boundary(&request.headers)?;
        let data = parse_multipart(&request.body, boundary)?;
        
        // Process uploaded file
        for (name, content) in data {
            if name == "file" {
                // Save or process file
                std::fs::write(format!("uploads/{}", name), content)?;
            }
        }
        
        let response = HttpResponse::json(&json!({
            "status": "uploaded"
        }));
        
        response.push_to_host(request_id, &host_url).await?;
    }
    
    Ok(())
}
```

### Template Rendering
```rust
async fn handle_request_impl(request_id: i64) -> anyhow::Result<()> {
    let host_url = std::env::var("HOST")?;
    let request = HttpRequest::fetch_from_host(request_id, &host_url).await?;
    
    // Use host's template engine
    let html = request.render_template(
        "<h1>Hello {{name}}</h1>",
        json!({"name": "World"})
    ).await?;
    
    let response = HttpResponse::html(&html);
    response.push_to_host(request_id, &host_url).await?;
    
    Ok(())
}
```

## Error Handling

Errors are automatically caught and returned to the host:

```rust
async fn handle_request_impl(request_id: i64) -> anyhow::Result<()> {
    // This will be caught and sent as error response
    let data = std::fs::read_to_string("missing.txt")?;
    
    // Or handle explicitly
    let result = risky_operation();
    if let Err(e) = result {
        let response = HttpResponse {
            status_code: 500,
            error: Some(format!("Operation failed: {}", e)),
            ..Default::default()
        };
        response.push_to_host(request_id, &host_url).await?;
        return Ok(());
    }
    
    Ok(())
}
```

## Testing

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_handler() {
        // Mock request
        let request = HttpRequest {
            url: "/test".to_string(),
            ..Default::default()
        };
        
        // Test processing logic
        let response = process_request(request);
        assert_eq!(response.status_code, 200);
    }
}
```

### Integration Tests
```rust
#[tokio::test]
async fn test_with_server() {
    // Start test server
    let server = TestServer::new().await;
    
    // Store test request
    let request_id = 123;
    server.store_request(request_id, test_request()).await;
    
    // Call handler
    handle_request_impl(request_id).await.unwrap();
    
    // Check response
    let response = server.get_response(request_id).await;
    assert_eq!(response.status_code, 200);
}
```

## Performance Tips

1. **Use async/await** - Non-blocking I/O for better performance
2. **Connection pooling** - Reuse database/HTTP connections
3. **Caching** - Cache frequently accessed data
4. **Batch operations** - Process multiple items together
5. **Lazy initialization** - Initialize resources once

## Debugging

### Enable Logging
```rust
use log::info;

async fn handle_request_impl(request_id: i64) -> anyhow::Result<()> {
    info!("Processing request: {}", request_id);
    // ...
}
```

Set log level:
```bash
RUST_LOG=debug cargo run
```

### Panic Handling
The macro automatically catches panics:
```rust
// This panic will be caught and reported
panic!("Something went wrong!");
```

## Common Patterns

### Router Pattern
```rust
async fn handle_request_impl(request_id: i64) -> anyhow::Result<()> {
    let host_url = std::env::var("HOST")?;
    let request = HttpRequest::fetch_from_host(request_id, &host_url).await?;
    
    let response = match request.get_suffix_url().as_str() {
        "/users" => handle_users(request).await?,
        "/posts" => handle_posts(request).await?,
        "/admin" if is_admin(&request) => handle_admin(request).await?,
        _ => HttpResponse::page_404(),
    };
    
    response.push_to_host(request_id, &host_url).await?;
    Ok(())
}
```

### Middleware Pattern
```rust
async fn handle_request_impl(request_id: i64) -> anyhow::Result<()> {
    let host_url = std::env::var("HOST")?;
    let request = HttpRequest::fetch_from_host(request_id, &host_url).await?;
    
    // Authentication middleware
    if !is_authenticated(&request) {
        let response = HttpResponse {
            status_code: 401,
            body: b"Unauthorized".to_vec(),
            ..Default::default()
        };
        return response.push_to_host(request_id, &host_url).await;
    }
    
    // Process request
    let response = process_authenticated_request(request).await?;
    response.push_to_host(request_id, &host_url).await?;
    Ok(())
}
```

## See Also

- [Example Plugin](../../play-dylib-example/src/lib.rs) - Full example implementation
- [ABI Documentation](../README_NEW_ARCHITECTURE.md) - Architecture details
- [Main README](../README.md) - Overview of all language bindings