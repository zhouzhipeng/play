#!/bin/bash

# Test script to verify MCP controller logging

echo "Testing MCP Controller Response Logging"
echo "======================================="

# Server URL
SERVER_URL="http://localhost:3000"

# Test 1: Initialize request
echo -e "\n1. Testing initialize request..."
curl -X POST "$SERVER_URL/mcp" \
  -H "Content-Type: application/json" \
  -H "Accept: text/event-stream" \
  -H "Origin: http://localhost:3000" \
  -H "Mcp-Session-Id: test-session-001" \
  -H "Mcp-Protocol-Version: 2025-03-26" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "initialize",
    "params": {
      "protocolVersion": "2025-03-26",
      "capabilities": {}
    }
  }' \
  --no-buffer

echo -e "\n\n2. Testing tools/list request..."
curl -X POST "$SERVER_URL/mcp" \
  -H "Content-Type: application/json" \
  -H "Accept: text/event-stream" \
  -H "Origin: http://localhost:3000" \
  -H "Mcp-Session-Id: test-session-001" \
  -H "Mcp-Protocol-Version: 2025-03-26" \
  -d '{
    "jsonrpc": "2.0",
    "id": 2,
    "method": "tools/list",
    "params": {}
  }' \
  --no-buffer

echo -e "\n\n3. Testing notification..."
curl -X POST "$SERVER_URL/mcp" \
  -H "Content-Type: application/json" \
  -H "Origin: http://localhost:3000" \
  -H "Mcp-Session-Id: test-session-001" \
  -d '{
    "jsonrpc": "2.0",
    "method": "progress",
    "params": {
      "progressToken": "test-token",
      "progress": 50,
      "total": 100
    }
  }'

echo -e "\n\n4. Testing response message..."
curl -X POST "$SERVER_URL/mcp" \
  -H "Content-Type: application/json" \
  -H "Origin: http://localhost:3000" \
  -H "Mcp-Session-Id: test-session-001" \
  -d '{
    "jsonrpc": "2.0",
    "id": 3,
    "result": {
      "status": "success",
      "data": "test response data"
    }
  }'

echo -e "\n\nDone! Check server logs for detailed response logging."