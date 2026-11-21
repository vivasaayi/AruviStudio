#!/usr/bin/env python3
"""Exercise bridge-token rotation in a disposable Studio container and browser."""
import argparse
import json
import os
from pathlib import Path
import secrets
import subprocess
import tempfile
import time
import urllib.error
import urllib.request
import uuid

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--browser-executable", required=True, type=Path)
args = parser.parse_args()
root = Path(__file__).resolve().parent.parent
plan = json.loads(subprocess.check_output(["bash", "scripts/env", "worktree", "plan"], cwd=root, text=True))
image = os.environ.get("ARUVI_TEST_APP_IMAGE")
if not image:
    with (Path(plan["state"]) / "environment.env").open() as env_file:
        image = next(line.split("=", 1)[1].strip() for line in env_file if line.startswith("APP_IMAGE="))
suffix = uuid.uuid4().hex[:16]
name = f'{plan["project"]}-bridge-token-{suffix}'
volume = f'{plan["project"]}-bridge-token-{suffix}'
bootstrap, mobile, replacement, mcp, saved_mcp = [secrets.token_urlsafe(32) for _ in range(5)]
opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
created_volume = False
running = False


def docker(*args):
    return subprocess.check_output(["docker", *args], text=True).strip()


with tempfile.TemporaryDirectory(prefix="aruvi-container-bridge-token-", dir="/private/tmp") as temporary:
    private = Path(temporary)

    def start(mobile_override):
        global running
        env_file = private / "container.env"
        values = {"ARUVI_WEBHOOK_HOST":"0.0.0.0", "ARUVI_WEBHOOK_PORT":"8787", "ARUVI_MCP_API_TOKEN":mcp}
        if mobile_override:
            values["ARUVI_MOBILE_API_TOKEN"] = bootstrap
        env_file.write_text("".join(f"{key}={value}\n" for key, value in values.items()))
        env_file.chmod(0o600)
        docker("run", "--rm", "-d", "--name", name, "-p", "127.0.0.1::8787", "--env-file", str(env_file), "-v", f"{volume}:/data", image)
        running = True
        address = docker("port", name, "8787/tcp").splitlines()[0]
        if not address.startswith("127.0.0.1:"):
            raise RuntimeError("Fixture must be bound to loopback")
        base = "http://" + address
        deadline = time.monotonic() + 60
        while time.monotonic() < deadline:
            if docker("inspect", "--format", "{{.State.Running}}", name) != "true":
                raise RuntimeError("Disposable container exited before becoming ready")
            try:
                with opener.open(base + "/health", timeout=2) as response:
                    if response.status == 200:
                        return base
            except (OSError, urllib.error.URLError):
                time.sleep(0.2)
        raise RuntimeError("Disposable container did not become healthy")

    def stop():
        global running
        if running:
            docker("stop", "-t", "15", name)
            running = False

    def call(base, token, path, method="GET", body=None):
        request = urllib.request.Request(base + path, method=method,
            headers={"Content-Type": "application/json", "Authorization": "Bearer " + token},
            data=None if body is None else json.dumps(body).encode())
        with opener.open(request, timeout=15) as response:
            return json.load(response)

    try:
        docker("volume", "create", volume)
        created_volume = True
        base = start(True)
        view = call(base, bootstrap, "/api/mobile/bridge-tokens")
        assert view["mobile"]["environment_override"]
        call(base, bootstrap, "/api/mobile/bridge-tokens", "POST", {"kind":"mobile", "expected_revision":view["mobile"]["revision"], "action":"replace", "token":mobile})
        stop()
        base = start(False)
        view = call(base, mobile, "/api/mobile/bridge-tokens")
        assert not view["mobile"]["environment_override"] and view["mcp"]["environment_override"]
        env = {key: value for key, value in os.environ.items() if not key.startswith("ARUVI_")}
        env.update(ARUVI_REMOTE_URL=base, ARUVI_MOBILE_API_TOKEN=mobile, ARUVI_MCP_API_TOKEN=mcp,
                   ARUVI_TEST_NEW_MOBILE_TOKEN=replacement, ARUVI_TEST_NEW_MCP_TOKEN=saved_mcp,
                   ARUVI_TEST_BRIDGE_TOKEN_ROTATION="1", ARUVI_CHROME_PATH=str(args.browser_executable.resolve(strict=True)))
        subprocess.run(["npx", "playwright", "test", "-c", "playwright.remote.config.js", "remote.spec.js", "-g", "browser rotates bridge tokens without exposing saved values"], cwd=root, env=env, check=True)
        try:
            call(base, mobile, "/api/mobile/bridge-tokens")
            raise AssertionError("Old mobile token was accepted after rotation")
        except urllib.error.HTTPError as error:
            assert error.code == 401
        call(base, replacement, "/api/mobile/bridge-tokens")
        stop()
        base = start(False)
        view = call(base, replacement, "/api/mobile/bridge-tokens")
        call(base, replacement, "/api/mobile/bridge-tokens", "POST", {"kind":"mobile", "expected_revision":view["mobile"]["revision"], "action":"clear"})
        try:
            call(base, replacement, "/api/mobile/bridge-tokens")
            raise AssertionError("Mobile access remained enabled after clearing its saved token")
        except urllib.error.HTTPError as error:
            assert error.code == 503
        stop()
        base = start(True)
        recovered = call(base, bootstrap, "/api/mobile/bridge-tokens")
        assert recovered["mobile"]["environment_override"] and not recovered["mobile"]["saved_configured"]
        print("Container token rotation survived restart; clearing and environment recovery passed.")
    finally:
        if running:
            subprocess.run(["docker", "rm", "-f", name], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        if created_volume:
            subprocess.run(["docker", "volume", "rm", volume], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
