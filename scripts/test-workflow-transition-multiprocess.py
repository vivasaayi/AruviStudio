#!/usr/bin/env python3
"""Race workflow gate decisions across disposable Studio containers."""
import concurrent.futures
import json
import os
from pathlib import Path
import secrets
import subprocess
import tempfile
import threading
import time
import urllib.error
import urllib.request
import uuid

root = Path(__file__).resolve().parent.parent
plan = json.loads(subprocess.check_output(["bash", "scripts/env", "worktree", "plan"], cwd=root, text=True))
image = os.environ.get("ARUVI_TEST_APP_IMAGE")
if not image:
    with (Path(plan["state"]) / "environment.env").open() as env_file:
        image = next(line.split("=", 1)[1].strip() for line in env_file if line.startswith("APP_IMAGE="))
suffix = uuid.uuid4().hex[:16]
volume = f'{plan["project"]}-transition-{suffix}'
containers = []
volume_created = False
token = secrets.token_urlsafe(32)
opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))


def docker(*args):
    return subprocess.check_output(["docker", *args], text=True).strip()


def call(base, tool, action, arguments):
    payload = json.dumps({"jsonrpc":"2.0", "id":1, "method":"tools/call", "params":{
        "name":tool, "arguments":{"action":action, "arguments":arguments}}}).encode()
    request = urllib.request.Request(base + "/api/mcp", data=payload, headers={
        "Authorization":"Bearer " + token, "Content-Type":"application/json", "Accept":"application/json"})
    with opener.open(request, timeout=30) as response:
        message = json.load(response)
    if "error" in message:
        raise RuntimeError("Unexpected JSON-RPC error")
    return message["result"]


def result(base, tool, action, arguments):
    response = call(base, tool, action, arguments)
    if response.get("isError"):
        raise RuntimeError("Fixture setup MCP action failed: " + action)
    return response["structuredContent"]["result"]


try:
    docker("image", "inspect", image)
    docker("image", "inspect", "python:3.12-slim")
    docker("volume", "create", volume)
    volume_created = True
    docker("run", "--rm", "--pull", "never", "--user", "0", "-v", f"{volume}:/data", "--entrypoint", "chown", image, "10001:10001", "/data")
    with tempfile.TemporaryDirectory(prefix="aruvi-transition-env-", dir="/private/tmp") as temporary:
        env_file = Path(temporary) / "environment.env"
        env_file.write_text(f"ARUVI_MOBILE_API_TOKEN={token}\nARUVI_MCP_API_TOKEN={token}\n")
        env_file.chmod(0o600)
        endpoints = []
        for index in range(2):
            name = f'{plan["project"]}-transition-{suffix}-{index}'
            docker("create", "--name", name, "--restart", "no", "--env-file", str(env_file),
                   "-e", "ARUVI_APP_DATA_DIR=/data", "-e", "ARUVI_DB_PATH=/data/studio.db",
                   "-v", f"{volume}:/data", "-p", "127.0.0.1::8787", image)
            containers.append(name)
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
                time.sleep(.2)
            endpoints.append(base)

        product = result(endpoints[0], "aruvi_catalog", "create_product", {"name":"Transition race fixture"})
        item = result(endpoints[0], "aruvi_work_items", "create_work_item", {"product_id":product["id"], "title":"Transition fixture", "work_item_type":"task"})
        seed = "import sqlite3,sys; c=sqlite3.connect('/data/studio.db'); c.execute(\"INSERT INTO workflow_runs(id,work_item_id,workflow_version,status,current_stage) VALUES (?,?,'v1','running','pending_task_approval')\",sys.argv[1:3]); c.commit()"
        lock = "import pathlib,sqlite3,sys,time; p=pathlib.Path('/data'); c=sqlite3.connect('/data/studio.db',timeout=10); c.execute('BEGIN IMMEDIATE'); (p/(sys.argv[1]+'.ready')).touch(); deadline=time.monotonic()+20;\nwhile not (p/(sys.argv[1]+'.release')).exists():\n if time.monotonic()>deadline: raise RuntimeError('lock timeout')\n time.sleep(.01)\nc.rollback()"
        for iteration in range(5):
            run_id = "race-" + str(iteration)
            docker("run", "--rm", "--pull", "never", "--user", "10001:10001", "-v", f"{volume}:/data", "--entrypoint", "python3", "python:3.12-slim", "-c", seed, run_id, item["id"])
            helper = f'{plan["project"]}-transition-lock-{suffix}-{iteration}'
            marker = "transition-lock-" + str(iteration)
            docker("run", "-d", "--name", helper, "--pull", "never", "--user", "10001:10001", "-v", f"{volume}:/data", "--entrypoint", "python3", "python:3.12-slim", "-c", lock, marker)
            containers.append(helper)
            deadline = time.monotonic() + 15
            while subprocess.run(["docker", "exec", helper, "test", "-f", f"/data/{marker}.ready"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode:
                if time.monotonic() >= deadline:
                    raise RuntimeError("Fixture did not acquire the database write lock")
                time.sleep(.05)
            barrier = threading.Barrier(3)
            def reject(base):
                barrier.wait(timeout=10)
                return call(base, "aruvi_workflows", "handle_workflow_user_action", {"workflow_run_id":run_id, "action":"reject"})
            with concurrent.futures.ThreadPoolExecutor(max_workers=2) as executor:
                futures = [executor.submit(reject, base) for base in endpoints]
                barrier.wait(timeout=10)
                time.sleep(.3)
                if any(future.done() for future in futures):
                    raise RuntimeError("A decision completed while the database write lock was held")
                docker("exec", helper, "touch", f"/data/{marker}.release")
                responses = [future.result(timeout=20) for future in futures]
            if docker("wait", helper) != "0":
                raise RuntimeError("Fixture lock helper failed")
            docker("rm", helper)
            containers.remove(helper)
            successes = sum(not response.get("isError", False) for response in responses)
            if successes != 1:
                raise RuntimeError(f"Expected one successful decision and one rejected stale decision; observed {successes} successes")
            run = result(endpoints[0], "aruvi_workflows", "get_workflow_run", {"workflow_run_id":run_id})
            history = result(endpoints[1], "aruvi_workflows", "get_workflow_history", {"workflow_run_id":run_id})
            if run["status"] != "cancelled" or run["current_stage"] != "cancelled" or len(history) != 1 or history[0]["to_stage"] != "cancelled":
                raise RuntimeError("Workflow state and transition history were inconsistent")
        print("Two Studio containers committed exactly one gate decision and history entry in five races.")
finally:
    for name in reversed(containers):
        subprocess.run(["docker", "rm", "-f", name], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    if volume_created:
        subprocess.run(["docker", "volume", "rm", volume], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
