/**
 * Play Dylib Plugin ABI for C/C++
 * 
 * This header defines the interface for creating plugins compatible with
 * the Play server's new architecture.
 * 
 * The plugin only needs to implement a single function:
 *   void handle_request(int64_t request_id)
 * 
 * The plugin should:
 * 1. Fetch request data from host via HTTP GET
 * 2. Process the request
 * 3. Push response back to host via HTTP POST
 */

#ifndef PLAY_PLUGIN_ABI_H
#define PLAY_PLUGIN_ABI_H

#include <stdint.h>
#include <stdbool.h>
#include <stdlib.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * HTTP Method enumeration
 */
typedef enum {
    HTTP_GET,
    HTTP_POST,
    HTTP_PUT,
    HTTP_DELETE
} HttpMethod;

/**
 * Host context structure
 * Contains environment information provided by the host
 */
typedef struct {
    const char* host_url;           // Host server URL (e.g., "http://127.0.0.1:3000")
    const char* plugin_prefix_url;  // Plugin URL prefix
    const char* data_dir;           // Data directory path
    const char* config_text;        // Optional configuration text (may be NULL)
} HostContext;

/**
 * HTTP Request structure
 * Note: In the new architecture, this is fetched via HTTP, not passed directly
 */
typedef struct {
    HttpMethod method;
    const char* url;
    const char* query;
    const char* body;
    // Headers would be a key-value map in real implementation
    HostContext context;
} HttpRequest;

/**
 * HTTP Response structure  
 * Note: In the new architecture, this is pushed via HTTP, not returned directly
 */
typedef struct {
    uint16_t status_code;
    const char* body;
    size_t body_len;
    // Headers would be a key-value map in real implementation
    const char* error; // Optional error message (NULL for success)
} HttpResponse;

/**
 * Main plugin entry point - MUST be implemented by the plugin
 * 
 * This function is called by the host with a unique request ID.
 * The plugin should:
 * 1. Fetch the request from: GET {host_url}/admin/get-request-info?request_id={request_id}
 * 2. Process the request according to business logic
 * 3. Push the response to: POST {host_url}/admin/push-response-info?request_id={request_id}
 * 
 * @param request_id Unique identifier for this request
 */
#ifdef _WIN32
    __declspec(dllexport)
#else
    __attribute__((visibility("default")))
#endif
void handle_request(int64_t request_id);

/**
 * Helper function to get host URL from environment
 * Returns "http://127.0.0.1:3000" if HOST env var is not set
 */
static inline const char* get_host_url() {
    const char* host = getenv("HOST");
    return host ? host : "http://127.0.0.1:3000";
}

#ifdef __cplusplus
}
#endif

#endif // PLAY_PLUGIN_ABI_H