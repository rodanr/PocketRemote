#!/usr/bin/env bash
# Builds PocketRemote.app (a menu-bar app, no Dock icon) and a drag-to-Applications .dmg.
# Nothing paid: the app is ad-hoc signed, not notarized. Users approve it once under
# System Settings > Privacy & Security > "Open Anyway" (see docs/INSTALL.md).
# Usage: scripts/build-macos.sh        (output: dist/PocketRemote.app and dist/PocketRemote-<version>.dmg)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' desktop/Cargo.toml | head -1)"
APP="dist/PocketRemote.app"
DMG="dist/PocketRemote-${VERSION}.dmg"

echo "==> building release binary (feature gui)"
(cd desktop && cargo build --release --locked --features gui --bin pocketremote-host)

echo "==> assembling ${APP}"
rm -rf "$APP" dist/dmg-root "$DMG"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp desktop/target/release/pocketremote-host "$APP/Contents/MacOS/PocketRemote"
cp brand/kit/dist/macos/AppIcon.icns "$APP/Contents/Resources/AppIcon.icns"
cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleName</key><string>PocketRemote</string>
  <key>CFBundleDisplayName</key><string>PocketRemote</string>
  <key>CFBundleIdentifier</key><string>com.pocketremote.host</string>
  <key>CFBundleExecutable</key><string>PocketRemote</string>
  <key>CFBundleIconFile</key><string>AppIcon</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>${VERSION}</string>
  <key>CFBundleVersion</key><string>${VERSION}</string>
  <key>LSMinimumSystemVersion</key><string>12.0</string>
  <key>LSUIElement</key><true/>
  <key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST

echo "==> ad-hoc signing"
codesign --force --deep --sign - --identifier com.pocketremote.host "$APP"
codesign --verify --deep --strict "$APP"

echo "==> building ${DMG}"
mkdir -p dist/dmg-root
cp -R "$APP" dist/dmg-root/
ln -s /Applications dist/dmg-root/Applications
hdiutil create -quiet -volname "PocketRemote" -srcfolder dist/dmg-root -ov -format UDZO "$DMG"
rm -rf dist/dmg-root

echo "done: $APP and $DMG"
