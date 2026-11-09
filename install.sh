#!/bin/sh
# Installs the latest prsito release into /Applications (macOS) or ~/.local/bin (Linux).
# curl -fsSL https://raw.githubusercontent.com/casassg/prsito/main/install.sh | sh
set -eu

tmp=$(mktemp -d)
if [ "$(uname -s)" = Linux ] && [ "$(uname -m)" = x86_64 ]; then
  trap 'rm -rf "$tmp"' EXIT
  bin="${XDG_BIN_HOME:-$HOME/.local/bin}"
  mkdir -p "$bin"
  curl -fsSL https://github.com/casassg/prsito/releases/latest/download/prsito-linux-x86_64.tar.gz | tar -xz -C "$tmp"
  mv "$tmp/prsito" "$bin/prsito"
  echo "installed $bin/prsito"
  exit 0
fi
[ "$(uname -s)" = Darwin ] && [ "$(uname -m)" = arm64 ] || { echo "prsito needs an Apple silicon Mac or x86_64 Linux" >&2; exit 1; }

trap 'hdiutil detach -quiet "$tmp/mnt" 2>/dev/null || true; rm -rf "$tmp"' EXIT
curl -fsSL -o "$tmp/prsito.dmg" https://github.com/casassg/prsito/releases/latest/download/prsito.dmg
hdiutil attach -quiet -nobrowse -mountpoint "$tmp/mnt" "$tmp/prsito.dmg"
osascript -e 'quit app "prsito"' >/dev/null 2>&1 || true
rm -rf /Applications/prsito.app
cp -R "$tmp/mnt/prsito.app" /Applications/prsito.app
echo "installed /Applications/prsito.app"
