#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  ./rotate_version.sh
  ./rotate_version.sh <patch|minor|major|X.Y.Z> [--no-commit] [--no-tag] [--no-backup] [--dry-run]

Examples:
  ./rotate_version.sh
  ./rotate_version.sh 0.2.0
  ./rotate_version.sh minor --dry-run
EOF
}

if [[ "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

if [[ $# -gt 0 && "${1:-}" != --* ]]; then
  BUMP="$1"
  shift
else
  BUMP="patch"
fi

COMMIT=1
TAG=1
BACKUP=1
DRY_RUN=0

for ARG in "$@"; do
  case "$ARG" in
    --no-commit) COMMIT=0 ;;
    --no-tag) TAG=0 ;;
    --no-backup) BACKUP=0 ;;
    --dry-run) DRY_RUN=1 ;;
    *)
      echo "Unknown argument: $ARG" >&2
      usage
      exit 1
      ;;
  esac
done

if [[ "$TAG" -eq 1 && "$COMMIT" -ne 1 ]]; then
  echo "Use --commit with --tag so the tag points at the version bump commit." >&2
  exit 1
fi

CURRENT_VERSION="$(node -p "require('./package.json').version")"
IFS=. read -r MAJOR MINOR PATCH <<<"$CURRENT_VERSION"

case "$BUMP" in
  major) NEXT_VERSION="$((MAJOR + 1)).0.0" ;;
  minor) NEXT_VERSION="${MAJOR}.$((MINOR + 1)).0" ;;
  patch) NEXT_VERSION="${MAJOR}.${MINOR}.$((PATCH + 1))" ;;
  [0-9]*.[0-9]*.[0-9]*) NEXT_VERSION="$BUMP" ;;
  *)
    echo "Invalid version bump: $BUMP" >&2
    usage
    exit 1
    ;;
esac

TAG_NAME="v${NEXT_VERSION}"

if git rev-parse -q --verify "refs/tags/${TAG_NAME}" >/dev/null; then
  echo "Git tag already exists: $TAG_NAME" >&2
  exit 1
fi

if [[ "$DRY_RUN" -eq 1 ]]; then
  echo "Would rotate version ${CURRENT_VERSION} -> ${NEXT_VERSION}"
  echo "Would update package.json, package-lock.json, src-tauri/tauri.conf.json, src-tauri/Cargo.toml, src-tauri/Cargo.lock"
  [[ "$BACKUP" -eq 1 ]] && echo "Would run backup.sh" || echo "Would skip database backup"
  [[ "$COMMIT" -eq 1 ]] && echo "Would commit: chore: release ${TAG_NAME}"
  [[ "$TAG" -eq 1 ]] && echo "Would create git tag: ${TAG_NAME}"
  exit 0
fi

if [[ "$BACKUP" -eq 1 ]]; then
  ARUVI_BACKUP_LABEL="${ARUVI_BACKUP_LABEL:-v${CURRENT_VERSION}}" bash backup.sh
fi

node -e "
const fs = require('fs');
const version = process.argv[1];
for (const file of ['package.json', 'package-lock.json', 'src-tauri/tauri.conf.json']) {
  const json = JSON.parse(fs.readFileSync(file, 'utf8'));
  json.version = version;
  if (file === 'package-lock.json' && json.packages && json.packages['']) {
    json.packages[''].version = version;
  }
  fs.writeFileSync(file, JSON.stringify(json, null, 2) + '\n');
}
" "$NEXT_VERSION"

perl -0pi -e "s/(^name = \"aruvi-studio\"\nversion = \")[^\"]+(\")/\${1}${NEXT_VERSION}\${2}/m" src-tauri/Cargo.toml
perl -0pi -e "s/(\\[\\[package\\]\\]\nname = \"aruvi-studio\"\nversion = \")[^\"]+(\")/\${1}${NEXT_VERSION}\${2}/m" src-tauri/Cargo.lock

echo "Version rotated: ${CURRENT_VERSION} -> ${NEXT_VERSION}"

if [[ "$COMMIT" -eq 1 ]]; then
  git add package.json package-lock.json src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock
  git commit -m "chore: release ${TAG_NAME}"
fi

if [[ "$TAG" -eq 1 ]]; then
  git tag -a "$TAG_NAME" -m "AruviStudio ${NEXT_VERSION}"
  echo "Created git tag: $TAG_NAME"
fi
