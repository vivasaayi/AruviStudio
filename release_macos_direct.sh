#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RELEASE_ROOT="${ARUVI_RELEASE_ROOT:-/Users/rajanpanneerselvam/work/releases}"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
OUTPUT_DIR="${ARUVI_DIRECT_RELEASE_DIR:-$RELEASE_ROOT/direct/$STAMP}"
APP_NAME="AruviStudio"
BUILD_CONFIG="src-tauri/tauri.direct-release.conf.json"
BUNDLES="${ARUVI_DIRECT_BUNDLES:-app,dmg}"
BUILD_APP_PATH="$ROOT_DIR/src-tauri/target/release/bundle/macos/$APP_NAME.app"
BUILD_DMG_DIR="$ROOT_DIR/src-tauri/target/release/bundle/dmg"

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Missing required command: $1" >&2
    exit 1
  fi
}

require_command npm
require_command git

has_api_notary_credentials() {
  [[ -n "${APPLE_API_ISSUER:-}" && -n "${APPLE_API_KEY:-}" && -n "${APPLE_API_KEY_PATH:-}" ]]
}

has_apple_id_notary_credentials() {
  [[ -n "${APPLE_ID:-}" && -n "${APPLE_PASSWORD:-}" && -n "${APPLE_TEAM_ID:-}" ]]
}

has_signing_certificate() {
  [[ -n "${APPLE_SIGNING_IDENTITY:-}" ]] || [[ -n "${APPLE_CERTIFICATE:-}" && -n "${APPLE_CERTIFICATE_PASSWORD:-}" ]]
}

TAURI_ARGS=(run tauri build -- --config "$BUILD_CONFIG" --bundles "$BUNDLES")
SIGNING_MODE="signed-notarized"

if [[ "${ARUVI_SKIP_SIGNING:-0}" == "1" ]]; then
  TAURI_ARGS+=(--no-sign)
  SIGNING_MODE="unsigned"
else
  if ! has_signing_certificate; then
    echo "Set APPLE_SIGNING_IDENTITY for a local Developer ID certificate, or APPLE_CERTIFICATE and APPLE_CERTIFICATE_PASSWORD for an exported .p12 certificate." >&2
    echo "Use ARUVI_SKIP_SIGNING=1 only for an unsigned smoke-test build." >&2
    exit 1
  fi

  if ! has_api_notary_credentials && ! has_apple_id_notary_credentials; then
    echo "Set notarization credentials: either APPLE_API_ISSUER, APPLE_API_KEY, APPLE_API_KEY_PATH or APPLE_ID, APPLE_PASSWORD, APPLE_TEAM_ID." >&2
    exit 1
  fi

  if [[ "${ARUVI_SKIP_STAPLING:-0}" == "1" ]]; then
    TAURI_ARGS+=(--skip-stapling)
    SIGNING_MODE="signed-notarized-unstapled"
  fi
fi

export MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-10.15}"
export CMAKE_OSX_DEPLOYMENT_TARGET="${CMAKE_OSX_DEPLOYMENT_TARGET:-$MACOSX_DEPLOYMENT_TARGET}"
export CFLAGS="${CFLAGS:--mmacosx-version-min=$MACOSX_DEPLOYMENT_TARGET}"
export CXXFLAGS="${CXXFLAGS:--mmacosx-version-min=$MACOSX_DEPLOYMENT_TARGET}"

rm -rf "$BUILD_APP_PATH"
mkdir -p "$OUTPUT_DIR"

(
  cd "$ROOT_DIR"
  npm "${TAURI_ARGS[@]}"
)

if [[ ! -d "$BUILD_APP_PATH" ]]; then
  echo "Expected app bundle was not created: $BUILD_APP_PATH" >&2
  exit 1
fi

rm -rf "$OUTPUT_DIR/$APP_NAME.app"
cp -R "$BUILD_APP_PATH" "$OUTPUT_DIR/"

if [[ -d "$BUILD_DMG_DIR" ]]; then
  find "$BUILD_DMG_DIR" -maxdepth 1 -name "*.dmg" -type f -exec cp {} "$OUTPUT_DIR/" \;
fi

APP_ARTIFACT="$OUTPUT_DIR/$APP_NAME.app"
DMG_ARTIFACT="$(find "$OUTPUT_DIR" -maxdepth 1 -name "*.dmg" -type f -print -quit)"

if [[ "$SIGNING_MODE" != "unsigned" && "${ARUVI_SKIP_VERIFY:-0}" != "1" ]]; then
  codesign --verify --deep --strict --verbose=2 "$APP_ARTIFACT"
  spctl -a -vv -t install "$APP_ARTIFACT"

  if [[ "${ARUVI_SKIP_STAPLING:-0}" != "1" ]]; then
    xcrun stapler validate "$APP_ARTIFACT"
    if [[ -n "$DMG_ARTIFACT" ]]; then
      xcrun stapler validate "$DMG_ARTIFACT"
    fi
  fi
fi

GIT_SHA="$(git -C "$ROOT_DIR" rev-parse --short HEAD 2>/dev/null || echo unknown)"
CREATED_AT="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
MANIFEST="$OUTPUT_DIR/aruvi_macos_direct_release_manifest.json"

cat >"$MANIFEST" <<EOF
{
  "name": "$APP_NAME",
  "bundleIdentifier": "com.aruvi.studio",
  "createdAt": "$CREATED_AT",
  "gitSha": "$GIT_SHA",
  "signingMode": "$SIGNING_MODE",
  "bundles": "$BUNDLES",
  "outputDir": "$OUTPUT_DIR",
  "app": "$APP_ARTIFACT",
  "dmg": "$DMG_ARTIFACT"
}
EOF

echo "macOS direct release built:"
echo "  Mode: $SIGNING_MODE"
echo "  Dir:  $OUTPUT_DIR"
echo "  App:  $APP_ARTIFACT"
if [[ -n "$DMG_ARTIFACT" ]]; then
  echo "  DMG:  $DMG_ARTIFACT"
fi
