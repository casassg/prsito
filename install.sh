#!/bin/sh
# Installs the latest prsito release into /Applications (macOS) or ~/.local/bin (Linux).
# curl -fsSL https://raw.githubusercontent.com/casassg/prsito/main/install.sh | sh
set -eu

repo=https://github.com/casassg/prsito
say() { printf '%s\n' "$*"; }
fail() { printf 'prsito: %s\n' "$*" >&2; exit 1; }
progress=-s
[ -t 2 ] && progress=-#

case "$(uname -s)/$(uname -m)" in
  Linux/x86_64) asset=prsito-linux-x86_64.tar.gz ;;
  Darwin/arm64) asset=prsito.dmg ;;
  *) fail "needs an Apple silicon Mac or x86_64 Linux, this is $(uname -s) $(uname -m)" ;;
esac

# The tag from the releases/latest redirect, so the version shown is the one downloaded.
tag=$(curl -fsS -o /dev/null -w '%{redirect_url}' "$repo/releases/latest" | sed -n 's|.*/tag/||p')
[ -n "$tag" ] || fail "can't find the latest release at $repo/releases"
say "installing prsito $tag ($asset from $repo/releases/tag/$tag)"

tmp=$(mktemp -d)
trap 'hdiutil detach -quiet "$tmp/mnt" 2>/dev/null || true; rm -rf "$tmp"' EXIT
curl -fSL "$progress" -o "$tmp/$asset" "$repo/releases/download/$tag/$asset" || fail "download failed"

if [ "$asset" != prsito.dmg ]; then
  bin="${XDG_BIN_HOME:-$HOME/.local/bin}"
  say "unpacking into $bin"
  mkdir -p "$bin"
  tar -xzf "$tmp/$asset" -C "$tmp"
  mv "$tmp/prsito" "$bin/prsito"
  say "installed prsito $tag at $bin/prsito"
  case ":$PATH:" in *":$bin:"*) ;; *) say "note: $bin isn't on your PATH" ;; esac
  exit 0
fi

app=/Applications/prsito.app
say "mounting the dmg"
hdiutil attach -quiet -nobrowse -readonly -mountpoint "$tmp/mnt" "$tmp/$asset"
if [ -d "$app" ]; then
  was=$(defaults read "$app/Contents/Info" CFBundleShortVersionString 2>/dev/null || echo unknown)
  say "replacing $app (was $was)"
  pgrep -xq prsito && say "quitting the running prsito" && { osascript -e 'quit app "prsito"' >/dev/null 2>&1 || true; }
  rm -rf "$app"
else
  say "copying to $app"
fi
cp -R "$tmp/mnt/prsito.app" "$app"
say "installed prsito $tag at $app, it updates itself from here on"
