#!/bin/bash
set -euo pipefail

APP_NAME="Play GUI"
APP_NAME_NOSPACE="PlayGUI"
BUNDLE_ID="com.zhouzhipeng.play-gui"
BINARY_NAME="play-gui"
VERSION="0.1.0"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
WORKSPACE_DIR="$(cd "$PROJECT_DIR/../.." && pwd)"
BUILD_DIR="$WORKSPACE_DIR/target/release"
BUNDLE_DIR="$BUILD_DIR/${APP_NAME}.app"
DMG_STAGING="$BUILD_DIR/dmg_staging"
DMG_FILE="$BUILD_DIR/${APP_NAME_NOSPACE}-${VERSION}.dmg"
ICON_PNG="$PROJECT_DIR/assets/icon.png"
ICONSET_DIR="$BUILD_DIR/AppIcon.iconset"
ICNS_FILE="$BUILD_DIR/AppIcon.icns"

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "This script only supports macOS." >&2
  exit 1
fi

for tool in cargo sips iconutil hdiutil; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "Missing required tool: $tool" >&2
    exit 1
  fi
done

if [[ ! -f "$ICON_PNG" ]]; then
  echo "Missing icon file: $ICON_PNG" >&2
  exit 1
fi

echo "==> Building ${APP_NAME} v${VERSION}"

echo "    Compiling release build..."
cd "$WORKSPACE_DIR"
cargo build --release -p "$BINARY_NAME"
echo "    Build complete."

echo "    Preparing macOS icon..."
rm -rf "$ICONSET_DIR"
mkdir -p "$ICONSET_DIR"

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

iconutil -c icns "$ICONSET_DIR" -o "$ICNS_FILE"
echo "    Icon created."

echo "    Creating app bundle..."
rm -rf "$BUNDLE_DIR"
mkdir -p "$BUNDLE_DIR/Contents/MacOS"
mkdir -p "$BUNDLE_DIR/Contents/Resources"

cp "$BUILD_DIR/$BINARY_NAME" "$BUNDLE_DIR/Contents/MacOS/"
cp "$ICNS_FILE" "$BUNDLE_DIR/Contents/Resources/AppIcon.icns"

cat > "$BUNDLE_DIR/Contents/Info.plist" <<PLIST
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
</dict>
</plist>
PLIST

echo "    App bundle created: $BUNDLE_DIR"

echo "    Packaging DMG..."
rm -rf "$DMG_STAGING"
rm -f "$DMG_FILE"
mkdir -p "$DMG_STAGING"

cp -R "$BUNDLE_DIR" "$DMG_STAGING/"
ln -s /Applications "$DMG_STAGING/Applications"

hdiutil create \
  -volname "$APP_NAME" \
  -srcfolder "$DMG_STAGING" \
  -ov \
  -format UDZO \
  "$DMG_FILE"

rm -rf "$DMG_STAGING"
rm -rf "$ICONSET_DIR"
rm -f "$ICNS_FILE"

echo
echo "==> Done!"
echo "    App: $BUNDLE_DIR"
echo "    DMG: $DMG_FILE"
