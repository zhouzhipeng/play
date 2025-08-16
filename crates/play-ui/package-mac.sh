#!/bin/bash

# Advanced macOS packaging script with code signing support
# Usage: ./package-mac.sh [--sign "Developer ID Application: YOUR NAME"]

set -e

echo "📦 Advanced macOS Packaging for Play Server"
echo "==========================================="

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Configuration
APP_NAME="Play Server"
BUNDLE_ID="com.zhouzhipeng.play"
VERSION=$(grep "^version" Cargo.toml | head -1 | cut -d'"' -f2)
DMG_NAME="PlayServer-${VERSION}.dmg"
DMG_VOLUME_NAME="Play Server ${VERSION}"

# Parse arguments
SIGN_IDENTITY=""
NOTARIZE=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --sign)
            SIGN_IDENTITY="$2"
            shift 2
            ;;
        --notarize)
            NOTARIZE=true
            shift
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

echo -e "${BLUE}Version: ${VERSION}${NC}"
echo -e "${BLUE}Bundle ID: ${BUNDLE_ID}${NC}"

# Step 1: Build release binary
echo -e "\n${YELLOW}📌 Building release binary...${NC}"
cargo build --release
echo -e "${GREEN}✓ Build complete${NC}"

# Step 2: Install cargo-bundle if not present
if ! command -v cargo-bundle &> /dev/null; then
    echo -e "${YELLOW}📌 Installing cargo-bundle...${NC}"
    cargo install cargo-bundle
fi

# Step 3: Create app bundle
echo -e "\n${YELLOW}📌 Creating app bundle...${NC}"
cargo bundle --release

BUNDLE_PATH="../../target/release/bundle/osx/${APP_NAME}.app"
DMG_PATH="../../target/release/bundle/osx/${DMG_NAME}"

if [ ! -d "$BUNDLE_PATH" ]; then
    echo -e "${RED}❌ Bundle not found at ${BUNDLE_PATH}${NC}"
    exit 1
fi

echo -e "${GREEN}✓ App bundle created${NC}"

# Step 4: Code signing (optional)
if [ -n "$SIGN_IDENTITY" ]; then
    echo -e "\n${YELLOW}📌 Signing app bundle...${NC}"
    echo -e "Identity: ${SIGN_IDENTITY}"
    
    # Sign all frameworks and dylibs first
    find "$BUNDLE_PATH" -name "*.dylib" -o -name "*.framework" | while read -r lib; do
        codesign --force --deep --sign "$SIGN_IDENTITY" \
                 --options runtime \
                 --timestamp \
                 --entitlements entitlements.plist \
                 "$lib" 2>/dev/null || true
    done
    
    # Sign the main app
    codesign --force --deep --sign "$SIGN_IDENTITY" \
             --options runtime \
             --timestamp \
             --entitlements entitlements.plist \
             "$BUNDLE_PATH"
    
    # Verify signature
    codesign --verify --deep --strict "$BUNDLE_PATH"
    echo -e "${GREEN}✓ App signed successfully${NC}"
fi

# Step 5: Create DMG
echo -e "\n${YELLOW}📌 Creating DMG installer...${NC}"

# Remove old DMG if exists
[ -f "$DMG_PATH" ] && rm "$DMG_PATH"

# Create temporary directory
DMG_TEMP=$(mktemp -d)
DMG_SRC="${DMG_TEMP}/dmg"
mkdir -p "$DMG_SRC"

# Copy app bundle
cp -R "$BUNDLE_PATH" "$DMG_SRC/"

# Create Applications symlink
ln -s /Applications "$DMG_SRC/Applications"

# Create .DS_Store for custom DMG appearance (optional)
# This would require additional tooling

# Create DMG
hdiutil create -volname "${DMG_VOLUME_NAME}" \
               -srcfolder "$DMG_SRC" \
               -ov \
               -format UDZO \
               -fs HFS+ \
               -imagekey zlib-level=9 \
               "$DMG_PATH"

# Clean up
rm -rf "$DMG_TEMP"

echo -e "${GREEN}✓ DMG created${NC}"

# Step 6: Sign DMG (optional)
if [ -n "$SIGN_IDENTITY" ]; then
    echo -e "\n${YELLOW}📌 Signing DMG...${NC}"
    codesign --force --sign "$SIGN_IDENTITY" "$DMG_PATH"
    echo -e "${GREEN}✓ DMG signed${NC}"
fi

# Step 7: Notarization (optional)
if [ "$NOTARIZE" = true ] && [ -n "$SIGN_IDENTITY" ]; then
    echo -e "\n${YELLOW}📌 Notarizing DMG...${NC}"
    echo -e "${YELLOW}Note: Make sure you have set up notarization credentials${NC}"
    
    # You need to set up notarization credentials first:
    # xcrun notarytool store-credentials "notarytool-password" \
    #     --apple-id "your-apple-id@example.com" \
    #     --team-id "YOUR_TEAM_ID" \
    #     --password "app-specific-password"
    
    xcrun notarytool submit "$DMG_PATH" \
                     --keychain-profile "notarytool-password" \
                     --wait
    
    # Staple the notarization ticket
    xcrun stapler staple "$DMG_PATH"
    echo -e "${GREEN}✓ DMG notarized and stapled${NC}"
fi

# Final summary
echo -e "\n${GREEN}========================================${NC}"
echo -e "${GREEN}✅ Packaging complete!${NC}"
echo -e "${GREEN}========================================${NC}"
echo -e "📦 DMG Location: ${BLUE}${DMG_PATH}${NC}"
echo -e "📏 File Size: ${BLUE}$(du -h "$DMG_PATH" | cut -f1)${NC}"

if [ -n "$SIGN_IDENTITY" ]; then
    echo -e "🔏 Signed: ${GREEN}Yes${NC}"
else
    echo -e "🔏 Signed: ${YELLOW}No (unsigned)${NC}"
fi

if [ "$NOTARIZE" = true ] && [ -n "$SIGN_IDENTITY" ]; then
    echo -e "✅ Notarized: ${GREEN}Yes${NC}"
else
    echo -e "✅ Notarized: ${YELLOW}No${NC}"
fi

# Open in Finder
echo -e "\n${BLUE}Opening DMG location in Finder...${NC}"
open -R "$DMG_PATH"