#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"
source "$root/scripts/test-fixture-env.sh"
name="${project}-ci-fixture-$$"
browser="${project}-ci-browser-$$"
cleanup() { docker rm -f "$browser" "$name" >/dev/null 2>&1 || true; docker volume rm "$TEST_SOCKET_NAME" "$TEST_PDF_SOCKET_NAME" >/dev/null 2>&1 || true; }
trap cleanup EXIT
"${fixture_compose[@]}" run --rm -d --name "$name" fixture >/dev/null
source "$root/scripts/test-ci-fixture-setup.sh"
"${fixture_compose[@]}" run --rm -d --name "$browser" -e "ARUVI_REMOTE_URL=http://${name}:8787" browser sh -c 'node /project/e2e-remote/mock-ci.mjs' >/dev/null
docker exec "$browser" sh -c 'cp /project/package.json /project/package-lock.json /project/playwright.remote.config.js ./ && cp -R /project/e2e-remote ./ && npm ci --silent && npx playwright test -c playwright.remote.config.js ci.spec.js'
