#!/usr/bin/env python3
"""Exercise saved bind settings and environment precedence on a disposable server."""
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
parser.add_argument("server", type=Path, help="Built native aruvi-studio-server binary")
binary = parser.parse_args().server.resolve(strict=True)
opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
token = secrets.token_urlsafe(32)
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


with tempfile.TemporaryDirectory(prefix="aruvi-bridge-bind-", dir="/private/tmp") as temporary:
    root = Path(temporary)
    with socket.socket() as first, socket.socket() as second:
        first.bind(("127.0.0.1", 0))
        second.bind(("127.0.0.1", 0))
        env_port, saved_port = first.getsockname()[1], second.getsockname()[1]

    def start(with_environment):
        global process
        env = {key: value for key, value in os.environ.items() if not key.startswith("ARUVI_")}
        env.update(ARUVI_APP_DATA_DIR=str(root / "data"), ARUVI_LLM_CONFIG_PATH=str(root / "llm.json"),
                   ARUVI_MOBILE_API_TOKEN=token, ARUVI_MCP_API_TOKEN=secrets.token_urlsafe(32))
        if with_environment:
            env.update(ARUVI_WEBHOOK_HOST="127.0.0.1", ARUVI_WEBHOOK_PORT=str(env_port))
        port = env_port if with_environment else saved_port
        process = subprocess.Popen([str(binary)], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        deadline = time.monotonic() + 45
        while time.monotonic() < deadline:
            if process.poll() is not None:
                raise RuntimeError("Disposable server exited before becoming ready")
            owners = subprocess.run(["lsof", "-nP", f"-iTCP:{port}", "-sTCP:LISTEN", "-t"], capture_output=True, text=True).stdout.split()
            if str(process.pid) in owners:
                try:
                    with opener.open(f"http://127.0.0.1:{port}/health", timeout=2) as response:
                        if response.status == 200:
                            return f"http://127.0.0.1:{port}"
                except (OSError, urllib.error.URLError):
                    pass
            time.sleep(0.2)
        raise RuntimeError("Disposable server did not become ready on the expected port")

    def call(base, path, method="GET", body=None, authenticated=True):
        headers = {"Content-Type": "application/json"}
        if authenticated:
            headers["Authorization"] = "Bearer " + token
        request = urllib.request.Request(base + path, method=method, headers=headers,
                                         data=None if body is None else json.dumps(body).encode())
        with opener.open(request, timeout=15) as response:
            return json.load(response)

    try:
        base = start(True)
        try:
            call(base, "/api/mobile/bridge-bind", authenticated=False)
            raise AssertionError("Unauthenticated bind read was accepted")
        except urllib.error.HTTPError as error:
            assert error.code == 401
        raw = call(base, "/api/mobile/bridge-bind")["raw"]
        result = call(base, "/api/mobile/bridge-bind", "POST", {"expected": raw, "host": "127.0.0.1", "port": str(saved_port)})
        assert result["raw"] == ["127.0.0.1", str(saved_port)]
        status = call(base, "/api/mobile/bridge-status")
        assert status["configured_bind_port"] == env_port and status["port_source"] == "env"
        stop()
        base = start(True)
        assert call(base, "/api/mobile/bridge-bind")["raw"] == result["raw"]
        status = call(base, "/api/mobile/bridge-status")
        assert status["configured_bind_port"] == env_port and status["port_source"] == "env"
        stop()
        base = start(False)
        assert call(base, "/api/mobile/bridge-bind")["raw"] == result["raw"]
        status = call(base, "/api/mobile/bridge-status")
        assert status["configured_bind_port"] == saved_port and status["port_source"] == "settings"
        print("Saved bridge bind persisted across restarts and yielded to the environment override.")
    finally:
        stop()
