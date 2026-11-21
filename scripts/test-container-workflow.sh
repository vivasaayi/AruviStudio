#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"
source "$root/scripts/test-fixture-env.sh"
name="${project}-workflow-fixture-$$"
browser="${project}-workflow-browser-$$"
cleanup() { docker rm -f "$browser" "$name" >/dev/null 2>&1 || true; docker volume rm "$TEST_SOCKET_NAME" "$TEST_PDF_SOCKET_NAME" >/dev/null 2>&1 || true; }
trap cleanup EXIT

"${fixture_compose[@]}" run --rm -d --name "$name" -e TZ=UTC -e HTTPS_PROXY=http://127.0.0.1:1 -e https_proxy=http://127.0.0.1:1 -e NO_PROXY= -e no_proxy= fixture >/dev/null
source "$root/scripts/test-workflow-fixture-setup.sh"

"${fixture_compose[@]}" run --rm -d --name "$browser" -e "ARUVI_REMOTE_URL=http://${name}:8787" -e "ARUVI_TEST_MODEL_URL=http://${browser}:19876/v1" -e "ARUVI_TEST_WHISPER_MODE=$whisper_test_mode" -e "ARUVI_TEST_LOCAL_AUDIO=${ARUVI_TEST_WHISPER_AUDIO:+/tmp/aruvi-test-speech.wav}" browser sh -c 'node /project/e2e-remote/mock-provider.mjs' >/dev/null
if [[ -n "${ARUVI_TEST_WHISPER_AUDIO:-}" ]]; then
  docker exec -i "$browser" sh -c 'umask 077; cat > /tmp/aruvi-test-speech.wav' < "$ARUVI_TEST_WHISPER_AUDIO"
fi
docker exec "$browser" sh -c 'cp /project/package.json /project/package-lock.json /project/playwright.remote.config.js ./ && cp -R /project/e2e-remote ./ && npm ci --silent && npx playwright test -c playwright.remote.config.js workflow.spec.js'

source "$root/scripts/test-workflow-fixture-checks.sh"
