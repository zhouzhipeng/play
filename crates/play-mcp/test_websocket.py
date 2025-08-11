#!/usr/bin/env python3
import asyncio
import websockets
import json

async def test_mcp_websocket():
    uri = "ws://127.0.0.1:8765"
    
    try:
        async with websockets.connect(uri) as websocket:
            print(f"Connected to {uri}")
            
            # Test 1: Initialize
            print("\n1. Testing initialize:")
            request = {
                "jsonrpc": "2.0",
                "method": "initialize",
                "params": {},
                "id": 1
            }
            await websocket.send(json.dumps(request))
            response = await websocket.recv()
            print(f"Response: {json.dumps(json.loads(response), indent=2)}")
            
            # Test 2: List tools
            print("\n2. Testing tools/list:")
            request = {
                "jsonrpc": "2.0",
                "method": "tools/list",
                "params": {},
                "id": 2
            }
            await websocket.send(json.dumps(request))
            response = await websocket.recv()
            print(f"Response: {json.dumps(json.loads(response), indent=2)}")
            
            # Test 3: Get disk space (all disks)
            print("\n3. Testing get_disk_space (all disks):")
            request = {
                "jsonrpc": "2.0",
                "method": "tools/call",
                "params": {
                    "name": "get_disk_space",
                    "arguments": {}
                },
                "id": 3
            }
            await websocket.send(json.dumps(request))
            response = await websocket.recv()
            response_json = json.loads(response)
            if response_json.get("result") and response_json["result"].get("content"):
                content = response_json["result"]["content"][0]["text"]
                print(f"Disk Space Info:\n{content}")
            else:
                print(f"Response: {json.dumps(response_json, indent=2)}")
            
            # Test 4: Get disk space (specific path)
            print("\n4. Testing get_disk_space (path: /):")
            request = {
                "jsonrpc": "2.0",
                "method": "tools/call",
                "params": {
                    "name": "get_disk_space",
                    "arguments": {"path": "/"}
                },
                "id": 4
            }
            await websocket.send(json.dumps(request))
            response = await websocket.recv()
            response_json = json.loads(response)
            if response_json.get("result") and response_json["result"].get("content"):
                content = response_json["result"]["content"][0]["text"]
                print(f"Disk Space Info for /:\n{content}")
            else:
                print(f"Response: {json.dumps(response_json, indent=2)}")
                
    except Exception as e:
        print(f"Error: {e}")
        print("Make sure the MCP server is running with:")
        print("  cargo run -- --mode websocket --address 127.0.0.1:8765")

if __name__ == "__main__":
    asyncio.run(test_mcp_websocket())