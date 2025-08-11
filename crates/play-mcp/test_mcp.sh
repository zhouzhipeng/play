#!/bin/bash

echo "Testing MCP Server..."
echo ""

echo "1. Testing initialize:"
echo '{"jsonrpc":"2.0","method":"initialize","params":{},"id":1}' | cargo run --quiet

echo ""
echo "2. Testing tools/list:"
echo '{"jsonrpc":"2.0","method":"tools/list","params":{},"id":2}' | cargo run --quiet

echo ""
echo "3. Testing get_disk_space (all disks):"
echo '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"get_disk_space","arguments":{}},"id":3}' | cargo run --quiet

echo ""
echo "4. Testing get_disk_space (specific path):"
echo '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"get_disk_space","arguments":{"path":"/"}},"id":4}' | cargo run --quiet