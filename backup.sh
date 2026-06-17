#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEFAULT_DB_PATH="/Users/rajanpanneerselvam/Library/Application Support/com.aruvi.studio/aruvi_studio.db"
if [[ -f "$SCRIPT_DIR/livedb_path" ]]; then
  DEFAULT_DB_PATH="$(<"$SCRIPT_DIR/livedb_path")"
fi

SOURCE_DB="${ARUVI_BACKUP_SOURCE_DB:-${ARUVI_DB_PATH:-$DEFAULT_DB_PATH}}"
BACKUP_DIR="${ARUVI_BACKUP_DIR:-/Users/rajanpanneerselvam/Documents/work-backups/AruviStudio}"
BACKUP_LABEL="${ARUVI_BACKUP_LABEL:-}"
if [[ -n "$BACKUP_LABEL" ]]; then
  BACKUP_LABEL="_$(printf '%s' "$BACKUP_LABEL" | tr -c '[:alnum:]._- ' '_' | tr ' ' '_')"
fi
TIMESTAMP="$(date '+%Y-%m-%d_%H-%M-%S')"
BACKUP_FILE="${BACKUP_DIR}/aruvi-studio_backup${BACKUP_LABEL}_${TIMESTAMP}.db"

if [[ ! -f "$SOURCE_DB" ]]; then
  echo "Source database does not exist: $SOURCE_DB" >&2
  exit 1
fi

mkdir -p "$BACKUP_DIR"
if command -v sqlite3 >/dev/null 2>&1; then
  sqlite3 "$SOURCE_DB" ".backup '$BACKUP_FILE'"
else
  cp "$SOURCE_DB" "$BACKUP_FILE"
  if [[ -f "${SOURCE_DB}-wal" ]]; then
    cp "${SOURCE_DB}-wal" "${BACKUP_FILE}-wal"
  fi
  if [[ -f "${SOURCE_DB}-shm" ]]; then
    cp "${SOURCE_DB}-shm" "${BACKUP_FILE}-shm"
  fi
fi

echo "Backup created: $BACKUP_FILE"
