# Go Plugin Development

This directory contains the Go implementation for Play server plugins.

## Files

- `golang_abi.go` - Core ABI bindings and helper functions
- `example_plugin.go` - Example plugin implementation
- `build_go_plugin.sh` - Build script for Unix-like systems

## Quick Start

### 1. Implement Your Plugin

Create a new Go file implementing the `HandleRequest` function:

```go
package main

import "C"

func HandleRequest(requestID int64) error {
    hostURL := GetHostURL() // Gets from ENV or default
    
    // Fetch request
    request, err := FetchRequestFromHost(requestID, hostURL)
    if err != nil {
        return err
    }
    
    // Process request
    response := NewHttpResponse()
    response.StatusCode = 200
    response.Body = []byte("Hello from Go!")
    
    // Push response
    return response.PushToHost(requestID, hostURL)
}

func main() {
    // Required for shared library
}
```

### 2. Build the Plugin

Using the build script:
```bash
./build_go_plugin.sh
```

Or manually:
```bash
# macOS
go build -buildmode=c-shared -o myplugin.dylib myplugin.go golang_abi.go

# Linux
go build -buildmode=c-shared -o myplugin.so myplugin.go golang_abi.go

# Windows
go build -buildmode=c-shared -o myplugin.dll myplugin.go golang_abi.go
```

### 3. Configure in Play Server

Add to `config.toml`:
```toml
[[plugin_config]]
name = "go_plugin"
file_path = "/path/to/myplugin.so"
url_prefix = "/go-plugin"
```

## API Reference

### Core Functions

#### `HandleRequest(requestID int64) error`
Main entry point that you must implement.

#### `GetHostURL() string`
Returns the host URL from `HOST` environment variable or default.

#### `FetchRequestFromHost(requestID int64, hostURL string) (*HttpRequest, error)`
Fetches the request data from the host server.

#### `(r HttpResponse) PushToHost(requestID int64, hostURL string) error`
Pushes the response back to the host server.

### Data Structures

#### HttpRequest
```go
type HttpRequest struct {
    Method  HttpMethod        // GET, POST, PUT, DELETE
    Headers map[string]string
    Query   string           // Query string
    URL     string           // Request URL path
    Body    string           // Request body
    Context HostContext      // Host environment info
}
```

#### HttpResponse
```go
type HttpResponse struct {
    Headers    map[string]string
    Body       []byte
    StatusCode int
    Error      *string  // Optional error message
}
```

## Example Use Cases

### JSON API Endpoint
```go
func HandleRequest(requestID int64) error {
    // ... fetch request ...
    
    // Parse JSON body
    var data map[string]interface{}
    json.Unmarshal([]byte(request.Body), &data)
    
    // Create JSON response
    responseData := map[string]interface{}{
        "message": "Processed successfully",
        "input": data,
    }
    
    jsonBytes, _ := json.Marshal(responseData)
    response := NewHttpResponse()
    response.Body = jsonBytes
    response.Headers["Content-Type"] = "application/json"
    
    return response.PushToHost(requestID, hostURL)
}
```

### File Processing
```go
func HandleRequest(requestID int64) error {
    // ... fetch request ...
    
    // Process file path from query
    params := parseQuery(request.Query)
    filePath := params["file"]
    
    content, err := ioutil.ReadFile(filePath)
    if err != nil {
        response := ErrResponse(err)
        return response.PushToHost(requestID, hostURL)
    }
    
    response := NewHttpResponse()
    response.Body = content
    response.Headers["Content-Type"] = "application/octet-stream"
    
    return response.PushToHost(requestID, hostURL)
}
```

## Building Complex Plugins

For complex plugins with multiple files:

1. Create a directory for your plugin
2. Put all Go files in the directory
3. Build with all files:

```bash
go build -buildmode=c-shared -o plugin.so *.go
```

## Testing

Test your plugin locally:

```bash
# Set host URL
export HOST=http://127.0.0.1:3000

# Run Play server
# Configure plugin in config.toml

# Test endpoint
curl http://127.0.0.1:3000/go-plugin/test
```

## Debugging

1. Add logging in your HandleRequest function
2. Check Play server logs for errors
3. Test HTTP endpoints directly:

```bash
# Test fetching request (need valid request_id)
curl "http://127.0.0.1:3000/admin/get-request-info?request_id=123"
```

## Performance Tips

1. Reuse HTTP clients if making multiple requests
2. Use goroutines for parallel processing (carefully)
3. Consider caching for repeated operations

## Common Issues

### Plugin not loading
- Check Go version compatibility
- Verify CGO is enabled: `CGO_ENABLED=1`
- Check shared library dependencies

### HTTP errors
- Verify HOST environment variable
- Check network connectivity
- Ensure Play server is running

### Build errors
- Install Go 1.16 or later
- Ensure `golang_abi.go` is included in build