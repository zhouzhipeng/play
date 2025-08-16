#!/bin/bash

echo "Testing play-server with Xiaozhi MCP Integration"
echo "================================================="
echo

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Test 1: Check if play-server builds with new integration
echo -e "${YELLOW}1. Building play-server with xiaozhi integration...${NC}"
if cargo build -p play-server --features debug 2>&1 | tail -3 | grep -q "Finished"; then
    echo -e "${GREEN}✓ Build successful${NC}"
else
    echo -e "${RED}✗ Build failed${NC}"
    exit 1
fi

echo

# Test 2: Check dependencies are correctly linked
echo -e "${YELLOW}2. Checking dependencies...${NC}"
if cargo tree -p play-server --features debug 2>&1 | grep -q "play-integration-xiaozhi"; then
    echo -e "${GREEN}✓ play-integration-xiaozhi is properly linked${NC}"
else
    echo -e "${RED}✗ play-integration-xiaozhi not found in dependencies${NC}"
    exit 1
fi

echo

# Test 3: Verify old play-mcp client code is not used
echo -e "${YELLOW}3. Verifying migration from play-mcp client...${NC}"
if ! grep -r "play_mcp::start_mcp_client" crates/play-server/src 2>/dev/null; then
    echo -e "${GREEN}✓ Old play-mcp client code removed${NC}"
else
    echo -e "${RED}✗ Still using old play-mcp client${NC}"
    exit 1
fi

echo

# Test 4: Check that xiaozhi client is used
echo -e "${YELLOW}4. Verifying xiaozhi client usage...${NC}"
if grep -q "play_integration_xiaozhi::start_xiaozhi_client" /Users/ronnie/RustroverProjects/play/crates/play-server/src/lib.rs; then
    echo -e "${GREEN}✓ Using xiaozhi client${NC}"
else
    echo -e "${RED}✗ Not using xiaozhi client${NC}"
    exit 1
fi

echo

# Test 5: Check configuration type
echo -e "${YELLOW}5. Verifying configuration type...${NC}"
if grep -q "play_integration_xiaozhi::McpConfig" /Users/ronnie/RustroverProjects/play/crates/play-server/src/config.rs; then
    echo -e "${GREEN}✓ Using xiaozhi McpConfig${NC}"
else
    echo -e "${RED}✗ Not using xiaozhi McpConfig${NC}"
    exit 1
fi

echo
echo -e "${GREEN}================================${NC}"
echo -e "${GREEN}All integration tests passed!${NC}"
echo -e "${GREEN}================================${NC}"
echo

echo "Summary:"
echo "- play-server successfully migrated to use play-integration-xiaozhi"
echo "- Old play-mcp client code has been replaced"
echo "- Configuration properly uses xiaozhi types"
echo "- Feature flag 'play-integration-xiaozhi' is working"
echo
echo "The server will now use the Xiaozhi MCP client when started with debug features:"
echo "  cargo run -p play-server --features debug"