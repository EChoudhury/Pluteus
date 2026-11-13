#!/bin/sh
set -eu

prefix="${1:-$HOME/.local}"
here="$(cd "$(dirname "$0")" && pwd)"

install -Dm755 "$here/pluteus" "$prefix/bin/pluteus"
install -Dm644 "$here/pluteus.png" \
    "$prefix/share/icons/hicolor/512x512/apps/pluteus.png"
install -Dm644 "$here/pluteus.desktop" \
    "$prefix/share/applications/pluteus.desktop"

echo "Installed Pluteus to $prefix"
