// Example Go plugin implementation using the new architecture
// Build with: go build -buildmode=c-shared -o example_plugin.so example_plugin.go

package main

import (
	"encoding/json"
	"fmt"
	"log"
	"time"
)

// HandleRequest is the main entry point for the plugin
// It receives a request ID and must:
// 1. Fetch the request from the host
// 2. Process the request
// 3. Push the response back to the host
func HandleRequest(requestID int64) error {
	log.Printf("Handling request with ID: %d", requestID)
	
	// Get host URL from environment or use default
	hostURL := GetHostURL()
	
	// Step 1: Fetch request from host
	request, err := FetchRequestFromHost(requestID, hostURL)
	if err != nil {
		return fmt.Errorf("failed to fetch request: %w", err)
	}
	
	log.Printf("Fetched request: Method=%s, URL=%s, Query=%s", 
		request.Method, request.URL, request.Query)
	
	// Step 2: Process the request (business logic)
	response := processBusinessLogic(request)
	
	// Step 3: Push response back to host
	if err := response.PushToHost(requestID, hostURL); err != nil {
		return fmt.Errorf("failed to push response: %w", err)
	}
	
	log.Printf("Successfully handled request %d", requestID)
	return nil
}

// processBusinessLogic contains the actual business logic for handling requests
func processBusinessLogic(request *HttpRequest) HttpResponse {
	response := NewHttpResponse()
	
	// Parse query parameters if needed
	queryParams := parseQueryString(request.Query)
	
	// Example: Different responses based on URL suffix
	if request.URL == "/plugin/hello" {
		response.Headers["Content-Type"] = "application/json"
		responseData := map[string]interface{}{
			"message":    "Hello from Go plugin!",
			"timestamp":  time.Now().Unix(),
			"request_id": queryParams["request_id"],
			"method":     string(request.Method),
		}
		
		jsonData, _ := json.Marshal(responseData)
		response.Body = jsonData
		response.StatusCode = 200
	} else if request.URL == "/plugin/echo" {
		// Echo back the request body
		response.Headers["Content-Type"] = "text/plain"
		response.Body = []byte(request.Body)
		response.StatusCode = 200
	} else {
		// Default response
		response.Headers["Content-Type"] = "text/plain"
		response.Body = []byte("Unknown endpoint")
		response.StatusCode = 404
	}
	
	return response
}

// parseQueryString is a simple query string parser
func parseQueryString(query string) map[string]string {
	params := make(map[string]string)
	if query == "" {
		return params
	}
	
	// Simple parsing (for production, use url.ParseQuery)
	// This is just for demonstration
	return params
}

// Required main function for shared library
func main() {
	// This is required for building as a shared library
	// The actual entry point is the exported handle_request function
}