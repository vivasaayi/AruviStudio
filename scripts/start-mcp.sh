#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
BINARY_PATH="${REPO_ROOT}/src-tauri/target/debug/aruvi-studio-mcp"

cd "${REPO_ROOT}"

if [[ ! -x "${BINARY_PATH}" ]]; then
  cargo build --manifest-path src-tauri/Cargo.toml --bin aruvi-studio-mcp
fi

exec "${BINARY_PATH}" "$@"
