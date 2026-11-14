#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$root/scripts/test-fixture-env.sh"
name="${project}-remote-fixture-$$"
browser="${project}-remote-browser-$$"
cleanup() { docker rm -f "$browser" "$name" >/dev/null 2>&1 || true; docker volume rm "$TEST_SOCKET_NAME" >/dev/null 2>&1 || true; }
trap cleanup EXIT
"${fixture_compose[@]}" run --rm -d --name "$name" fixture >/dev/null
docker exec "$name" sh -c '
  d=/repositories/remote-fixture
  mkdir -p "$d" && git init -q "$d" && printf "base line\n" > "$d/git-diff.txt" && git -C "$d" add git-diff.txt &&
  git -C "$d" -c user.name=Fixture -c user.email=fixture@example.invalid commit -q -m fixture &&
  printf "staged line\n" > "$d/git-diff.txt" && git -C "$d" add git-diff.txt && printf "working line\n" > "$d/git-diff.txt" &&
  mkdir "$d/second" "$d/node_modules" && git init -q "$d/second" &&
  printf "workspace inspection fixture\n" > "$d/inspection.txt" && printf "windows line\r\n" > "$d/windows.txt" &&
  printf "one\r\ntwo\n" > "$d/mixed.txt" && printf "hidden fixture\n" > "$d/node_modules/excluded.txt" &&
  ln -s /etc/passwd "$d/outside-link" && truncate -s 2097153 "$d/oversized.txt" && yes é | head -c 3000 > "$d/aaa_unicode.txt"
'
"${fixture_compose[@]}" run --rm -d --name "$browser" \
  -e "ARUVI_REMOTE_URL=http://${name}:8787" -e "ARUVI_TEST_MODEL_URL=http://${browser}:19876/v1" \
  -e "ARUVI_TEST_BOOK_EPUB_CAPTURE=${ARUVI_REMOTE_EPUB_OUTPUT:+/tmp/aruvi-captured-book.epub}" \
  -e ARUVI_TEST_REPOSITORY_PATH=/repositories/remote-fixture -e ARUVI_TEST_SECOND_REPOSITORY_PATH=/repositories/remote-fixture/second \
  -e ARUVI_TEST_INSPECT=1 browser sh -c 'node /project/e2e-remote/mock-provider.mjs' >/dev/null
docker exec "$browser" sh -c 'cp /project/package.json /project/package-lock.json /project/playwright.remote.config.js ./ && cp -R /project/e2e-remote ./ && npm ci --silent && npx playwright test -c playwright.remote.config.js remote.spec.js'
if [[ -n "${ARUVI_REMOTE_EPUB_OUTPUT:-}" ]]; then
  docker cp "$browser:/tmp/aruvi-captured-book.epub" "$ARUVI_REMOTE_EPUB_OUTPUT"
fi
