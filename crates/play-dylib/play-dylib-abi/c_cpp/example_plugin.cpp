/**
 * Example C++ Plugin Implementation
 * 
 * Build commands:
 * Linux:   g++ -shared -fPIC -std=c++17 -o plugin.so example_plugin.cpp -lcurl -ljsoncpp
 * macOS:   g++ -shared -std=c++17 -o plugin.dylib example_plugin.cpp -lcurl -ljsoncpp
 * Windows: cl /LD /std:c++17 example_plugin.cpp /Fe:plugin.dll libcurl.lib jsoncpp.lib
 */

#include <iostream>
#include <string>
#include <vector>
#include <map>
#include <memory>
#include <sstream>
#include <curl/curl.h>
#include <json/json.h>
#include "plugin_abi.h"

namespace PlayPlugin {

// HTTP Client class for making requests
class HttpClient {
public:
    HttpClient() {
        curl_global_init(CURL_GLOBAL_DEFAULT);
    }
    
    ~HttpClient() {
        curl_global_cleanup();
    }
    
    // Perform GET request
    std::string get(const std::string& url) {
        CURL* curl = curl_easy_init();
        if (!curl) {
            throw std::runtime_error("Failed to initialize CURL");
        }
        
        std::string response;
        curl_easy_setopt(curl, CURLOPT_URL, url.c_str());
        curl_easy_setopt(curl, CURLOPT_WRITEFUNCTION, writeCallback);
        curl_easy_setopt(curl, CURLOPT_WRITEDATA, &response);
        
        CURLcode res = curl_easy_perform(curl);
        curl_easy_cleanup(curl);
        
        if (res != CURLE_OK) {
            throw std::runtime_error(std::string("CURL GET failed: ") + curl_easy_strerror(res));
        }
        
        return response;
    }
    
    // Perform POST request with JSON body
    std::string post(const std::string& url, const std::string& json_body) {
        CURL* curl = curl_easy_init();
        if (!curl) {
            throw std::runtime_error("Failed to initialize CURL");
        }
        
        std::string response;
        struct curl_slist* headers = nullptr;
        headers = curl_slist_append(headers, "Content-Type: application/json");
        
        curl_easy_setopt(curl, CURLOPT_URL, url.c_str());
        curl_easy_setopt(curl, CURLOPT_POST, 1L);
        curl_easy_setopt(curl, CURLOPT_POSTFIELDS, json_body.c_str());
        curl_easy_setopt(curl, CURLOPT_HTTPHEADER, headers);
        curl_easy_setopt(curl, CURLOPT_WRITEFUNCTION, writeCallback);
        curl_easy_setopt(curl, CURLOPT_WRITEDATA, &response);
        
        CURLcode res = curl_easy_perform(curl);
        curl_slist_free_all(headers);
        curl_easy_cleanup(curl);
        
        if (res != CURLE_OK) {
            throw std::runtime_error(std::string("CURL POST failed: ") + curl_easy_strerror(res));
        }
        
        return response;
    }
    
private:
    static size_t writeCallback(void* contents, size_t size, size_t nmemb, void* userp) {
        size_t realsize = size * nmemb;
        std::string* str = static_cast<std::string*>(userp);
        str->append(static_cast<char*>(contents), realsize);
        return realsize;
    }
};

// Request wrapper class
class Request {
public:
    HttpMethod method;
    std::string url;
    std::string query;
    std::string body;
    std::map<std::string, std::string> headers;
    
    // Parse from JSON
    static Request fromJson(const std::string& json_str) {
        Json::CharReaderBuilder builder;
        Json::Value root;
        std::string errors;
        std::istringstream stream(json_str);
        
        if (!Json::parseFromStream(builder, stream, &root, &errors)) {
            throw std::runtime_error("Failed to parse request JSON: " + errors);
        }
        
        Request req;
        
        // Parse method
        std::string method_str = root["method"].asString();
        if (method_str == "GET") req.method = HTTP_GET;
        else if (method_str == "POST") req.method = HTTP_POST;
        else if (method_str == "PUT") req.method = HTTP_PUT;
        else if (method_str == "DELETE") req.method = HTTP_DELETE;
        
        // Parse other fields
        req.url = root["url"].asString();
        req.query = root["query"].asString();
        req.body = root["body"].asString();
        
        // Parse headers
        const Json::Value& headers = root["headers"];
        for (const auto& key : headers.getMemberNames()) {
            req.headers[key] = headers[key].asString();
        }
        
        return req;
    }
};

// Response wrapper class
class Response {
public:
    uint16_t status_code = 200;
    std::vector<uint8_t> body;
    std::map<std::string, std::string> headers;
    std::optional<std::string> error;
    
