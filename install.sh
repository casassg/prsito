#!/bin/sh
# Installs the latest prsito release into /Applications.
# curl -fsSL https://raw.githubusercontent.com/casassg/prsito/main/install.sh | sh
set -eu

[ "$(uname -s)" = Darwin ] && [ "$(uname -m)" = arm64 ] || { echo "prsito needs an Apple silicon Mac" >&2; exit 1; }

tmp=$(mktemp -d)
trap 'hdiutil detach -quiet "$tmp/mnt" 2>/dev/null || true; rm -rf "$tmp"' EXIT
curl -fsSL -o "$tmp/prsito.dmg" https://github.com/casassg/prsito/releases/latest/download/prsito.dmg
hdiutil attach -quiet -nobrowse -mountpoint "$tmp/mnt" "$tmp/prsito.dmg"
osascript -e 'quit app "prsito"' >/dev/null 2>&1 || true
rm -rf /Applications/prsito.app
cp -R "$tmp/mnt/prsito.app" /Applications/prsito.app
echo "installed /Applications/prsito.app"
