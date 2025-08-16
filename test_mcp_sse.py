#!/usr/bin/env python3

import requests
import json
import sys
from time import sleep

def test_mcp_endpoint():
    base_url = "http://localhost:3001/mcp"
    headers = {
        "Content-Type": "application/json",
        "Accept": "text/event-stream",
        "Origin": "http://localhost:3001",
        "Mcp-Protocol-Version": "2025-06-18"
    }
    
    print("Testing MCP Controller with SSE Support")
    print("=" * 40)
    
    # Test 1: Initialize
    print("\n1. Testing initialize request:")
    payload = {
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    }
    
    try:
        response = requests.post(base_url, json=payload, headers=headers, stream=True, timeout=5)
        print(f"Status Code: {response.status_code}")
        
        # Read SSE stream
        for line in response.iter_lines():
            if line:
                line_str = line.decode('utf-8')
                if line_str.startswith('data:'):
                    data = line_str[5:].strip()
                    if data:
                        print("Response:", json.dumps(json.loads(data), indent=2))
                        break
    except requests.Timeout:
        print("Request timed out after 5 seconds")
    except Exception as e:
        print(f"Error: {e}")
    
    # Test 2: List tools
    print("\n2. Testing tools/list request:")
    payload = {
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list"
    }
    
    try:
        response = requests.post(base_url, json=payload, headers=headers, stream=True, timeout=5)
        print(f"Status Code: {response.status_code}")
        
        for line in response.iter_lines():
            if line:
                line_str = line.decode('utf-8')
                if line_str.startswith('data:'):
                    data = line_str[5:].strip()
                    if data:
                        result = json.loads(data)
                        print("Response:", json.dumps(result, indent=2))
                        if 'result' in result and 'tools' in result['result']:
                            print(f"Found {len(result['result']['tools'])} tools")
                        break
    except requests.Timeout:
        print("Request timed out after 5 seconds")
    except Exception as e:
        print(f"Error: {e}")
    
    # Test 3: Call a tool
    print("\n3. Testing tools/call with http_request:")
    payload = {
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "http_request",
            "arguments": {
                "url": "https://api.example.com/test",
                "method": "GET"
            }
        }
    }
    
    try:
        response = requests.post(base_url, json=payload, headers=headers, stream=True, timeout=5)
        print(f"Status Code: {response.status_code}")
        
        for line in response.iter_lines():
            if line:
                line_str = line.decode('utf-8')
                if line_str.startswith('data:'):
                    data = line_str[5:].strip()
                    if data:
                        print("Response:", json.dumps(json.loads(data), indent=2))
                        break
    except requests.Timeout:
        print("Request timed out after 5 seconds")
    except Exception as e:
        print(f"Error: {e}")
    
    print("\n" + "=" * 40)
    print("Tests completed!")

if __name__ == "__main__":
    test_mcp_endpoint()