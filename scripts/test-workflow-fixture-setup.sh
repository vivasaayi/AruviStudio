#!/usr/bin/env bash
# Sourced with the disposable fixture container name in $name.
docker exec "$name" sh -c 'for fixture in workflow-test workflow-test-passed workflow-test-override workflow-test-failed workflow-test-skipped planner-active-fixture; do mkdir -p "/repositories/$fixture" && git init -q "/repositories/$fixture" && printf "test repo\n" > "/repositories/$fixture/README.md" || exit 1; done'
docker exec "$name" sh -c 'mkdir -p /repositories/occupied-workspace && printf "existing readme\n" > /repositories/occupied-workspace/README.md && printf "existing ignore\n" > /repositories/occupied-workspace/.gitignore && ln -s occupied-workspace /repositories/alias-workspace'
docker exec "$name" sh -c 'git init -q --bare /repositories/mcp-origin.git && git init -q -b main /repositories/mcp-fixture && git -C /repositories/mcp-fixture config user.name "Aruvi Test" && git -C /repositories/mcp-fixture config user.email "aruvi-test@example.invalid" && printf "MCP fixture\n" > /repositories/mcp-fixture/README.md && git -C /repositories/mcp-fixture add README.md && git -C /repositories/mcp-fixture commit -q -m initial && git -C /repositories/mcp-fixture remote add origin /repositories/mcp-origin.git && git -C /repositories/mcp-fixture push -q origin main'
docker exec "$name" sh -c 'umask 077; printf "%s" "{\"refs\":{\"synthetic-external-reference-with-hyphens\":\"synthetic-legacy-key-12345\"}}" > /data/llm-config.json'
docker exec "$name" sh -c 'mkdir -p /data/models/fixture && printf "synthetic model" > /data/models/fixture/existing.bin && printf "alternative model" > /data/models/fixture/alternative.bin'
docker exec "$name" sh -c 'mkdir -p /data/models/whisper-cpp-tiny-en-local'
if [[ "${ARUVI_TEST_WHISPER_DOWNLOAD:-}" == 1 ]]; then
  whisper_test_mode=download
elif [[ -n "${ARUVI_TEST_WHISPER_MODEL:-}" ]]; then
  docker exec -i "$name" sh -c 'cat > /data/models/whisper-cpp-tiny-en-local/ggml-tiny.en.bin' < "$ARUVI_TEST_WHISPER_MODEL"
  docker exec "$name" sh -c 'cp /data/models/whisper-cpp-tiny-en-local/ggml-tiny.en.bin /data/models/fixture/alternative.bin'
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

