#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TAURI_DIR="${ROOT_DIR}/src-tauri"

if [[ ! -d "${TAURI_DIR}" ]]; then
  echo "src-tauri directory not found at ${TAURI_DIR}" >&2
  exit 1
fi

if ! command -v node >/dev/null 2>&1; then
  echo "node is required to read ~/.aruvistudio/llm-config.json" >&2
  exit 1
fi

API_KEY="${ARUVI_LIVE_API_KEY:-}"
if [[ -z "${API_KEY}" ]]; then
  API_KEY="$(node -e 'const fs=require("fs"); const p=process.env.HOME+"/.aruvistudio/llm-config.json"; const o=JSON.parse(fs.readFileSync(p,"utf8")); const key=o?.api_keys?.deepseek||""; process.stdout.write(key);' 2>/dev/null || true)"
fi

if [[ -z "${API_KEY}" ]]; then
  echo "DeepSeek API key not found. Set ARUVI_LIVE_API_KEY or add api_keys.deepseek in ~/.aruvistudio/llm-config.json" >&2
  exit 1
fi

export ARUVI_LIVE_API_KEY="${API_KEY}"
export ARUVI_LIVE_BASE_URL="${ARUVI_LIVE_BASE_URL:-https://api.deepseek.com/v1}"
export ARUVI_LIVE_MODEL="${ARUVI_LIVE_MODEL:-deepseek-chat}"
export ARUVI_LIVE_ITERATIONS="${ARUVI_LIVE_ITERATIONS:-1}"
export ARUVI_LIVE_COMPLETE_TO_DONE="${ARUVI_LIVE_COMPLETE_TO_DONE:-false}"
export ARUVI_LIVE_STAGE_TIMEOUT_SECS="${ARUVI_LIVE_STAGE_TIMEOUT_SECS:-1800}"
export ARUVI_LIVE_KEEP_TEMP="${ARUVI_LIVE_KEEP_TEMP:-true}"

echo "Running live calculator workflow smoke test..."
echo "  BASE_URL=${ARUVI_LIVE_BASE_URL}"
echo "  MODEL=${ARUVI_LIVE_MODEL}"
echo "  ITERATIONS=${ARUVI_LIVE_ITERATIONS}"
echo "  COMPLETE_TO_DONE=${ARUVI_LIVE_COMPLETE_TO_DONE}"
echo "  KEEP_TEMP=${ARUVI_LIVE_KEEP_TEMP}"

cd "${TAURI_DIR}"
cargo test live_calculator_iterative_workflow_smoke -- --ignored --nocapture
