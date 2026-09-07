#!/usr/bin/env bash
set -euo pipefail

: "${ANDROID_KEYSTORE_BASE64:?Missing Android keystore}"
: "${ANDROID_KEY_ALIAS:?Missing Android key alias}"
: "${ANDROID_KEYSTORE_PASSWORD:?Missing Android keystore password}"
: "${ANDROID_KEY_PASSWORD:?Missing Android key password}"
: "${ANDROID_HOME:?Missing Android SDK}"

signing_dir=$(mktemp -d)
trap 'rm -rf "$signing_dir"' EXIT
chmod 700 "$signing_dir"
printf '%s' "$ANDROID_KEYSTORE_BASE64" | base64 --decode > "$signing_dir/release.jks"
chmod 600 "$signing_dir/release.jks"

build_tools="$ANDROID_HOME/build-tools/36.0.0"
unsigned_apk=src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release-unsigned.apk
"$build_tools/zipalign" -P 16 -f 4 "$unsigned_apk" "$signing_dir/aligned.apk"
mkdir "$signing_dir/signed"
"$build_tools/apksigner" sign \
  --ks "$signing_dir/release.jks" --ks-key-alias "$ANDROID_KEY_ALIAS" \
  --ks-pass env:ANDROID_KEYSTORE_PASSWORD --key-pass env:ANDROID_KEY_PASSWORD \
  --out "$signing_dir/signed/yomira.apk" "$signing_dir/aligned.apk"
"$build_tools/apksigner" verify --verbose "$signing_dir/signed/yomira.apk"
"$build_tools/zipalign" -c -P 16 4 "$signing_dir/signed/yomira.apk"
node scripts/release/assets.mjs android-arm "$signing_dir/signed" release-assets
