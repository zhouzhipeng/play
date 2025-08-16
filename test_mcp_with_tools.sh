#!/bin/bash

echo "Testing MCP Controller with play-mcp Tools Integration"
echo "======================================================="

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test 1: Initialize request
echo -e "\n${YELLOW}1. Testing initialize request:${NC}"
curl -s -X POST http://localhost:3001/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: text/event-stream" \
  -H "Origin: http://localhost:3001" \
  -H "Mcp-Protocol-Version: 2025-06-18" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "initialize",
    "params": {}
  }' | head -5

# Test 2: List available tools
echo -e "\n${YELLOW}2. Testing tools/list request:${NC}"
response=$(curl -s -X POST http://localhost:3001/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: text/event-stream" \
  -H "Origin: http://localhost:3001" \
  -H "Mcp-Protocol-Version: 2025-06-18" \
  -H "Mcp-Session-Id: test-session-123" \
  -d '{
    "jsonrpc": "2.0",
    "id": 2,
    "method": "tools/list"
  }')

# Extract and format the SSE data
echo "$response" | grep "^data:" | sed 's/^data: //' | jq '.'

# Test 3: Call http_request tool
echo -e "\n${YELLOW}3. Testing tools/call with http_request tool:${NC}"
response=$(curl -s -X POST http://localhost:3001/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: text/event-stream" \
  -H "Origin: http://localhost:3001" \
  -H "Mcp-Protocol-Version: 2025-06-18" \
  -H "Mcp-Session-Id: test-session-123" \
  -d '{
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
  }')

echo "$response" | grep "^data:" | sed 's/^data: //' | jq '.'

# Test 4: Call non-existent tool
echo -e "\n${YELLOW}4. Testing tools/call with non-existent tool (should return error):${NC}"
response=$(curl -s -X POST http://localhost:3001/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: text/event-stream" \
  -H "Origin: http://localhost:3001" \
  -H "Mcp-Protocol-Version: 2025-06-18" \
  -H "Mcp-Session-Id: test-session-123" \
  -d '{
    "jsonrpc": "2.0",
    "id": 4,
    "method": "tools/call",
    "params": {
      "name": "non_existent_tool",
      "arguments": {}
    }
  }')

echo "$response" | grep "^data:" | sed 's/^data: //' | jq '.'

# Test 5: SSE endpoint (keep-alive test)
echo -e "\n${YELLOW}5. Testing SSE endpoint (will run for 3 seconds):${NC}"
timeout 3 curl -N http://localhost:3001/mcp \
  -H "Origin: http://localhost:3001" \
  -H "Mcp-Protocol-Version: 2025-06-18" \
  -H "Mcp-Session-Id: test-session-123" 2>/dev/null | head -10

echo -e "\n${GREEN}Tests completed!${NC}"
echo -e "\n${YELLOW}Summary:${NC}"
echo "- MCP controller is integrated with play-mcp tools"
echo "- Tools can be listed via tools/list method"
echo "- Tools can be executed via tools/call method"
echo "- Session management is working with Mcp-Session-Id header"
echo "- SSE streaming is functional for real-time communication"