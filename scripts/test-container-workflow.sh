#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"
source "$root/scripts/test-fixture-env.sh"
name="${project}-workflow-fixture-$$"
browser="${project}-workflow-browser-$$"
cleanup() { docker rm -f "$browser" "$name" >/dev/null 2>&1 || true; docker volume rm "$TEST_SOCKET_NAME" "$TEST_PDF_SOCKET_NAME" >/dev/null 2>&1 || true; }
trap cleanup EXIT

"${fixture_compose[@]}" run --rm -d --name "$name" fixture >/dev/null
docker exec "$name" sh -c 'for fixture in workflow-test workflow-test-passed workflow-test-failed workflow-test-skipped; do mkdir -p "/repositories/$fixture" && git init -q "/repositories/$fixture" && printf "test repo\n" > "/repositories/$fixture/README.md" || exit 1; done'
docker exec "$name" sh -c 'mkdir -p /repositories/occupied-workspace && printf "existing readme\n" > /repositories/occupied-workspace/README.md && printf "existing ignore\n" > /repositories/occupied-workspace/.gitignore && ln -s occupied-workspace /repositories/alias-workspace'
docker exec "$name" sh -c 'git init -q --bare /repositories/mcp-origin.git && git init -q -b main /repositories/mcp-fixture && git -C /repositories/mcp-fixture config user.name "Aruvi Test" && git -C /repositories/mcp-fixture config user.email "aruvi-test@example.invalid" && printf "MCP fixture\n" > /repositories/mcp-fixture/README.md && git -C /repositories/mcp-fixture add README.md && git -C /repositories/mcp-fixture commit -q -m initial && git -C /repositories/mcp-fixture remote add origin /repositories/mcp-origin.git && git -C /repositories/mcp-fixture push -q origin main'
docker exec "$name" sh -c 'umask 077; printf "%s" "{\"refs\":{\"synthetic-external-reference-with-hyphens\":\"synthetic-legacy-key-12345\"}}" > /data/llm-config.json'
docker exec "$name" sh -c 'mkdir -p /data/models/fixture && printf "synthetic model" > /data/models/fixture/existing.bin'
docker exec "$name" sh -c 'mkdir -p /data/models/whisper-cpp-tiny-en-local'
if [[ "${ARUVI_TEST_WHISPER_DOWNLOAD:-}" == 1 ]]; then
  whisper_test_mode=download
elif [[ -n "${ARUVI_TEST_WHISPER_MODEL:-}" ]]; then
  docker exec -i "$name" sh -c 'cat > /data/models/whisper-cpp-tiny-en-local/ggml-tiny.en.bin' < "$ARUVI_TEST_WHISPER_MODEL"
  whisper_test_mode=existing
else
  docker exec "$name" sh -c 'printf "synthetic preset" > /data/models/whisper-cpp-tiny-en-local/ggml-tiny.en.bin'
  whisper_test_mode=corrupt
fi
docker exec "$name" sh -c 'git init -q --bare /repositories/push-origin.git && git init -q -b main /repositories/push-fixture && git -C /repositories/push-fixture config user.name "Aruvi Test" && git -C /repositories/push-fixture config user.email "aruvi-test@example.invalid" && printf "base fixture\n" > /repositories/push-fixture/README.md && git -C /repositories/push-fixture add README.md && git -C /repositories/push-fixture commit -q -m "Initial fixture" && git -C /repositories/push-fixture remote add origin /repositories/push-origin.git && git -C /repositories/push-fixture push -q origin main'
docker exec "$name" sh -c 'printf "unrelated local edit\n" >> /repositories/push-fixture/README.md && printf "untracked user data\n" > /repositories/push-fixture/user-untracked.txt'
docker exec "$name" sh -c 'for fixture in staged changed remote; do git init -q --bare "/repositories/push-$fixture-origin.git" && git init -q -b main "/repositories/push-$fixture-fixture" && git -C "/repositories/push-$fixture-fixture" config user.name "Aruvi Test" && git -C "/repositories/push-$fixture-fixture" config user.email "aruvi-test@example.invalid" && printf "base fixture\n" > "/repositories/push-$fixture-fixture/README.md" && git -C "/repositories/push-$fixture-fixture" add README.md && git -C "/repositories/push-$fixture-fixture" commit -q -m "Initial fixture" && git -C "/repositories/push-$fixture-fixture" remote add origin "/repositories/push-$fixture-origin.git" && git -C "/repositories/push-$fixture-fixture" push -q origin main || exit 1; done; printf "user staged data\n" > /repositories/push-staged-fixture/user-staged.txt && git -C /repositories/push-staged-fixture add user-staged.txt'
docker exec "$name" sh -c 'mv /repositories/push-remote-origin.git /repositories/push-remote-origin.saved'
docker exec -d "$name" sh -c 'while ! test -f /repositories/push-remote-fixture/allow-push; do sleep 0.1; done; mv /repositories/push-remote-origin.saved /repositories/push-remote-origin.git && touch /repositories/push-remote-fixture/origin-ready'

