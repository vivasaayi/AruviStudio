#!/usr/bin/env python3
"""Exercise guarded database overrides with two disposable Linux servers."""
import concurrent.futures
import json
import os
from pathlib import Path
import secrets
import subprocess
import sys
import tempfile
import threading
import time
import urllib.error
import urllib.request
import uuid

root = Path(__file__).resolve().parent.parent
plan = json.loads(subprocess.check_output(["bash", "scripts/env", "worktree", "plan"], cwd=root, text=True))
environment = dict(line.split("=", 1) for line in (Path(plan["state"]) / "environment.env").read_text().splitlines()
                   if line and not line.startswith("#") and "=" in line)
image = os.environ.get("ARUVI_TEST_APP_IMAGE") or environment["APP_IMAGE"]
suffix = uuid.uuid4().hex[:16]
volume = f'{plan["project"]}_db_override_test_{suffix}'
names = [f'{plan["project"]}-db-override-{suffix}-{number}' for number in (1, 2)]
owned_volume = False
owned_containers = []
opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
token = secrets.token_urlsafe(32)

def docker(*args):
    return subprocess.check_output(["docker", *args], cwd=root, text=True).strip()

def call(base, path, data=None):
    headers = {"Authorization": "Bearer " + token}
    payload = None
    if data is not None:
        payload = json.dumps(data).encode()
        headers["Content-Type"] = "application/json"
    request = urllib.request.Request(base + path, data=payload, headers=headers)
    try:
        with opener.open(request, timeout=15) as response:
            return response.status, json.load(response)
    except urllib.error.HTTPError as error:
        return error.code, error.read().decode("utf-8", "replace")

try:
    for name in names:
        if subprocess.run(["docker", "inspect", name], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode == 0:
            raise RuntimeError("Disposable container name already exists")
    if subprocess.run(["docker", "volume", "inspect", volume], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode == 0:
        raise RuntimeError("Disposable volume name already exists")
    docker("volume", "create", volume)
    owned_volume = True
    docker("run", "--rm", "--user", "0", "-v", f"{volume}:/data", "--entrypoint", "chown", image, "10001:10001", "/data")
    with tempfile.TemporaryDirectory(prefix="aruvi-db-override-env-") as temporary:
        env_file = Path(temporary) / "environment.env"
        env_file.write_text(f"ARUVI_MOBILE_API_TOKEN={token}\nARUVI_MCP_API_TOKEN={token}\n")
        env_file.chmod(0o600)
        endpoints = []
        for name in names:
            docker("create", "--name", name, "--restart", "no", "--env-file", str(env_file),
                   "-e", "ARUVI_APP_DATA_DIR=/data", "-e", "ARUVI_DB_PATH=/data/studio.db",
                   "-v", f"{volume}:/data", "-p", "127.0.0.1::8787", image)
            owned_containers.append(name)
            docker("start", name)
            address = docker("port", name, "8787/tcp").splitlines()[0]
            if not address.startswith("127.0.0.1:"):
                raise RuntimeError("Fixture did not bind loopback")
            base = "http://" + address
            deadline = time.monotonic() + 90
            while True:
                try:
                    with opener.open(base + "/health", timeout=2) as response:
                        if response.status == 200:
                            break
                except (OSError, urllib.error.URLError):
                    pass
                if time.monotonic() >= deadline:
                    raise RuntimeError("Fixture did not become healthy")
                time.sleep(.25)
            endpoints.append(base)

        for iteration in range(10):
            baseline = f"/tmp/baseline-{iteration}.db"
            status, _ = call(endpoints[0], "/api/mobile/database-settings", {"path": baseline})
            if status != 200:
                raise RuntimeError("Unable to set baseline")
            barrier = threading.Barrier(2)
            def update(index):
                barrier.wait(timeout=10)
                return call(endpoints[index], "/api/mobile/database-settings",
                            {"path": f"/tmp/winner-{iteration}-{index}.db", "expected": {"path": baseline}})
            with concurrent.futures.ThreadPoolExecutor(max_workers=2) as executor:
                futures = [executor.submit(update, index) for index in range(2)]
                results = [future.result(timeout=30) for future in futures]
            if sorted(status for status, _ in results) != [200, 409]:
                raise RuntimeError("Concurrent writes did not produce one success and one conflict")
            values = [call(base, "/api/mobile/database-settings") for base in endpoints]
            if any(status != 200 for status, _ in values):
                raise RuntimeError("Unable to read override after update")
            actual = [body["override_path"] for _, body in values]
            if actual[0] != actual[1] or actual[0] not in {f"/tmp/winner-{iteration}-0.db", f"/tmp/winner-{iteration}-1.db"}:
                raise RuntimeError("Servers did not observe the same complete winning override")
    print("Two Linux Studio processes serialized 10 concurrent database override updates.")
finally:
    cleanup_failed = False
    for name in owned_containers:
        cleanup_failed |= subprocess.run(["docker", "rm", "--force", name], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode != 0
    if owned_volume:
        cleanup_failed |= subprocess.run(["docker", "volume", "rm", volume], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode != 0
    if cleanup_failed:
        print("Failed to remove one or more disposable database override fixture resources", file=sys.stderr)
        if sys.exc_info()[0] is None:
            raise RuntimeError("Database override fixture cleanup failed")
