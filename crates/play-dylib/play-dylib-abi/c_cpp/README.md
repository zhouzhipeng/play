# C/C++ Plugin Development

This directory contains C and C++ implementations for Play server plugins.

## Files

- `plugin_abi.h` - Header file with ABI definitions
- `example_plugin.c` - Complete C example
- `example_plugin.cpp` - Complete C++ example
- `Makefile` - Unix-like build system
- `CMakeLists.txt` - CMake build configuration

## Dependencies

### Required Libraries

#### C Plugin
- `libcurl` - HTTP client
- `json-c` - JSON parsing

#### C++ Plugin
- `libcurl` - HTTP client
- `jsoncpp` - JSON parsing

### Installing Dependencies

#### Ubuntu/Debian
```bash
sudo apt-get update
sudo apt-get install -y libcurl4-openssl-dev libjson-c-dev libjsoncpp-dev
```

#### macOS
```bash
brew install curl json-c jsoncpp
```

#### Windows
- [libcurl](https://curl.se/windows/)
- [json-c](https://github.com/json-c/json-c)
- [jsoncpp](https://github.com/open-source-parsers/jsoncpp)

## Quick Start

### 1. Implement Your Plugin

#### C Version
```c
#include "plugin_abi.h"
#include <curl/curl.h>
#include <json-c/json.h>

void handle_request(int64_t request_id) {
    const char* host_url = get_host_url();
    
    // Fetch request
    HttpRequest* request = fetch_request(request_id, host_url);
    
    // Process request
    HttpResponse response = {
        .status_code = 200,
        .body = "Hello from C!",
        .body_len = 13,
        .error = NULL
    };
    
    // Push response
    push_response(request_id, host_url, &response);
    
    // Cleanup
    free_request(request);
}
```

#### C++ Version
```cpp
#include "plugin_abi.h"
#include <string>
#include <curl/curl.h>
#include <json/json.h>

extern "C" void handle_request(int64_t request_id) {
    try {
        auto host_url = get_host_url();
        
        // Fetch and process request
        auto request = fetch_request(request_id, host_url);
        
        Response response;
        response.status_code = 200;
        response.setBody("Hello from C++!");
        
        response.pushToHost(request_id, host_url);
    }
    catch (const std::exception& e) {
        // Handle error
    }
}
```

### 2. Build the Plugin

#### Using Make
```bash
# Build C plugin
make c_plugin

# Build C++ plugin
make cpp_plugin

# Build both
make all
```

#### Using CMake
```bash
mkdir build && cd build
cmake ..
make
```

#### Manual Build

**C Plugin:**
```bash
# Linux
gcc -shared -fPIC -o plugin.so plugin.c -lcurl -ljson-c

# macOS
gcc -shared -o plugin.dylib plugin.c -lcurl -ljson-c

# Windows
cl /LD plugin.c /Fe:plugin.dll libcurl.lib json-c.lib
```

**C++ Plugin:**
```bash
# Linux
g++ -shared -fPIC -std=c++17 -o plugin.so plugin.cpp -lcurl -ljsoncpp

# macOS
g++ -shared -std=c++17 -o plugin.dylib plugin.cpp -lcurl -ljsoncpp

# Windows
cl /LD /std:c++17 plugin.cpp /Fe:plugin.dll libcurl.lib jsoncpp.lib
```

### 3. Configure in Play Server

```toml
[[plugin_config]]
name = "c_plugin"
file_path = "/path/to/plugin.so"
url_prefix = "/c-plugin"
```

## API Reference

### Core Function

```c
void handle_request(int64_t request_id);
```
The only function you must implement. It should:
1. Fetch request from host
2. Process the request
3. Push response back to host

### Helper Functions

```c
const char* get_host_url();
```
Returns host URL from `HOST` environment variable.

### Data Structures

#### HttpRequest
```c
typedef struct {
    HttpMethod method;      // GET, POST, PUT, DELETE
    const char* url;       // Request URL path
    const char* query;     // Query string
    const char* body;      // Request body
    HostContext context;   // Host environment
} HttpRequest;
```

#### HttpResponse
```c
typedef struct {
    uint16_t status_code;  // HTTP status code
    const char* body;      // Response body
    size_t body_len;       // Body length
    const char* error;     // Optional error (NULL for success)
} HttpResponse;
```

## Advanced Examples

### JSON Processing (C)
```c
void handle_request(int64_t request_id) {
    // ... fetch request ...
    
    // Parse JSON request
    struct json_object* json = json_tokener_parse(request->body);
    const char* name = json_object_get_string(
        json_object_object_get(json, "name")
    );
    
    // Create JSON response
    struct json_object* response_json = json_object_new_object();
    json_object_object_add(response_json, "greeting",
        json_object_new_string("Hello"));
    json_object_object_add(response_json, "name",
        json_object_new_string(name));
    
    const char* json_str = json_object_to_json_string(response_json);
    
    HttpResponse response = {
        .status_code = 200,
        .body = json_str,
        .body_len = strlen(json_str)
    };
    
    // ... push response ...
    
    json_object_put(json);
    json_object_put(response_json);
}
```

### Binary Data (C++)
```cpp
void handle_request(int64_t request_id) {
    // ... fetch request ...
    
    // Process image data
    std::vector<uint8_t> image_data = load_image();
    
    Response response;
    response.status_code = 200;
    response.body = image_data;
    response.headers["Content-Type"] = "image/png";
    
    response.pushToHost(request_id, host_url);
}
```

### Error Handling (C++)
```cpp
void handle_request(int64_t request_id) {
    try {
        // ... process request ...
    }
    catch (const std::exception& e) {
        Response error_response;
        error_response.status_code = 500;
        error_response.error = e.what();
        error_response.pushToHost(request_id, host_url);
    }
}
```

## Memory Management

### C Guidelines
- Always free allocated memory
- Use `free_request()` for requests
- Check malloc returns
- Avoid memory leaks in error paths

### C++ Guidelines
- Use RAII (smart pointers, containers)
- Prefer stack allocation
- Use std::string and std::vector
- Exception safety with try-catch

## Debugging

### Enable Logging
```c
#define DEBUG 1
#if DEBUG
    printf("Debug: Processing request %lld\n", request_id);
#endif
```

### Check Dependencies
```bash
# Linux
ldd plugin.so

# macOS
otool -L plugin.dylib

# Windows
dumpbin /dependents plugin.dll
```

### Common Issues

**Undefined symbols:**
- Link all required libraries
- Check function signatures match

**Segmentation faults:**
- Check pointer validity
- Verify memory allocation
- Use valgrind for debugging

**JSON parsing errors:**
- Validate JSON format
- Check encoding (UTF-8)
- Handle null/missing fields

## Performance Optimization

1. **Connection Pooling** - Reuse CURL handles
2. **Memory Pools** - Pre-allocate buffers
3. **Lazy Initialization** - Initialize once, reuse
4. **Compiler Optimization** - Use -O2 or -O3

## Build Options

### Debug Build
```bash
gcc -shared -fPIC -g -O0 -DDEBUG=1 -o plugin.so plugin.c -lcurl -ljson-c
```

### Release Build
```bash
gcc -shared -fPIC -O3 -DNDEBUG -o plugin.so plugin.c -lcurl -ljson-c
```

### Static Linking (Linux)
```bash
gcc -shared -fPIC -o plugin.so plugin.c -Wl,-Bstatic -lcurl -ljson-c -Wl,-Bdynamic -lpthread
```

## Testing

```bash
# Set environment
export HOST=http://127.0.0.1:3000

# Test with curl
curl http://127.0.0.1:3000/c-plugin/test
curl -X POST http://127.0.0.1:3000/cpp-plugin/json \
  -H "Content-Type: application/json" \
  -d '{"test": "data"}'
```

## Platform-Specific Notes

### Linux
- Ensure `libc` compatibility
- Check SELinux permissions
- Use `-fPIC` for position-independent code

### macOS
- Handle different architectures (x86_64, arm64)
- Code signing may be required
- Use `-undefined dynamic_lookup` if needed

### Windows
- Export functions with `__declspec(dllexport)`
- Handle different calling conventions
- Link with appropriate runtime (/MD, /MT)