"${fixture_compose[@]}" run --rm -d --name "$browser" -e "ARUVI_REMOTE_URL=http://${name}:8787" -e "ARUVI_TEST_MODEL_URL=http://${browser}:19876/v1" -e "ARUVI_TEST_WHISPER_MODE=$whisper_test_mode" -e "ARUVI_TEST_LOCAL_AUDIO=${ARUVI_TEST_WHISPER_AUDIO:+/tmp/aruvi-test-speech.wav}" browser sh -c 'node /project/e2e-remote/mock-provider.mjs' >/dev/null
if [[ -n "${ARUVI_TEST_WHISPER_AUDIO:-}" ]]; then
  docker exec -i "$browser" sh -c 'umask 077; cat > /tmp/aruvi-test-speech.wav' < "$ARUVI_TEST_WHISPER_AUDIO"
fi
docker exec "$browser" sh -c 'cp /project/package.json /project/package-lock.json /project/playwright.remote.config.js ./ && cp -R /project/e2e-remote ./ && npm ci --silent && npx playwright test -c playwright.remote.config.js workflow.spec.js'

docker exec "$name" sh -c 'test "$(git --git-dir=/repositories/push-origin.git show main:src/generated.txt)" = "generated by workflow test"'
docker exec "$name" sh -c 'test "$(cat /repositories/occupied-workspace/README.md)" = "existing readme" && test "$(cat /repositories/occupied-workspace/.gitignore)" = "existing ignore" && test ! -e /repositories/occupied-workspace/.git && test ! -e /repositories/other-product-foreign-area'
docker exec "$name" sh -c 'test "$(git -C /repositories/fresh-workspace symbolic-ref --short HEAD)" = main && git -C /repositories/fresh-workspace diff --quiet --cached'
docker exec "$name" sh -c 'test "$(git --git-dir=/repositories/mcp-origin.git show main:src/generated.txt)" = "generated by workflow test"'
docker exec "$name" sh -c 'test "$(git --git-dir=/repositories/push-origin.git show main:README.md)" = "base fixture" && test "$(cat /repositories/push-fixture/user-untracked.txt)" = "untracked user data" && git -C /repositories/push-fixture diff --quiet --cached && git -C /repositories/push-fixture diff -- README.md | grep -q "unrelated local edit"'
docker exec "$name" sh -c 'test "$(git -C /repositories/push-staged-fixture diff --cached --name-only)" = "user-staged.txt" && test "$(cat /repositories/push-staged-fixture/user-staged.txt)" = "user staged data" && test "$(git --git-dir=/repositories/push-staged-origin.git rev-list --count main)" = 1 && test "$(git --git-dir=/repositories/push-changed-origin.git rev-list --count main)" = 2 && test "$(git --git-dir=/repositories/push-remote-origin.git rev-list --count main)" = 2 && test "$(git --git-dir=/repositories/push-remote-origin.git show main:src/generated.txt)" = "generated by workflow test" && ! git --git-dir=/repositories/push-remote-origin.git cat-file -e main:allow-push 2>/dev/null'
