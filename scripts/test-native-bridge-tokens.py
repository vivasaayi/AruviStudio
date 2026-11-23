#!/usr/bin/env python3
"""Verify bridge-token rotation against a disposable native server."""
import argparse
import json
import os
from pathlib import Path
import secrets
import socket
import subprocess
import tempfile
import time
import urllib.error
import urllib.request

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("server", type=Path)
parser.add_argument("--browser-executable", type=Path)
args = parser.parse_args()
binary = args.server.resolve(strict=True)
root = Path(__file__).resolve().parent.parent
opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
bootstrap, mobile, replacement, mcp, saved_mcp = [secrets.token_urlsafe(32) for _ in range(5)]
process = None


def stop():
    global process
    if process is not None and process.poll() is None:
        process.terminate()
        try:
            process.wait(timeout=15)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()
            raise RuntimeError("Disposable server did not shut down cleanly")
        if process.returncode != 0:
            raise RuntimeError("Disposable server exited unsuccessfully")
    process = None


with tempfile.TemporaryDirectory(prefix="aruvi-bridge-token-", dir="/private/tmp") as temporary:
    data = Path(temporary)
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        port = sock.getsockname()[1]
    base = f"http://127.0.0.1:{port}"

    def start(mobile_override):
        global process
        env = {key: value for key, value in os.environ.items() if not key.startswith("ARUVI_")}
        env.update(ARUVI_APP_DATA_DIR=str(data / "app"), ARUVI_LLM_CONFIG_PATH=str(data / "llm.json"),
                   ARUVI_WEBHOOK_HOST="127.0.0.1", ARUVI_WEBHOOK_PORT=str(port), ARUVI_MCP_API_TOKEN=mcp)
        if mobile_override:
            env["ARUVI_MOBILE_API_TOKEN"] = bootstrap
        process = subprocess.Popen([str(binary)], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        deadline = time.monotonic() + 45
        while time.monotonic() < deadline:
            if process.poll() is not None:
                raise RuntimeError("Disposable server exited before becoming ready")
            owners = subprocess.run(["lsof", "-nP", f"-iTCP:{port}", "-sTCP:LISTEN", "-t"], capture_output=True, text=True).stdout.split()
            if str(process.pid) in owners:
                try:
                    with opener.open(base + "/health", timeout=2) as response:
                        if response.status == 200:
                            return
                except (OSError, urllib.error.URLError):
                    pass
            time.sleep(0.2)
        raise RuntimeError("Disposable server did not become ready")

    def call(token, path, method="GET", body=None):
        request = urllib.request.Request(base + path, method=method,
            headers={"Content-Type": "application/json", "Authorization": "Bearer " + token},
            data=None if body is None else json.dumps(body).encode())
        with opener.open(request, timeout=15) as response:
            return json.load(response)

    try:
        start(True)
        view = call(bootstrap, "/api/mobile/bridge-tokens")
        assert view["mobile"]["environment_override"]
        call(bootstrap, "/api/mobile/bridge-tokens", "POST", {"kind":"mobile", "expected_revision":view["mobile"]["revision"], "action":"replace", "token":mobile})
        stop()
        start(False)
        view = call(mobile, "/api/mobile/bridge-tokens")
        assert not view["mobile"]["environment_override"] and view["mcp"]["environment_override"]
        if args.browser_executable:
            env = {key: value for key, value in os.environ.items() if not key.startswith("ARUVI_")}
            env.update(ARUVI_REMOTE_URL=base, ARUVI_MOBILE_API_TOKEN=mobile, ARUVI_MCP_API_TOKEN=mcp,
                       ARUVI_TEST_NEW_MOBILE_TOKEN=replacement, ARUVI_TEST_NEW_MCP_TOKEN=saved_mcp,
                       ARUVI_TEST_BRIDGE_TOKEN_ROTATION="1", ARUVI_CHROME_PATH=str(args.browser_executable.resolve(strict=True)))
            subprocess.run(["npx", "playwright", "test", "-c", "playwright.remote.config.js", "remote.spec.js", "-g", "browser rotates bridge tokens without exposing saved values"], cwd=root, env=env, check=True)
        else:
            call(mobile, "/api/mobile/bridge-tokens", "POST", {"kind":"mobile", "expected_revision":view["mobile"]["revision"], "action":"replace", "token":replacement})
        try:
            call(mobile, "/api/mobile/bridge-tokens")
            raise AssertionError("Old mobile token was accepted after rotation")
        except urllib.error.HTTPError as error:
            assert error.code == 401
        call(replacement, "/api/mobile/bridge-tokens")
        stop()
        start(False)
        view = call(replacement, "/api/mobile/bridge-tokens")
        call(replacement, "/api/mobile/bridge-tokens", "POST", {"kind":"mobile", "expected_revision":view["mobile"]["revision"], "action":"clear"})
        try:
            call(replacement, "/api/mobile/bridge-tokens")
            raise AssertionError("Mobile access remained enabled after clearing its saved token")
        except urllib.error.HTTPError as error:
            assert error.code == 503
        stop()
        start(True)
        recovered = call(bootstrap, "/api/mobile/bridge-tokens")
        assert recovered["mobile"]["environment_override"] and not recovered["mobile"]["saved_configured"]
        print("Mobile token rotation persisted across restart; clearing disabled access and environment recovery succeeded.")
    finally:
        stop()
