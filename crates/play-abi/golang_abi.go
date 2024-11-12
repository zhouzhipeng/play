// usage:
// impl method in your code:
// func HandleRequest(request *HttpRequest) HttpResponse

package main

/*
#include <stdlib.h>
*/
import "C"
import (
	"encoding/json"
	"fmt"
	"log"
	"unsafe"
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

// ParseHttpRequest parses a JSON string into an HttpRequest
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
func handle_request(input *C.char) *C.char {
	goInput := C.GoString(input)
	request, err := ParseHttpRequest(goInput)

	if err != nil {
		log.Fatalf("Error parsing request: %v", err)
	}

	response := HandleRequest(request)

	return C.CString(response.String())
}

//export free_c_string
func free_c_string(cstr *C.char) {
	C.free(unsafe.Pointer(cstr))
}