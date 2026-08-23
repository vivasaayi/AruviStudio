#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
BINARY_PATH="${REPO_ROOT}/src-tauri/target/debug/aruvi-studio-server"

if [[ ! -x "${BINARY_PATH}" ]]; then
  cargo build --manifest-path "${REPO_ROOT}/src-tauri/Cargo.toml" --bin aruvi-studio-server
fi

exec "${BINARY_PATH}"
