#!/usr/bin/env python3
"""Check browser and MCP artifact confinement on a disposable Studio container."""
import json
import os
from pathlib import Path
import secrets
import subprocess
import time
import urllib.error
import urllib.request
import uuid

root = Path(__file__).resolve().parent.parent
plan = json.loads(subprocess.check_output(["bash", "scripts/env", "worktree", "plan"], cwd=root, text=True))
image = os.environ.get("ARUVI_TEST_APP_IMAGE")
if not image:
    lines = (Path(plan["state"]) / "environment.env").read_text().splitlines()
    image = next(line.split("=", 1)[1] for line in lines if line.startswith("APP_IMAGE="))
suffix = uuid.uuid4().hex[:16]
volume = f'{plan["project"]}-artifact-{suffix}'
container = f'{plan["project"]}-artifact-{suffix}'
token = secrets.token_urlsafe(32)
created = False
opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))

def docker(*args):
    return subprocess.check_output(["docker", *args], text=True).strip()

def request(base, path, data=None, authenticated=True):
    headers = {"Content-Type":"application/json", "Accept":"application/json"}
    if authenticated: headers["Authorization"] = "Bearer " + token
    body = None if data is None else json.dumps(data).encode()
    try:
        with opener.open(urllib.request.Request(base + path, data=body, headers=headers), timeout=20) as response:
            return response.status, json.load(response)
    except urllib.error.HTTPError as error:
        return error.code, None

def call(base, tool, action, arguments):
    payload = {"jsonrpc":"2.0", "id":1, "method":"tools/call", "params":{
        "name":tool, "arguments":{"action":action, "arguments":arguments}}}
    status, response = request(base, "/api/mcp", payload)
    if status != 200 or "result" not in response: raise RuntimeError("Unexpected MCP response")
    return response["result"]

def result(base, tool, action, arguments):
    response = call(base, tool, action, arguments)
    if response.get("isError"): raise RuntimeError("Fixture MCP setup failed: " + action)
    return response["structuredContent"]["result"]

try:
    docker("image", "inspect", image)
    docker("image", "inspect", "python:3.12-slim")
    docker("volume", "create", volume)
    created = True
    docker("run", "--rm", "--pull", "never", "--user", "0", "-v", f"{volume}:/data", "--entrypoint", "chown", image, "10001:10001", "/data")
    docker("run", "-d", "--name", container, "--pull", "never", "--restart", "no", "-e", "ARUVI_APP_DATA_DIR=/data", "-e", "ARUVI_DB_PATH=/data/studio.db", "-e", "ARUVI_MOBILE_API_TOKEN=" + token, "-e", "ARUVI_MCP_API_TOKEN=" + token, "-v", f"{volume}:/data", "-p", "127.0.0.1::8787", image)
    address = docker("port", container, "8787/tcp").splitlines()[0]
    if not address.startswith("127.0.0.1:"): raise RuntimeError("Fixture did not bind loopback")
    base = "http://" + address
    deadline = time.monotonic() + 90
    while True:
        try:
            with opener.open(base + "/health", timeout=2) as response:
                if response.status == 200: break
        except OSError: pass
        if time.monotonic() >= deadline: raise RuntimeError("Fixture did not become healthy")
        time.sleep(.2)
    product = result(base, "aruvi_catalog", "create_product", {"name":"Artifact fixture"})
    item = result(base, "aruvi_work_items", "create_work_item", {"product_id":product["id"], "title":"Artifact fixture", "work_item_type":"task"})
    seed = """import pathlib,sqlite3,sys
p=pathlib.Path('/data'); a=p/'artifacts'; a.mkdir(exist_ok=True)
(a/'safe.txt').write_text('fixture content'); (p/'private.txt').write_text('private fixture')
(a/'escape').symlink_to(p/'private.txt'); (a/'large.txt').write_bytes(b'x'*(2*1024*1024+1))
c=sqlite3.connect(p/'studio.db')
for name,path in [('safe',a/'safe.txt'),('outside',p/'private.txt'),('escape',a/'escape'),('large',a/'large.txt')]:
 c.execute('INSERT INTO artifacts(id,work_item_id,artifact_type,storage_path,summary) VALUES(?,?,?,?,?)',(name,sys.argv[1],'coding_output',str(path),name))
c.commit()
"""
    docker("run", "--rm", "--pull", "never", "--user", "10001:10001", "-v", f"{volume}:/data", "--entrypoint", "python3", "python:3.12-slim", "-c", seed, item["id"])
    uri = f'/api/mobile/work-items/{item["id"]}/artifacts/'
    if request(base, uri + "safe", authenticated=False)[0] != 401: raise RuntimeError("Browser artifact read accepted no token")
    status, body = request(base, uri + "safe")
    if status != 200 or body["content"] != "fixture content": raise RuntimeError("Valid browser artifact failed")
    response = call(base, "aruvi_checkpoints", "read_artifact_content", {"artifact_id":"safe"})
    if response.get("isError") or response["structuredContent"]["result"]["content"] != "fixture content": raise RuntimeError("Valid MCP artifact failed")
    for artifact_id in ("outside", "escape", "large"):
        status, _ = request(base, uri + artifact_id)
        response = call(base, "aruvi_checkpoints", "read_artifact_content", {"artifact_id":artifact_id})
        if status not in (400, 404) or not response.get("isError"): raise RuntimeError("Unsafe artifact was accepted: " + artifact_id)
    print("Disposable Linux browser and MCP artifact reads enforced confinement and size limits.")
finally:
    subprocess.run(["docker", "rm", "-f", container], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    if created: subprocess.run(["docker", "volume", "rm", volume], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
