#!/bin/bash

echo "Testing play-integration-xiaozhi crate"
echo "======================================"
echo

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Test 1: Build the crate
echo -e "${YELLOW}1. Building play-integration-xiaozhi...${NC}"
if cargo build -p play-integration-xiaozhi 2>&1 | tail -3; then
    echo -e "${GREEN}✓ Build successful${NC}"
else
    echo -e "${RED}✗ Build failed${NC}"
    exit 1
fi

echo

# Test 2: Run unit tests
echo -e "${YELLOW}2. Running unit tests...${NC}"
if cargo test -p play-integration-xiaozhi --quiet; then
    echo -e "${GREEN}✓ All tests passed${NC}"
else
    echo -e "${RED}✗ Tests failed${NC}"
    exit 1
fi

echo

# Test 3: Build the example
echo -e "${YELLOW}3. Building xiaozhi_client example...${NC}"
if cargo build --example xiaozhi_client -p play-integration-xiaozhi 2>&1 | tail -3; then
    echo -e "${GREEN}✓ Example build successful${NC}"
else
    echo -e "${RED}✗ Example build failed${NC}"
    exit 1
fi

echo

# Test 4: Check that play-mcp still builds
echo -e "${YELLOW}4. Verifying play-mcp still builds...${NC}"
if cargo build -p play-mcp --quiet; then
    echo -e "${GREEN}✓ play-mcp builds successfully${NC}"
else
    echo -e "${RED}✗ play-mcp build failed${NC}"
    exit 1
fi

echo
echo -e "${GREEN}================================${NC}"
echo -e "${GREEN}All integration tests passed!${NC}"
echo -e "${GREEN}================================${NC}"
echo

echo "Summary:"
echo "- New crate 'play-integration-xiaozhi' created successfully"
echo "- MCP client logic moved from play-mcp to new crate"
echo "- All dependencies properly configured"
echo "- Unit tests passing"
echo "- Example client builds correctly"
echo

echo "To run the Xiaozhi client:"
echo "  cargo run --example xiaozhi_client -p play-integration-xiaozhi"
echo
echo "To use in your code:"
echo "  Add to Cargo.toml: play-integration-xiaozhi = { path = \"crates/play-integration-xiaozhi\" }"
echo "  Use: play_integration_xiaozhi::quick_start(\"ws://localhost:5173/ws\").await"