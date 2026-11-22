#!/usr/bin/env bash
# Run disposable Docker application fixtures with the host's Playwright/Chrome.
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"
mode=${1:?Usage: test-host-browser-fixture.sh remote\|workflow\|ci}
[[ $mode == remote || $mode == workflow || $mode == ci ]] || { printf '%s\n' 'Choose remote, workflow or ci.' >&2; exit 2; }
shift
[[ -x "$root/node_modules/.bin/playwright" ]] || { printf '%s\n' 'Install the project Node dependencies on the host first.' >&2; exit 1; }
source "$root/scripts/test-fixture-env.sh"
name="${project}-host-${mode}-app-$$"
mock="${project}-host-${mode}-mock-$$"
pdf="${project}-host-${mode}-pdf-$$"
ci_mock="${project}-host-${mode}-ci-$$"
for candidate in "$name" "$mock" "$pdf" "$ci_mock"; do
  if docker inspect "$candidate" >/dev/null 2>&1; then printf '%s\n' 'Fixture container name is already in use.' >&2; exit 1; fi
done
for volume in "$TEST_SOCKET_NAME" "$TEST_PDF_SOCKET_NAME"; do
  if docker volume inspect "$volume" >/dev/null 2>&1; then printf '%s\n' 'Fixture volume name is already in use.' >&2; exit 1; fi
done
cleanup() {
  docker rm -f "$name" "$mock" "$pdf" "$ci_mock" >/dev/null 2>&1 || true
  docker volume rm "$TEST_SOCKET_NAME" "$TEST_PDF_SOCKET_NAME" >/dev/null 2>&1 || true
}
trap cleanup EXIT
# The fixture name and volumes were confirmed unused above. Bind only loopback;
# the mock port is fixed because some workflow tests explicitly use 19876.
docker volume create "$TEST_SOCKET_NAME" >/dev/null
"${fixture_compose[@]}" run --rm -d --name "$pdf" pdf >/dev/null
docker inspect "$pdf" | python3 -c 'import json,sys; d=json.load(sys.stdin)[0]; h=d["HostConfig"]; mounts=d["Mounts"]; assert d["Config"]["User"] == "aruvi" and h["NetworkMode"] == "none" and h["ReadonlyRootfs"] and "ALL" in h["CapDrop"]; assert any(x.startswith("no-new-privileges") for x in h["SecurityOpt"]) and h["Memory"] > 0 and h["PidsLimit"] > 0; assert [m["Destination"] for m in mounts] == ["/pdf-socket"]; env=d["Config"]["Env"]; assert not any(x.startswith(("ARUVI_MOBILE_API_TOKEN=", "ARUVI_MCP_API_TOKEN=")) for x in env); print("PDF renderer isolation verified")'
mock_ports=(-p 127.0.0.1::8787)
if [[ $mode != ci ]]; then mock_ports+=(-p 127.0.0.1:19876:19876); fi
docker run --rm -d --name "$mock" "${mock_ports[@]}" \
  -v "$root/e2e-remote/mock-provider.mjs:/tmp/provider.mjs:ro" \
  --entrypoint /usr/local/bin/node "$APP_IMAGE" /tmp/provider.mjs >/dev/null
extra=(-e TZ=UTC)
if [[ $mode == workflow ]]; then
  extra=(-e TZ=UTC -e HTTPS_PROXY=http://127.0.0.1:1 -e https_proxy=http://127.0.0.1:1 -e NO_PROXY= -e no_proxy=)
fi
docker run --rm -d --name "$name" --network "container:$mock" \
  --tmpfs /data:mode=1777 --tmpfs /repositories:mode=1777 \
  -v "$TEST_SOCKET_NAME:/ci" -v "$TEST_PDF_SOCKET_NAME:/pdf-socket" \
  -e ARUVI_MOBILE_API_TOKEN=workflow-mobile-test -e ARUVI_MCP_API_TOKEN=workflow-mcp-test \
  -e ARUVI_MCP_ALLOWED_ORIGINS=https://studio.example -e ARUVI_LLM_CONFIG_PATH=/data/llm-config.json \
  -e ARUVI_PDF_SOCKET=/pdf-socket/renderer.sock -e ARUVI_CI_SOCKET=/ci/ci.sock \
  -e DBUS_SESSION_BUS_ADDRESS=unix:path=/nonexistent "${extra[@]}" "$APP_IMAGE" >/dev/null
address=$(docker port "$mock" 8787/tcp)
[[ $address == 127.0.0.1:* ]] || { printf '%s\n' 'Fixture did not publish a loopback endpoint.' >&2; exit 1; }
export ARUVI_REMOTE_URL="http://$address" ARUVI_MOBILE_API_TOKEN=workflow-mobile-test ARUVI_MCP_API_TOKEN=workflow-mcp-test
export ARUVI_TEST_MODEL_URL=http://127.0.0.1:19876/v1
unset ARUVI_TEST_BOOK_EPUB_CAPTURE ARUVI_TEST_BOOK_IMAGE_CAPTURE_DIR ARUVI_TEST_BOOK_PDF_CAPTURE_DIR ARUVI_TEST_LOCAL_AUDIO ARUVI_TEST_INSPECT ARUVI_ISOLATED_WORKFLOW
python3 - "$ARUVI_REMOTE_URL" <<'PY'
import sys, time, urllib.error, urllib.request
for _ in range(120):
    try:
        with urllib.request.urlopen(sys.argv[1] + '/health', timeout=1) as response:
            if response.status == 200: break
    except (OSError, urllib.error.URLError): pass
    time.sleep(.25)
else: raise SystemExit('Fixture did not become healthy')
PY
if [[ $mode == remote ]]; then
  source "$root/scripts/test-remote-fixture-setup.sh"
  export ARUVI_TEST_INSPECT=1 ARUVI_TEST_REPOSITORY_PATH=/repositories/remote-fixture ARUVI_TEST_SECOND_REPOSITORY_PATH=/repositories/remote-fixture/second
  "$root/node_modules/.bin/playwright" test -c playwright.remote.config.js remote.spec.js "$@"
  docker stop "$pdf" >/dev/null
  source "$root/scripts/test-remote-fixture-outage.sh"
elif [[ $mode == workflow ]]; then
  # Synthetic corrupt preset only; this host fixture does not accept model/audio copies.
  unset ARUVI_TEST_WHISPER_DOWNLOAD ARUVI_TEST_WHISPER_MODEL ARUVI_TEST_WHISPER_AUDIO
  source "$root/scripts/test-workflow-fixture-setup.sh"
  export ARUVI_TEST_WHISPER_MODE="$whisper_test_mode" ARUVI_ISOLATED_WORKFLOW=1
  export ARUVI_TEST_REPOSITORY_PATH=/repositories/workflow-test ARUVI_TEST_PUSH_REPOSITORY_PATH=/repositories/push-fixture
  "$root/node_modules/.bin/playwright" test -c playwright.remote.config.js workflow.spec.js "$@"
  source "$root/scripts/test-workflow-fixture-checks.sh"
else
  source "$root/scripts/test-ci-fixture-setup.sh"
  docker run --rm -d --name "$ci_mock" --network none --user 0 \
    -v "$TEST_SOCKET_NAME:/ci" -v "$root/e2e-remote/mock-ci.mjs:/tmp/mock-ci.mjs:ro" \
    --entrypoint /usr/local/bin/node "$APP_IMAGE" /tmp/mock-ci.mjs >/dev/null
  "$root/node_modules/.bin/playwright" test -c playwright.remote.config.js ci.spec.js "$@"
fi