    // Convert to JSON for pushing to host
    std::string toJson() const {
        Json::Value root;
        root["status_code"] = status_code;
        
        // Convert body to integer array (for compatibility)
        Json::Value body_array(Json::arrayValue);
        for (uint8_t byte : body) {
            body_array.append(static_cast<int>(byte));
        }
        root["body"] = body_array;
        
        // Add headers
        Json::Value headers_obj;
        for (const auto& [key, value] : headers) {
            headers_obj[key] = value;
        }
        root["headers"] = headers_obj;
        
        // Add error if present
        if (error.has_value()) {
            root["error"] = error.value();
        }
        
        Json::StreamWriterBuilder builder;
        return Json::writeString(builder, root);
    }
    
    // Helper to set body from string
    void setBody(const std::string& str) {
        body.assign(str.begin(), str.end());
    }
};

// Plugin handler class
class PluginHandler {
public:
    void handleRequest(int64_t request_id) {
        std::cout << "C++ Plugin: Handling request " << request_id << std::endl;
        
        try {
            // Get host URL
            std::string host_url = getHostUrl();
            
            // Fetch request
            Request request = fetchRequest(request_id, host_url);
            
            // Process request
            Response response = processRequest(request, request_id);
            
            // Push response
            pushResponse(request_id, host_url, response);
            
            std::cout << "C++ Plugin: Successfully handled request " << request_id << std::endl;
        }
        catch (const std::exception& e) {
            std::cerr << "C++ Plugin Error: " << e.what() << std::endl;
            
            // Try to send error response
            try {
                Response error_response;
                error_response.status_code = 500;
                error_response.error = e.what();
                pushResponse(request_id, getHostUrl(), error_response);
            }
            catch (...) {
                // Failed to send error response
            }
        }
    }
    
private:
    HttpClient http_client;
    
    std::string getHostUrl() {
        const char* host = std::getenv("HOST");
        return host ? host : "http://127.0.0.1:3000";
    }
    
    Request fetchRequest(int64_t request_id, const std::string& host_url) {
        std::stringstream url;
        url << host_url << "/admin/get-request-info?request_id=" << request_id;
        
        std::string response = http_client.get(url.str());
        return Request::fromJson(response);
    }
    
    void pushResponse(int64_t request_id, const std::string& host_url, const Response& response) {
        std::stringstream url;
        url << host_url << "/admin/push-response-info?request_id=" << request_id;
        
        http_client.post(url.str(), response.toJson());
    }
    
    Response processRequest(const Request& request, int64_t request_id) {
        Response response;
        
        // Route based on URL
        if (request.url.find("/hello") != std::string::npos) {
            // Hello endpoint
            response.setBody("Hello from C++ plugin!");
            response.headers["Content-Type"] = "text/plain";
            response.status_code = 200;
        }
        else if (request.url.find("/echo") != std::string::npos) {
            // Echo endpoint
            response.setBody(request.body);
            response.headers["Content-Type"] = "text/plain";
            response.status_code = 200;
        }
        else if (request.url.find("/json") != std::string::npos) {
            // JSON response example
            Json::Value json_response;
            json_response["message"] = "Response from C++ plugin";
            json_response["request_id"] = static_cast<Json::Int64>(request_id);
            json_response["method"] = methodToString(request.method);
            json_response["url"] = request.url;
            json_response["timestamp"] = static_cast<Json::Int64>(std::time(nullptr));
            
            Json::StreamWriterBuilder builder;
            std::string json_str = Json::writeString(builder, json_response);
            response.setBody(json_str);
            response.headers["Content-Type"] = "application/json";
            response.status_code = 200;
        }
        else {
            // Default response
            std::stringstream msg;
            msg << "C++ Plugin processed request " << request_id 
                << " with URL: " << request.url;
            response.setBody(msg.str());
            response.headers["Content-Type"] = "text/plain";
            response.status_code = 200;
        }
        
        return response;
    }
    
    std::string methodToString(HttpMethod method) {
        switch (method) {
            case HTTP_GET: return "GET";
            case HTTP_POST: return "POST";
            case HTTP_PUT: return "PUT";
            case HTTP_DELETE: return "DELETE";
            default: return "UNKNOWN";
        }
    }
};

// Global handler instance
static std::unique_ptr<PluginHandler> g_handler;

} // namespace PlayPlugin

// Export C interface
extern "C" {

void handle_request(int64_t request_id) {
    // Initialize handler on first use
    if (!PlayPlugin::g_handler) {
        PlayPlugin::g_handler = std::make_unique<PlayPlugin::PluginHandler>();
    }
    
    PlayPlugin::g_handler->handleRequest(request_id);
}

} // extern "C"