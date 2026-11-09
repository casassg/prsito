#!/usr/bin/env bash
# Builds prsito.app, packs it into target/prsito.dmg (with an Applications link to drag it onto)
# and installs it into /Applications.
# Usage: scripts/release.sh [version]   (defaults to the version in Cargo.toml)
set -euo pipefail
cd "$(dirname "$0")/.."
. bin/activate-hermit >/dev/null 2>&1 || true

version="${1:-$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)}"
# Only release builds (an explicit version) learn their version and check for updates.
if [ -n "${1:-}" ]; then export PRSITO_VERSION="$1"; fi
cargo build --release

out=target/dmg
app="$out/stage/prsito.app"
rm -rf "$out"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp target/release/prsito "$app/Contents/MacOS/prsito"
cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>prsito</string>
  <key>CFBundleDisplayName</key><string>prsito</string>
  <key>CFBundleIdentifier</key><string>com.casassg.prsito</string>
  <key>CFBundleExecutable</key><string>prsito</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>${version}</string>
  <key>CFBundleVersion</key><string>${version}</string>
  <key>LSMinimumSystemVersion</key><string>13.0</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>CFBundleIconFile</key><string>AppIcon</string>
</dict>
</plist>
PLIST

# The icon: every size macOS asks for, cut from assets/icon.png (1024 px square).
iconset="$out/AppIcon.iconset"
mkdir -p "$iconset"
for size in 16 32 128 256 512; do
  sips -z $size $size assets/icon.png --out "$iconset/icon_${size}x${size}.png" >/dev/null
  sips -z $((size * 2)) $((size * 2)) assets/icon.png --out "$iconset/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$iconset" -o "$app/Contents/Resources/AppIcon.icns"
rm -rf "$iconset"

# Ad hoc signature: enough to run on this Mac and on others after "Open" from the context menu.
codesign --force --deep --sign - "$app"

ln -s /Applications "$out/stage/Applications"
rm -f target/prsito.dmg
hdiutil create -volname "prsito" -srcfolder "$out/stage" -ov -format UDZO target/prsito.dmg 2>&1 >/dev/null | { grep -v deprecated || true; } >&2

if [ -n "${CI:-}" ]; then
  rm -rf "$out"
  echo "target/prsito.dmg ($(du -h target/prsito.dmg | cut -f1))"
  exit 0
fi
osascript -e 'quit app "prsito"' >/dev/null 2>&1 || true
rm -rf /Applications/prsito.app
cp -R "$app" /Applications/prsito.app
rm -rf "$out"
echo "target/prsito.dmg ($(du -h target/prsito.dmg | cut -f1)), installed /Applications/prsito.app"
