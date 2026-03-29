#!/bin/bash
set -euo pipefail

# ── Config ──
APP_NAME="Curl Helper"
APP_NAME_NOSPACE="CurlHelper"
BUNDLE_ID="com.curl-helper.app"
BINARY_NAME="curl-helper"
VERSION="0.1.0"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
BUILD_DIR="$PROJECT_DIR/target/release"
BUNDLE_DIR="$BUILD_DIR/${APP_NAME}.app"
DMG_DIR="$BUILD_DIR/dmg"

echo "==> Building $APP_NAME v$VERSION"

# ── Step 1: Build release binary ──
echo "    Compiling release build..."
cd "$PROJECT_DIR"
cargo build --release
echo "    Build complete."

# ── Step 2: Generate icon ──
echo "    Generating app icon..."
ICON_PNG="$BUILD_DIR/icon_1024.png"
python3 "$SCRIPT_DIR/generate_icon.py" "$ICON_PNG"

# Create iconset
ICONSET_DIR="$BUILD_DIR/AppIcon.iconset"
rm -rf "$ICONSET_DIR"
mkdir -p "$ICONSET_DIR"

# Standard macOS icon sizes
sips -z   16   16 "$ICON_PNG" --out "$ICONSET_DIR/icon_16x16.png"      >/dev/null
sips -z   32   32 "$ICON_PNG" --out "$ICONSET_DIR/icon_16x16@2x.png"   >/dev/null
sips -z   32   32 "$ICON_PNG" --out "$ICONSET_DIR/icon_32x32.png"      >/dev/null
sips -z   64   64 "$ICON_PNG" --out "$ICONSET_DIR/icon_32x32@2x.png"   >/dev/null
sips -z  128  128 "$ICON_PNG" --out "$ICONSET_DIR/icon_128x128.png"    >/dev/null
sips -z  256  256 "$ICON_PNG" --out "$ICONSET_DIR/icon_128x128@2x.png" >/dev/null
sips -z  256  256 "$ICON_PNG" --out "$ICONSET_DIR/icon_256x256.png"    >/dev/null
sips -z  512  512 "$ICON_PNG" --out "$ICONSET_DIR/icon_256x256@2x.png" >/dev/null
sips -z  512  512 "$ICON_PNG" --out "$ICONSET_DIR/icon_512x512.png"    >/dev/null
sips -z 1024 1024 "$ICON_PNG" --out "$ICONSET_DIR/icon_512x512@2x.png" >/dev/null

# Convert to icns
ICNS_FILE="$BUILD_DIR/AppIcon.icns"
iconutil -c icns "$ICONSET_DIR" -o "$ICNS_FILE"
echo "    Icon created."

# ── Step 3: Create .app bundle ──
echo "    Creating app bundle..."
rm -rf "$BUNDLE_DIR"
mkdir -p "$BUNDLE_DIR/Contents/MacOS"
mkdir -p "$BUNDLE_DIR/Contents/Resources"

# Copy binary
cp "$BUILD_DIR/$BINARY_NAME" "$BUNDLE_DIR/Contents/MacOS/"

# Copy icon
cp "$ICNS_FILE" "$BUNDLE_DIR/Contents/Resources/AppIcon.icns"

# Write Info.plist
cat > "$BUNDLE_DIR/Contents/Info.plist" << PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>${APP_NAME}</string>
    <key>CFBundleDisplayName</key>
    <string>${APP_NAME}</string>
    <key>CFBundleIdentifier</key>
    <string>${BUNDLE_ID}</string>
    <key>CFBundleVersion</key>
    <string>${VERSION}</string>
    <key>CFBundleShortVersionString</key>
    <string>${VERSION}</string>
    <key>CFBundleExecutable</key>
    <string>${BINARY_NAME}</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.15</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSSupportsAutomaticGraphicsSwitching</key>
    <true/>
</dict>
</plist>
PLIST

echo "    App bundle created: $BUNDLE_DIR"

# ── Step 4: Create DMG ──
echo "    Packaging DMG..."
DMG_STAGING="$BUILD_DIR/dmg_staging"
DMG_FILE="$BUILD_DIR/${APP_NAME_NOSPACE}-${VERSION}.dmg"

rm -rf "$DMG_STAGING"
rm -f "$DMG_FILE"
mkdir -p "$DMG_STAGING"

# Copy app to staging
cp -R "$BUNDLE_DIR" "$DMG_STAGING/"

# Create Applications symlink for drag-install
ln -s /Applications "$DMG_STAGING/Applications"

# Create DMG
hdiutil create \
    -volname "$APP_NAME" \
    -srcfolder "$DMG_STAGING" \
    -ov \
    -format UDZO \
    "$DMG_FILE"

# Cleanup staging
rm -rf "$DMG_STAGING"
rm -rf "$ICONSET_DIR"
rm -f "$ICON_PNG"

echo ""
echo "==> Done!"
echo "    App:  $BUNDLE_DIR"
echo "    DMG:  $DMG_FILE"
echo ""
echo "    You can also run directly:  open \"$BUNDLE_DIR\""
