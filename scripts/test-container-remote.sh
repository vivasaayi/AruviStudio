#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$root/scripts/test-fixture-env.sh"
name="${project}-remote-fixture-$$"
browser="${project}-remote-browser-$$"
pdf="${project}-remote-pdf-$$"
cleanup() { docker rm -f "$browser" "$name" "$pdf" >/dev/null 2>&1 || true; docker volume rm "$TEST_SOCKET_NAME" "$TEST_PDF_SOCKET_NAME" >/dev/null 2>&1 || true; }
trap cleanup EXIT
"${fixture_compose[@]}" run --rm -d --name "$pdf" pdf >/dev/null
docker inspect "$pdf" | python3 -c 'import json,sys; d=json.load(sys.stdin)[0]; h=d["HostConfig"]; mounts=d["Mounts"]; assert d["Config"]["User"] == "aruvi" and h["NetworkMode"] == "none" and h["ReadonlyRootfs"] and "ALL" in h["CapDrop"]; assert any(x.startswith("no-new-privileges") for x in h["SecurityOpt"]) and h["Memory"] > 0 and h["PidsLimit"] > 0; assert [m["Destination"] for m in mounts] == ["/pdf-socket"]; env=d["Config"]["Env"]; assert not any(x.startswith(("ARUVI_MOBILE_API_TOKEN=", "ARUVI_MCP_API_TOKEN=")) for x in env); print("PDF renderer isolation verified")'
"${fixture_compose[@]}" run --rm -d --name "$name" fixture >/dev/null
source "$root/scripts/test-remote-fixture-setup.sh"

"${fixture_compose[@]}" run --rm -d --name "$browser" \
  -e "ARUVI_REMOTE_URL=http://${name}:8787" -e "ARUVI_TEST_MODEL_URL=http://${browser}:19876/v1" \
  -e "ARUVI_TEST_BOOK_EPUB_CAPTURE=${ARUVI_REMOTE_EPUB_OUTPUT:+/tmp/aruvi-captured-book.epub}" \
  -e "ARUVI_TEST_BOOK_PDF_CAPTURE_DIR=${ARUVI_REMOTE_PDF_OUTPUT_DIR:+/tmp}" \
  -e "ARUVI_TEST_BOOK_IMAGE_CAPTURE_DIR=${ARUVI_REMOTE_IMAGE_OUTPUT_DIR:+/tmp}" \
  -e ARUVI_TEST_REPOSITORY_PATH=/repositories/remote-fixture -e ARUVI_TEST_SECOND_REPOSITORY_PATH=/repositories/remote-fixture/second \
  -e ARUVI_TEST_INSPECT=1 browser sh -c 'node /project/e2e-remote/mock-provider.mjs' >/dev/null
docker exec "$browser" sh -c 'cp /project/package.json /project/package-lock.json /project/playwright.remote.config.js ./ && cp -R /project/e2e-remote ./ && npm ci --silent && npx playwright test -c playwright.remote.config.js remote.spec.js'
if [[ -n "${ARUVI_REMOTE_EPUB_OUTPUT:-}" ]]; then
  docker cp "$browser:/tmp/aruvi-captured-book.epub" "$ARUVI_REMOTE_EPUB_OUTPUT"
fi
if [[ -n "${ARUVI_REMOTE_PDF_OUTPUT_DIR:-}" ]]; then
  mkdir -p "$ARUVI_REMOTE_PDF_OUTPUT_DIR"
  for preset in trade-6x9 a5 us-letter; do
    docker cp "$browser:/tmp/$preset.pdf" "$ARUVI_REMOTE_PDF_OUTPUT_DIR/$preset.pdf"
  done
fi
if [[ -n "${ARUVI_REMOTE_IMAGE_OUTPUT_DIR:-}" ]]; then
  mkdir -p "$ARUVI_REMOTE_IMAGE_OUTPUT_DIR"
  docker cp "$browser:/tmp/inline-image.epub" "$ARUVI_REMOTE_IMAGE_OUTPUT_DIR/inline-image.epub"
  docker cp "$browser:/tmp/inline-image.pdf" "$ARUVI_REMOTE_IMAGE_OUTPUT_DIR/inline-image.pdf"
fi
docker stop "$pdf" >/dev/null
source "$root/scripts/test-remote-fixture-outage.sh"
