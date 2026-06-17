#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RELEASE_ROOT="${ARUVI_RELEASE_ROOT:-/Users/rajanpanneerselvam/work/releases}"

APP_NAME="AruviStudio Local"
APP_ID="com.aruvi.studio.localrelease"
PROFILE="local-release"
BUILD_CONFIG="src-tauri/tauri.local-release.conf.json"
BUILD_APP_PATH="$ROOT_DIR/src-tauri/target/release/bundle/macos/$APP_NAME.app"
APP_DEST="$RELEASE_ROOT/$APP_NAME.app"
DATA_DIR="${ARUVI_LOCAL_RELEASE_DATA_DIR:-$RELEASE_ROOT/aruvi-studio-local-data}"
DB_PATH="${ARUVI_LOCAL_RELEASE_DB_PATH:-$DATA_DIR/aruvi_studio.db}"
MCP_HOST="${ARUVI_LOCAL_RELEASE_MCP_HOST:-127.0.0.1}"
MCP_PORT="${ARUVI_LOCAL_RELEASE_MCP_PORT:-8788}"
RUNNER="$RELEASE_ROOT/run_aruvi_local_release.command"
MCP_DESCRIPTOR="$RELEASE_ROOT/mcp_aruvi_local_release.json"
MANIFEST="$RELEASE_ROOT/aruvi_local_release_manifest.json"

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Missing required command: $1" >&2
    exit 1
  fi
}

require_command npm
require_command git

mkdir -p "$RELEASE_ROOT" "$DATA_DIR"

if [[ -n "${ARUVI_LOCAL_RELEASE_SEED_DB:-}" && ! -f "$DB_PATH" ]]; then
  if [[ ! -f "$ARUVI_LOCAL_RELEASE_SEED_DB" ]]; then
    echo "Seed database does not exist: $ARUVI_LOCAL_RELEASE_SEED_DB" >&2
    exit 1
  fi
  cp "$ARUVI_LOCAL_RELEASE_SEED_DB" "$DB_PATH"
fi

rm -rf "$BUILD_APP_PATH" "$APP_DEST"

export MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-10.15}"
export CMAKE_OSX_DEPLOYMENT_TARGET="${CMAKE_OSX_DEPLOYMENT_TARGET:-$MACOSX_DEPLOYMENT_TARGET}"
export CFLAGS="${CFLAGS:--mmacosx-version-min=$MACOSX_DEPLOYMENT_TARGET}"
export CXXFLAGS="${CXXFLAGS:--mmacosx-version-min=$MACOSX_DEPLOYMENT_TARGET}"

(
  cd "$ROOT_DIR"
  npm run tauri build -- --config "$BUILD_CONFIG" --bundles app --no-sign
)

if [[ ! -d "$BUILD_APP_PATH" ]]; then
  echo "Expected app bundle was not created: $BUILD_APP_PATH" >&2
  exit 1
fi

cp -R "$BUILD_APP_PATH" "$APP_DEST"

cat >"$RUNNER" <<EOF
#!/usr/bin/env bash
set -euo pipefail

RELEASE_ROOT="\$(cd "\$(dirname "\${BASH_SOURCE[0]}")" && pwd)"
APP_PATH="\$RELEASE_ROOT/$APP_NAME.app"

export ARUVI_PROFILE="$PROFILE"
export ARUVI_APP_DATA_DIR="$DATA_DIR"
export ARUVI_DB_PATH="$DB_PATH"
export ARUVI_WEBHOOK_HOST="$MCP_HOST"
export ARUVI_WEBHOOK_PORT="$MCP_PORT"
export ARUVI_KEYCHAIN_SERVICE="$APP_ID"

EXECUTABLE_NAME="\$(/usr/libexec/PlistBuddy -c 'Print :CFBundleExecutable' "\$APP_PATH/Contents/Info.plist")"
exec "\$APP_PATH/Contents/MacOS/\$EXECUTABLE_NAME"
EOF
chmod +x "$RUNNER"

cat >"$MCP_DESCRIPTOR" <<EOF
{
  "name": "AruviStudio Local MCP",
  "endpoint": "http://$MCP_HOST:$MCP_PORT/api/mcp",
  "app": "$APP_DEST",
  "launcher": "$RUNNER",
  "profile": "$PROFILE",
  "bundleIdentifier": "$APP_ID",
  "appDataDir": "$DATA_DIR",
  "database": "$DB_PATH"
}
EOF

GIT_SHA="$(git -C "$ROOT_DIR" rev-parse --short HEAD 2>/dev/null || echo unknown)"
CREATED_AT="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

cat >"$MANIFEST" <<EOF
{
  "name": "$APP_NAME",
  "profile": "$PROFILE",
  "bundleIdentifier": "$APP_ID",
  "createdAt": "$CREATED_AT",
  "gitSha": "$GIT_SHA",
  "app": "$APP_DEST",
  "launcher": "$RUNNER",
  "mcpEndpoint": "http://$MCP_HOST:$MCP_PORT/api/mcp",
  "mcpHost": "$MCP_HOST",
  "mcpPort": $MCP_PORT,
  "appDataDir": "$DATA_DIR",
  "database": "$DB_PATH",
  "seedDatabase": "${ARUVI_LOCAL_RELEASE_SEED_DB:-}"
}
EOF

echo "Local release built:"
echo "  App:      $APP_DEST"
echo "  Launcher: $RUNNER"
echo "  MCP:      http://$MCP_HOST:$MCP_PORT/api/mcp"
echo "  DB:       $DB_PATH"
