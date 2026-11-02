#!/usr/bin/env bash
# Sign, notarize and staple a DMG. Tauri already did the same for the app
# inside it; notarizing the outer container is what Apple asks of downloads.
#
# Needs the same values Tauri's build uses: APPLE_SIGNING_IDENTITY,
# APPLE_API_KEY (key ID), APPLE_API_ISSUER and APPLE_API_KEY_PATH (.p8 file).
#
#   scripts/release/macos-notarize-dmg.sh path/to/App.dmg
set -euo pipefail

dmg="$1"
: "${APPLE_SIGNING_IDENTITY:?}" "${APPLE_API_KEY:?}" "${APPLE_API_ISSUER:?}" "${APPLE_API_KEY_PATH:?}"

codesign --force --timestamp --sign "$APPLE_SIGNING_IDENTITY" "$dmg"

result=$(xcrun notarytool submit "$dmg" \
  --key "$APPLE_API_KEY_PATH" --key-id "$APPLE_API_KEY" --issuer "$APPLE_API_ISSUER" \
  --wait --timeout 30m --output-format json)
echo "$result"

# notarytool can exit 0 for a rejected submission, so read the status itself
status=$(plutil -extract status raw -o - - <<<"$result")
if [[ "$status" != "Accepted" ]]; then
  id=$(plutil -extract id raw -o - - <<<"$result")
  xcrun notarytool log "$id" \
    --key "$APPLE_API_KEY_PATH" --key-id "$APPLE_API_KEY" --issuer "$APPLE_API_ISSUER" || true
  echo "notarization status: $status" >&2
  exit 1
fi

xcrun stapler staple "$dmg"
