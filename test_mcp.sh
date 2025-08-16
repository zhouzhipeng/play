#!/bin/bash

echo "Testing MCP Controller"
echo "====================="

# Test 1: Initialize request
echo -e "\n1. Testing initialize request:"
curl -X POST http://localhost:3001/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: text/event-stream" \
  -H "Origin: http://localhost:3001" \
  -H "Mcp-Protocol-Version: 2025-06-18" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "initialize",
    "params": {}
  }'

# Test 2: Tools list request
echo -e "\n\n2. Testing tools/list request:"
curl -X POST http://localhost:3001/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: text/event-stream" \
  -H "Origin: http://localhost:3001" \
  -H "Mcp-Protocol-Version: 2025-06-18" \
  -d '{
    "jsonrpc": "2.0",
    "id": 2,
    "method": "tools/list"
  }'

# Test 3: SSE endpoint
echo -e "\n\n3. Testing SSE endpoint (will run for 5 seconds):"
timeout 5 curl -N http://localhost:3001/mcp \
  -H "Origin: http://localhost:3001" \
  -H "Mcp-Protocol-Version: 2025-06-18"

echo -e "\n\nTests completed!"