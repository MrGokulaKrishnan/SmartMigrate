#!/usr/bin/env bash
# Smart Migrate — Linux / CI Android Release Build & Signing Pipeline
# Reads credentials from environment variables:
#   SM_KEYSTORE_PATH (optional, defaults to apps/android-client/app/release.keystore)
#   SM_KEYSTORE_PASSWORD
#   SM_KEY_ALIAS
#   SM_KEY_PASSWORD
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
ANDROID_DIR="$REPO_ROOT/apps/android-client"
RELEASE_APK="$ANDROID_DIR/app/build/outputs/apk/release/app-release.apk"
LINEAGE_FILE="$ANDROID_DIR/app/signing_lineage"
RELEASE_KEYSTORE="${SM_KEYSTORE_PATH:-$ANDROID_DIR/app/release.keystore}"

KEYSTORE_PASS="${SM_KEYSTORE_PASSWORD:-smartmigrate_keystore_pass}"
KEY_ALIAS="${SM_KEY_ALIAS:-smartmigrate_release}"
KEY_PASS="${SM_KEY_PASSWORD:-$KEYSTORE_PASS}"

echo "=== 1. Building Android Release APK ==="
cd "$ANDROID_DIR"
./gradlew assembleRelease

echo "=== 2. Signing APK with apksigner ==="
if [ ! -f "$RELEASE_KEYSTORE" ]; then
  echo "Error: Release keystore not found at $RELEASE_KEYSTORE" >&2
  exit 1
fi

APKSIGNER="apksigner"
if ! command -v apksigner &> /dev/null; then
  if [ -n "${ANDROID_HOME:-}" ] && [ -d "$ANDROID_HOME/build-tools" ]; then
    LATEST_BUILD_TOOLS=$(find "$ANDROID_HOME/build-tools" -mindepth 1 -maxdepth 1 -type d | sort -r | head -n 1)
    if [ -x "$LATEST_BUILD_TOOLS/apksigner" ]; then
      APKSIGNER="$LATEST_BUILD_TOOLS/apksigner"
    fi
  fi
fi

if [ -f "$LINEAGE_FILE" ]; then
  "$APKSIGNER" sign \
    --ks "$RELEASE_KEYSTORE" --ks-pass "pass:$KEYSTORE_PASS" \
    --ks-key-alias "$KEY_ALIAS" --key-pass "pass:$KEY_PASS" \
    --lineage "$LINEAGE_FILE" \
    "$RELEASE_APK"
else
  "$APKSIGNER" sign \
    --ks "$RELEASE_KEYSTORE" --ks-pass "pass:$KEYSTORE_PASS" \
    --ks-key-alias "$KEY_ALIAS" --key-pass "pass:$KEY_PASS" \
    "$RELEASE_APK"
fi

echo "=== 3. Verifying Cryptographic Signatures ==="
"$APKSIGNER" verify --verbose --print-certs "$RELEASE_APK"

echo "=== 4. Staging to Website Download Directories ==="
TARGETS=(
  "$REPO_ROOT/apps/website/downloads/android/SmartMigrate.apk"
  "$REPO_ROOT/apps/website/downloads/android/SmartMigrate.apk.bin"
  "$REPO_ROOT/apps/website/public/downloads/android/SmartMigrate.apk"
  "$REPO_ROOT/apps/website/public/downloads/android/SmartMigrate.apk.bin"
)

for target in "${TARGETS[@]}"; do
  mkdir -p "$(dirname "$target")"
  cp -f "$RELEASE_APK" "$target"
  echo "Staged: $target"
done

echo "=== Android Release Pipeline Complete ==="
