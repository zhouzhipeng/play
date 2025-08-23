// usage:
// impl method in your code:
// func HandleRequest(requestID int64) error
// The function should:
// 1. Fetch request from host via GET /admin/get-request-info?request_id=${requestID}
// 2. Process the request
// 3. Push response to host via POST /admin/push-response-info?request_id=${requestID}

package main

/*
#include <stdlib.h>
#include <stdint.h>
*/
import "C"
import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"log"
	"net/http"
	"os"
)

// HttpMethod represents HTTP request methods
type HttpMethod string

const (
	GET    HttpMethod = "GET"
	POST   HttpMethod = "POST"
	PUT    HttpMethod = "PUT"
	DELETE HttpMethod = "DELETE"
)

// HostContext represents the context information for the host
type HostContext struct {
	// HostURL represents the host http url, eg. http://127.0.0.1:3000
	HostURL         string  `json:"host_url"`
	PluginPrefixURL string  `json:"plugin_prefix_url"`
	DataDir         string  `json:"data_dir"`
	ConfigText      *string `json:"config_text,omitempty"`
}

// DefaultHostContext returns a new HostContext with default values
func DefaultHostContext() HostContext {
	return HostContext{
		HostURL:         "",
		PluginPrefixURL: "",
		DataDir:         "",
		ConfigText:      nil,
	}
}

// HttpRequest represents an HTTP request with its components
type HttpRequest struct {
	Method  HttpMethod        `json:"method"`
	Headers map[string]string `json:"headers"`
	Query   string            `json:"query"`
	URL     string            `json:"url"`
	Body    string            `json:"body"`
	Context HostContext       `json:"context"`
}

// DefaultHttpRequest returns a new HttpRequest with default values
func DefaultHttpRequest() HttpRequest {
	return HttpRequest{
		Method:  GET,
		Headers: make(map[string]string),
		Query:   "",
		URL:     "",
		Body:    "",
		Context: DefaultHostContext(),
	}
}

// FetchRequestFromHost fetches the request data from the host server
func FetchRequestFromHost(requestID int64, hostURL string) (*HttpRequest, error) {
	url := fmt.Sprintf("%s/admin/get-request-info?request_id=%d", hostURL, requestID)
	resp, err := http.Get(url)
	if err != nil {
		return nil, fmt.Errorf("failed to fetch request: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		body, _ := io.ReadAll(resp.Body)
		return nil, fmt.Errorf("failed to fetch request, status: %d, body: %s", resp.StatusCode, string(body))
	}

	var request HttpRequest
	if err := json.NewDecoder(resp.Body).Decode(&request); err != nil {
		return nil, fmt.Errorf("failed to decode request: %w", err)
	}

	return &request, nil
}

// ParseHttpRequest parses a JSON string into an HttpRequest (kept for compatibility)
func ParseHttpRequest(jsonStr string) (*HttpRequest, error) {
	request := DefaultHttpRequest()
	err := json.Unmarshal([]byte(jsonStr), &request)
	if err != nil {
		return nil, fmt.Errorf("failed to parse HTTP request: %w", err)
	}
	return &request, nil
}

// String returns a string representation of HttpRequest
func (r HttpRequest) String() string {
	bytes, err := json.MarshalIndent(r, "", "  ")
	if err != nil {
		return fmt.Sprintf("error marshaling request: %v", err)
	}
	return string(bytes)
}

// HttpResponse represents an HTTP response
type HttpResponse struct {
	Headers    map[string]string `json:"headers"`
	Body       []byte            `json:"body"`
	StatusCode int               `json:"status_code"`
	// Error is used to mark if current plugin running is successful or not
	// should be nil for normal business logic
	Error *string `json:"error,omitempty"`
}

// DefaultStatusCode returns the default status code (200 OK)
func DefaultStatusCode() int {
	return 200
}

// NewHttpResponse creates a new HttpResponse with default values
func NewHttpResponse() HttpResponse {
	return HttpResponse{
		Headers:    make(map[string]string),
		Body:       make([]byte, 0),
		StatusCode: DefaultStatusCode(),
		Error:      nil,
	}
}

func ErrResponse(err error) HttpResponse {
	errStr := err.Error()
	return HttpResponse{
		Headers:    make(map[string]string),
		Body:       make([]byte, 0),
		StatusCode: 500,
		Error:      &errStr,
	}
}

// PushResponseToHost sends the response back to the host server
func (r HttpResponse) PushToHost(requestID int64, hostURL string) error {
	url := fmt.Sprintf("%s/admin/push-response-info?request_id=%d", hostURL, requestID)
	
	// Convert response to JSON with body as int array for compatibility
	type Alias struct {
		Headers    map[string]string `json:"headers"`
		Body       []int             `json:"body"` // body as int array
		StatusCode int               `json:"status_code"`
		Error      *string           `json:"error,omitempty"`
	}

	// Convert []byte to []int
	body := make([]int, len(r.Body))
	for i, b := range r.Body {
		body[i] = int(b)
	}

	jsonData, err := json.Marshal(Alias{
		Headers:    r.Headers,
		Body:       body,
		StatusCode: r.StatusCode,
		Error:      r.Error,
	})
	if err != nil {
		return fmt.Errorf("failed to marshal response: %w", err)
	}

	resp, err := http.Post(url, "application/json", bytes.NewBuffer(jsonData))
	if err != nil {
		return fmt.Errorf("failed to push response: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		body, _ := io.ReadAll(resp.Body)
		return fmt.Errorf("failed to push response, status: %d, body: %s", resp.StatusCode, string(body))
	}

	return nil
}

// String returns a string representation of HttpResponse
func (r HttpResponse) String() string {
	// 创建一个用于JSON序列化的临时结构体
	type Alias struct {
		Headers    map[string]string `json:"headers"`
		Body       []int             `json:"body"` // body as int array
		StatusCode int               `json:"status_code"`
		Error      *string           `json:"error,omitempty"`
	}

	// 将 []byte 转换为 []int
	body := make([]int, len(r.Body))
	for i, b := range r.Body {
		body[i] = int(b)
	}

	bytes, err := json.Marshal(Alias{
		Headers:    r.Headers,
		Body:       body,
		StatusCode: r.StatusCode,
		Error:      r.Error,
	})
	if err != nil {
		return fmt.Sprintf("error marshaling response: %v", err)
	}
	return string(bytes)
}

//export handle_request
func handle_request(requestID C.int64_t) {
	// Convert C int64 to Go int64
	goRequestID := int64(requestID)
	
	// Call the user-implemented function
	if err := HandleRequest(goRequestID); err != nil {
		log.Printf("Error handling request %d: %v", goRequestID, err)
	}
}

// Helper function to get host URL from environment or default
func GetHostURL() string {
	if hostURL := os.Getenv("HOST"); hostURL != "" {
		return hostURL
	}
	return "http://127.0.0.1:3000"
}

// Example implementation (user should replace this)
// func HandleRequest(requestID int64) error {
// 	hostURL := GetHostURL()
// 	
// 	// Fetch request from host
// 	request, err := FetchRequestFromHost(requestID, hostURL)
// 	if err != nil {
// 		return err
// 	}
// 	
// 	// Process request and create response
// 	response := NewHttpResponse()
// 	response.StatusCode = 200
// 	response.Headers["Content-Type"] = "text/plain"
// 	response.Body = []byte("Hello from Go plugin")
// 	
// 	// Push response back to host
// 	return response.PushToHost(requestID, hostURL)
// }