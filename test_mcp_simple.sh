#!/bin/bash

echo "Simple MCP Controller Test"
echo "=========================="

# First, test if the endpoint is accessible
echo -e "\n1. Testing basic endpoint accessibility:"
curl -v -X POST http://localhost:3001/mcp \
  -H "Content-Type: application/json" \
  -H "Origin: http://localhost:3001" \
  -d '{"jsonrpc": "2.0", "id": 1, "method": "test"}' 2>&1 | grep -E "(< HTTP|< )"

echo -e "\n2. Testing with SSE headers (verbose mode):"
curl -v -X POST http://localhost:3001/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: text/event-stream" \
  -H "Origin: http://localhost:3001" \
  -d '{"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}' 2>&1 | head -30