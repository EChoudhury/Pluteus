#!/usr/bin/env bash
set -euo pipefail

# Usage: bundle.sh <path-to-pluteus-binary> <version> <output.app>
binary="$1"
version="$2"
app="${3:-Pluteus.app}"

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
icons="$(dirname "$binary")/pluteus-icons"

rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"

cp "$binary" "$app/Contents/MacOS/pluteus"
chmod +x "$app/Contents/MacOS/pluteus"

sed "s/__VERSION__/${version}/g" "$root/packaging/macos/Info.plist" \
    > "$app/Contents/Info.plist"

cp "$icons/Pluteus.icns" "$app/Contents/Resources/Pluteus.icns"
cp "$root/README.md" "$app/Contents/Resources/README.md"
cp "$root/assets/FiraSans-OFL.txt" "$app/Contents/Resources/FiraSans-OFL.txt"
cp "$root/assets/Montserrat-OFL.txt" "$app/Contents/Resources/Montserrat-OFL.txt"

if command -v codesign >/dev/null 2>&1; then
    codesign --force --deep --sign - "$app"
fi
