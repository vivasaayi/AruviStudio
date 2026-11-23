#!/usr/bin/env bash
# Source this file from a fixture script after setting root.
if [[ -n "${ARUVI_TEST_PROJECT:-}" || -n "${ARUVI_TEST_ENV_FILE:-}" ]]; then
  # CI can use an explicit disposable identity without requiring the local
  # worktree launcher or reading any developer environment credentials.
  [[ "${ARUVI_TEST_PROJECT:-}" =~ ^aruvi-studio-fixture-[a-z0-9-]+$ ]] || { printf '%s\n' 'Invalid disposable fixture project name.' >&2; exit 1; }
  [[ -n "${ARUVI_TEST_ENV_FILE:-}" && -f "$ARUVI_TEST_ENV_FILE" && -n "${ARUVI_TEST_APP_IMAGE:-}" ]] || { printf '%s\n' 'Explicit fixture project requires an environment file and test image.' >&2; exit 1; }
  project="$ARUVI_TEST_PROJECT"
  env_file="$ARUVI_TEST_ENV_FILE"
else
  plan=$(bash "$root/scripts/env" worktree plan)
  project=$(python3 -c 'import json,sys; print(json.load(sys.stdin)["project"])' <<<"$plan")
  state=$(python3 -c 'import json,sys; print(json.load(sys.stdin)["state"])' <<<"$plan")
  env_file="$state/environment.env"
fi
export TEST_SOCKET_NAME="${project}_test_socket_$$"
export TEST_PDF_SOCKET_NAME="${project}_test_pdf_socket_$$"
if [[ ! -f "$env_file" ]]; then
  printf '%s\n' 'Initialize and build this worktree first: bash scripts/env worktree up' >&2
  exit 1
fi
APP_IMAGE=${ARUVI_TEST_APP_IMAGE:-$(python3 -c 'import sys; print(next(line.split("=",1)[1] for line in open(sys.argv[1]) if line.startswith("APP_IMAGE=")))' "$env_file")}
export APP_IMAGE PRODUCT_SOURCE="$root" COMPOSE_IGNORE_ORPHANS=true
if ! docker image inspect "$APP_IMAGE" >/dev/null 2>&1; then
  printf '%s\n' 'The worktree image is unavailable; run bash scripts/build worktree first.' >&2
  exit 1
fi
fixture_compose=(docker compose --project-name "$project" --project-directory "$root" --env-file "$env_file" -f "$root/deploy/compose.test.yaml")
