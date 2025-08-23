/**
 * Example C Plugin Implementation
 * 
 * Build commands:
 * Linux:   gcc -shared -fPIC -o plugin.so example_plugin.c -lcurl -ljson-c
 * macOS:   gcc -shared -o plugin.dylib example_plugin.c -lcurl -ljson-c
 * Windows: cl /LD example_plugin.c /Fe:plugin.dll libcurl.lib json-c.lib
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <curl/curl.h>
#include <json-c/json.h>
#include "plugin_abi.h"

// Buffer structure for CURL responses
typedef struct {
    char* data;
    size_t size;
} ResponseBuffer;

// CURL write callback
static size_t write_callback(void* contents, size_t size, size_t nmemb, void* userp) {
    size_t realsize = size * nmemb;
    ResponseBuffer* mem = (ResponseBuffer*)userp;
    
    char* ptr = realloc(mem->data, mem->size + realsize + 1);
    if (!ptr) {
        fprintf(stderr, "Not enough memory\n");
        return 0;
    }
    
    mem->data = ptr;
    memcpy(&(mem->data[mem->size]), contents, realsize);
    mem->size += realsize;
    mem->data[mem->size] = 0;
    
    return realsize;
}

// Fetch request from host
static HttpRequest* fetch_request(int64_t request_id, const char* host_url) {
    CURL* curl;
    CURLcode res;
    ResponseBuffer response = {0};
    HttpRequest* request = NULL;
    
    curl = curl_easy_init();
    if (!curl) {
        fprintf(stderr, "Failed to initialize CURL\n");
        return NULL;
    }
    
    // Build URL
    char url[512];
    snprintf(url, sizeof(url), "%s/admin/get-request-info?request_id=%lld", 
             host_url, (long long)request_id);
    
    // Set CURL options
    curl_easy_setopt(curl, CURLOPT_URL, url);
    curl_easy_setopt(curl, CURLOPT_WRITEFUNCTION, write_callback);
    curl_easy_setopt(curl, CURLOPT_WRITEDATA, (void*)&response);
    
    // Perform request
    res = curl_easy_perform(curl);
    
    if (res != CURLE_OK) {
        fprintf(stderr, "CURL failed: %s\n", curl_easy_strerror(res));
    } else {
        // Parse JSON response
        struct json_object* json = json_tokener_parse(response.data);
        if (json) {
            request = malloc(sizeof(HttpRequest));
            memset(request, 0, sizeof(HttpRequest));
            
            // Parse method
            struct json_object* method_obj;
            if (json_object_object_get_ex(json, "method", &method_obj)) {
                const char* method_str = json_object_get_string(method_obj);
                if (strcmp(method_str, "GET") == 0) request->method = HTTP_GET;
                else if (strcmp(method_str, "POST") == 0) request->method = HTTP_POST;
                else if (strcmp(method_str, "PUT") == 0) request->method = HTTP_PUT;
                else if (strcmp(method_str, "DELETE") == 0) request->method = HTTP_DELETE;
            }
            
            // Parse URL
            struct json_object* url_obj;
            if (json_object_object_get_ex(json, "url", &url_obj)) {
                request->url = strdup(json_object_get_string(url_obj));
            }
            
            // Parse query
            struct json_object* query_obj;
            if (json_object_object_get_ex(json, "query", &query_obj)) {
                request->query = strdup(json_object_get_string(query_obj));
            }
            
            // Parse body
            struct json_object* body_obj;
            if (json_object_object_get_ex(json, "body", &body_obj)) {
                request->body = strdup(json_object_get_string(body_obj));
            }
            
            json_object_put(json);
        }
    }
    
    curl_easy_cleanup(curl);
    free(response.data);
    
    return request;
}

// Push response to host
static bool push_response(int64_t request_id, const char* host_url, HttpResponse* response) {
    CURL* curl;
    CURLcode res;
    bool success = false;
    
    curl = curl_easy_init();
    if (!curl) {
        fprintf(stderr, "Failed to initialize CURL\n");
        return false;
    }
    
    // Build URL
    char url[512];
    snprintf(url, sizeof(url), "%s/admin/push-response-info?request_id=%lld",
             host_url, (long long)request_id);
    
    // Build JSON response
    struct json_object* json = json_object_new_object();
    json_object_object_add(json, "status_code", json_object_new_int(response->status_code));
    
    // Convert body to byte array (as integers for compatibility)
    struct json_object* body_array = json_object_new_array();
    for (size_t i = 0; i < response->body_len; i++) {
        json_object_array_add(body_array, json_object_new_int((unsigned char)response->body[i]));
    }
    json_object_object_add(json, "body", body_array);
    
    // Add headers
    struct json_object* headers = json_object_new_object();
    json_object_object_add(headers, "Content-Type", json_object_new_string("text/plain"));
    json_object_object_add(json, "headers", headers);
    
    // Add error if present
    if (response->error) {
        json_object_object_add(json, "error", json_object_new_string(response->error));
    }
    
    const char* json_str = json_object_to_json_string(json);
    
    // Set CURL options
    curl_easy_setopt(curl, CURLOPT_URL, url);
    curl_easy_setopt(curl, CURLOPT_POST, 1L);
    curl_easy_setopt(curl, CURLOPT_POSTFIELDS, json_str);
    
    struct curl_slist* headers_list = NULL;
    headers_list = curl_slist_append(headers_list, "Content-Type: application/json");
    curl_easy_setopt(curl, CURLOPT_HTTPHEADER, headers_list);
    
    // Perform request
    res = curl_easy_perform(curl);
    
    if (res == CURLE_OK) {
        success = true;
        printf("Response pushed successfully for request %lld\n", (long long)request_id);
    } else {
        fprintf(stderr, "Failed to push response: %s\n", curl_easy_strerror(res));
    }
    
    curl_slist_free_all(headers_list);
    json_object_put(json);
    curl_easy_cleanup(curl);
    
    return success;
}

// Free request memory
static void free_request(HttpRequest* request) {
    if (!request) return;
    free((void*)request->url);
    free((void*)request->query);
    free((void*)request->body);
    free(request);
}

// Main plugin entry point
void handle_request(int64_t request_id) {
    printf("C Plugin: Handling request %lld\n", (long long)request_id);
    
    // Get host URL
    const char* host_url = get_host_url();
    
    // Fetch request
    HttpRequest* request = fetch_request(request_id, host_url);
    if (!request) {
        fprintf(stderr, "Failed to fetch request\n");
        return;
    }
    
    printf("Received request: Method=%d, URL=%s, Query=%s\n", 
           request->method, request->url, request->query);
    
    // Process request (business logic)
    HttpResponse response = {0};
    
    if (request->url && strstr(request->url, "/hello")) {
        // Hello endpoint
        const char* message = "Hello from C plugin!";
        response.status_code = 200;
        response.body = message;
        response.body_len = strlen(message);
        response.error = NULL;
    } else if (request->url && strstr(request->url, "/echo")) {
        // Echo endpoint
        response.status_code = 200;
        response.body = request->body ? request->body : "";
        response.body_len = request->body ? strlen(request->body) : 0;
        response.error = NULL;
    } else {
        // Default response
        char buffer[256];
        snprintf(buffer, sizeof(buffer), 
                "C Plugin processed request %lld with URL: %s",
                (long long)request_id, request->url ? request->url : "unknown");
        response.status_code = 200;
        response.body = buffer;
        response.body_len = strlen(buffer);
        response.error = NULL;
    }
    
    // Push response
    if (!push_response(request_id, host_url, &response)) {
        fprintf(stderr, "Failed to push response\n");
    }
    
    // Cleanup
    free_request(request);
    
    printf("C Plugin: Finished handling request %lld\n", (long long)request_id);
}