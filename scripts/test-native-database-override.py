#!/usr/bin/env python3
"""Check database override and environment precedence in disposable native servers."""
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
args = parser.parse_args()
binary = args.server.resolve(strict=True)
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

with tempfile.TemporaryDirectory(prefix="aruvi-database-override-", dir="/private/tmp") as temporary:
    root = Path(temporary)
    data_dir = root / "app-data"
    default_db = data_dir / "aruvi_studio.db"
    override_db = root / "override" / "studio.db"
    forced_db = root / "forced" / "studio.db"

    def start(forced=None):
        global process
        with socket.socket() as reserve:
            reserve.bind(("127.0.0.1", 0))
            port = reserve.getsockname()[1]
        env = {key: value for key, value in os.environ.items() if not key.startswith("ARUVI_")}
        env.update(ARUVI_APP_DATA_DIR=str(data_dir), ARUVI_LLM_CONFIG_PATH=str(root / "llm.json"),
                   ARUVI_WEBHOOK_HOST="127.0.0.1", ARUVI_WEBHOOK_PORT=str(port),
                   ARUVI_MOBILE_API_TOKEN=token, ARUVI_MCP_API_TOKEN=token)
        if forced is not None:
            env["ARUVI_DB_PATH"] = str(forced)
        process = subprocess.Popen([str(binary)], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        base = f"http://127.0.0.1:{port}"
        deadline = time.monotonic() + 45
        while time.monotonic() < deadline:
            if process.poll() is not None:
                raise RuntimeError("Disposable server exited before becoming ready")
            owners = subprocess.run(["lsof", "-nP", f"-iTCP:{port}", "-sTCP:LISTEN", "-t"], capture_output=True, text=True).stdout.split()
            if str(process.pid) in owners:
                try:
                    with opener.open(base + "/health", timeout=2) as response:
                        if response.status == 200:
                            return base
                except (OSError, urllib.error.URLError):
                    pass
            time.sleep(0.2)
        raise RuntimeError("Disposable server did not become ready")

    def call(base, path, method="GET", body=None, authenticated=True):
        headers = {"Content-Type": "application/json"}
        if authenticated:
            headers["Authorization"] = "Bearer " + token
        request = urllib.request.Request(base + path, method=method, headers=headers,
                                         data=None if body is None else json.dumps(body).encode())
        with opener.open(request, timeout=15) as response:
            return json.load(response)

    def settings(base, active, override, environment):
        state = call(base, "/api/mobile/database-settings")
        assert Path(state["active_path"]).resolve() == active.resolve()
        assert state["override_path"] == (str(override) if override else None)
        assert state["environment_path_set"] is environment
        assert state["health"]["applied_migrations"] > 0

    def names(base):
        return {item["name"] for item in call(base, "/api/mobile/products")}

    try:
        base = start()
        try:
            call(base, "/api/mobile/database-settings", authenticated=False)
            raise RuntimeError("Unauthenticated database settings request was accepted")
        except urllib.error.HTTPError as error:
            if error.code != 401: raise
        settings(base, default_db, None, False)
        call(base, "/api/mobile/products", "POST", {"name": "Default database fixture"})
        call(base, "/api/mobile/database-settings", "POST", {"path": str(override_db)})
        settings(base, default_db, override_db, False)
        stop()

        base = start()
        settings(base, override_db, override_db, False)
        assert "Default database fixture" not in names(base)
        call(base, "/api/mobile/products", "POST", {"name": "Override database fixture"})
        stop()

        base = start()
        settings(base, override_db, override_db, False)
        assert "Override database fixture" in names(base)
        call(base, "/api/mobile/database-settings", "POST", {"path": None})
        stop()

        base = start()
        settings(base, default_db, None, False)
        assert "Default database fixture" in names(base) and "Override database fixture" not in names(base)
        call(base, "/api/mobile/database-settings", "POST", {"path": str(override_db)})
        stop()

        base = start(forced_db)
        settings(base, forced_db, override_db, True)
        assert "Default database fixture" not in names(base) and "Override database fixture" not in names(base)
        stop()

        base = start()
        settings(base, override_db, override_db, False)
        assert "Override database fixture" in names(base)
        stop()
        print("Database override, restart persistence, clear, environment precedence and authorization passed.")
    finally:
        stop()
