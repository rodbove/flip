#!/bin/sh
set -e
cd "$(dirname "$0")/.."
cargo build --release
APP=target/Flip.app
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS"
cp target/release/flip "$APP/Contents/MacOS/flip"
cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>Flip</string>
  <key>CFBundleDisplayName</key><string>Flip</string>
  <key>CFBundleIdentifier</key><string>app.flip.sampler</string>
  <key>CFBundleExecutable</key><string>flip</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleVersion</key><string>0.1.0</string>
  <key>CFBundleShortVersionString</key><string>0.1.0</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSMicrophoneUsageDescription</key><string>Flip records sounds from your microphone or audio input into pads.</string>
</dict>
</plist>
PLIST
codesign --force --deep --sign - "$APP"
echo "built $APP"
