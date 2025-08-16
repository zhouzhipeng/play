#!/bin/bash

# Build script for creating macOS DMG package

set -e

echo "🚀 Building Play Server for macOS..."

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if running on macOS
if [[ "$OSTYPE" != "darwin"* ]]; then
    echo -e "${RED}This script must be run on macOS${NC}"
    exit 1
fi

# Navigate to play-ui directory
cd "$(dirname "$0")"

# Step 1: Create ICNS file from PNG if it doesn't exist
if [ ! -f "icons/icon.icns" ]; then
    echo -e "${YELLOW}Creating ICNS file from PNG...${NC}"
    
    # Create iconset directory
    mkdir -p icons/icon.iconset
    
    # Generate different sizes (macOS requires specific sizes)
    sips -z 16 16     icons/icon.png --out icons/icon.iconset/icon_16x16.png
    sips -z 32 32     icons/icon.png --out icons/icon.iconset/icon_16x16@2x.png
    sips -z 32 32     icons/icon.png --out icons/icon.iconset/icon_32x32.png
    sips -z 64 64     icons/icon.png --out icons/icon.iconset/icon_32x32@2x.png
    sips -z 128 128   icons/icon.png --out icons/icon.iconset/icon_128x128.png
    sips -z 256 256   icons/icon.png --out icons/icon.iconset/icon_128x128@2x.png
    sips -z 256 256   icons/icon.png --out icons/icon.iconset/icon_256x256.png
    sips -z 512 512   icons/icon.png --out icons/icon.iconset/icon_256x256@2x.png
    sips -z 512 512   icons/icon.png --out icons/icon.iconset/icon_512x512.png
    sips -z 1024 1024 icons/icon.png --out icons/icon.iconset/icon_512x512@2x.png
    
    # Convert iconset to icns
    iconutil -c icns icons/icon.iconset -o icons/icon.icns
    
    # Clean up iconset
    rm -rf icons/icon.iconset
    
    echo -e "${GREEN}✓ ICNS file created${NC}"
fi

# Step 2: Build the application in release mode
echo -e "${YELLOW}Building release binary...${NC}"
cargo build --release
echo -e "${GREEN}✓ Build complete${NC}"

# Step 3: Check if cargo-bundle is installed
if ! command -v cargo-bundle &> /dev/null; then
    echo -e "${YELLOW}Installing cargo-bundle...${NC}"
    cargo install cargo-bundle
fi

# Step 4: Create app bundle
echo -e "${YELLOW}Creating app bundle...${NC}"
cargo bundle --release

# Step 5: Create DMG
echo -e "${YELLOW}Creating DMG installer...${NC}"

APP_NAME="Play Server"
DMG_NAME="PlayServer-$(grep version Cargo.toml | head -1 | cut -d'"' -f2).dmg"
BUNDLE_PATH="../../target/release/bundle/osx/${APP_NAME}.app"
DMG_PATH="../../target/release/bundle/osx/${DMG_NAME}"

if [ -d "$BUNDLE_PATH" ]; then
    # Create a temporary directory for DMG contents
    DMG_TEMP=$(mktemp -d)
    
    # Copy the app bundle
    cp -R "$BUNDLE_PATH" "$DMG_TEMP/"
    
    # Create a symbolic link to Applications
    ln -s /Applications "$DMG_TEMP/Applications"
    
    # Create DMG
    hdiutil create -volname "${APP_NAME}" \
                   -srcfolder "$DMG_TEMP" \
                   -ov \
                   -format UDZO \
                   "$DMG_PATH"
    
    # Clean up
    rm -rf "$DMG_TEMP"
    
    echo -e "${GREEN}✅ DMG created successfully: ${DMG_PATH}${NC}"
    echo -e "${GREEN}📦 File size: $(du -h "$DMG_PATH" | cut -f1)${NC}"
    
    # Open the DMG location in Finder
    open -R "$DMG_PATH"
else
    echo -e "${RED}❌ App bundle not found at $BUNDLE_PATH${NC}"
    echo -e "${YELLOW}Make sure cargo-bundle ran successfully${NC}"
    exit 1
fi