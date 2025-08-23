# Play Dylib Plugin ABI

This directory contains the Application Binary Interface (ABI) definitions and examples for creating plugins in various programming languages.

## 📁 Directory Structure

```
play-dylib-abi/
├── src/              # Rust ABI implementation (core)
│   ├── lib.rs       # Main Rust ABI definitions
│   ├── http_abi.rs  # HTTP request/response handling
│   └── server_abi.rs # Server-mode plugin support
├── rust/            # Rust plugin examples
├── golang/          # Go plugin implementation
│   ├── golang_abi.go     # Go ABI bindings
│   ├── example_plugin.go # Example Go plugin
│   └── build_go_plugin.sh # Build script
├── c_cpp/           # C/C++ plugin implementation
│   ├── plugin_abi.h      # C/C++ header file
│   ├── example_plugin.c  # C example
│   ├── example_plugin.cpp # C++ example
│   ├── Makefile          # Unix build system
│   └── CMakeLists.txt    # CMake configuration
└── README_NEW_ARCHITECTURE.md # Architecture documentation
```

## 🏗️ New Architecture Overview

The plugin system uses a simple request ID pattern that eliminates complex FFI memory management:

1. **Plugin receives** only an `int64_t request_id`
2. **Fetches request** via HTTP GET from host
3. **Processes** the business logic
4. **Pushes response** via HTTP POST to host

### Benefits

- ✅ **No memory leaks** - No manual memory management across FFI
- ✅ **Language agnostic** - Any language that can make HTTP calls
- ✅ **Simple interface** - Just one function to implement
- ✅ **Better debugging** - HTTP endpoints can be tested directly
- ✅ **Plugin isolation** - Crashes don't affect host memory

## 🚀 Quick Start

### Rust Plugin

See [rust/README.md](rust/README.md) or check the [example](../play-dylib-example/src/lib.rs).

```rust
async fn handle_request_impl(request_id: i64) -> anyhow::Result<()> {
    let host_url = std::env::var("HOST")?;
    let request = HttpRequest::fetch_from_host(request_id, &host_url).await?;
    // Process request...
    response.push_to_host(request_id, &host_url).await?;
    Ok(())
}
```

### Go Plugin

See [golang/README.md](golang/README.md) for details.

```go
func HandleRequest(requestID int64) error {
    request, err := FetchRequestFromHost(requestID, hostURL)
    // Process request...
    return response.PushToHost(requestID, hostURL)
}
```

### C/C++ Plugin

See [c_cpp/README.md](c_cpp/README.md) for details.

```c
void handle_request(int64_t request_id) {
    HttpRequest* request = fetch_request(request_id, host_url);
    // Process request...
    push_response(request_id, host_url, response);
}
```

## 📋 Plugin Configuration

Add to your `config.toml`:

```toml
[[plugin_config]]
name = "my_plugin"
file_path = "/path/to/plugin.so"  # .dylib on macOS, .dll on Windows
url_prefix = "/my-plugin"
disable = false
```

## 🔧 Environment Variables

- `HOST` - The Play server URL (default: `http://127.0.0.1:3000`)

## 📚 Documentation

- [Architecture Details](README_NEW_ARCHITECTURE.md) - Deep dive into the new architecture
- [Rust Plugins](rust/README.md) - Rust-specific guide
- [Go Plugins](golang/README.md) - Go-specific guide
- [C/C++ Plugins](c_cpp/README.md) - C/C++ specific guide

## 🧪 Testing

1. Start the Play server
2. Build your plugin in the respective language directory
3. Configure the plugin in `config.toml`
4. Test with curl:

```bash
curl http://127.0.0.1:3000/my-plugin/endpoint
```

## 📦 Supported Languages

- ✅ Rust (native support)
- ✅ Go (via CGO)
- ✅ C/C++ (native)
- 🔜 Python (planned)
- 🔜 JavaScript/WebAssembly (planned)

## 🤝 Contributing

When adding support for a new language:

1. Create a new directory for the language
2. Implement the `handle_request(int64_t)` function
3. Add HTTP client for fetching/pushing data
4. Include examples and build scripts
5. Document in a language-specific README

## 📄 License

See the main project license.