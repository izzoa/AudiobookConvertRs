#!/usr/bin/env bash
# Fail unless a release DMG is ready to ship: the app inside it is signed,
# passes Gatekeeper and runs on both Apple Silicon and Intel, and the DMG
# carries a stapled notarization ticket.
#
#   scripts/release/macos-verify.sh path/to/App.dmg
set -euo pipefail

dmg="$1"
mount=$(mktemp -d)
hdiutil attach -nobrowse -readonly -mountpoint "$mount" "$dmg" >/dev/null
trap 'hdiutil detach -quiet "$mount"' EXIT

app=$(find "$mount" -maxdepth 1 -name '*.app' | head -1)
[[ -n "$app" ]] || { echo "no .app in $dmg" >&2; exit 1; }
exe="$app/Contents/MacOS/$(/usr/libexec/PlistBuddy -c 'Print :CFBundleExecutable' "$app/Contents/Info.plist")"

echo "== app signature"
codesign --verify --deep --strict --verbose=2 "$app"
echo "== Gatekeeper, app"
spctl --assess --type execute --verbose=2 "$app"
echo "== stapled ticket, dmg"
xcrun stapler validate "$dmg"
echo "== Gatekeeper, dmg"
spctl --assess --type open --context context:primary-signature --verbose=2 "$dmg"
echo "== architectures"
archs=$(lipo -archs "$exe")
echo "$archs"
[[ "$archs" == *x86_64* && "$archs" == *arm64* ]] || { echo "not universal: $archs" >&2; exit 1; }
echo "release DMG verified"
