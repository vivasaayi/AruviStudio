#!/usr/bin/env python3
"""Upgrade a synthetic pre-product-area database through real Studio servers."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import secrets
import socket
import sqlite3
import subprocess
import tempfile
import time
import urllib.error
import urllib.request

root = Path(__file__).resolve().parent.parent
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("server", type=Path, help="Native Studio server built from the revision under test")
parser.add_argument("--image", help="Also test a local Linux Studio image")
args = parser.parse_args()
server = args.server.resolve(strict=True)
opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
token = secrets.token_urlsafe(32)

def seed(path):
    with sqlite3.connect(path) as db:
        db.execute("PRAGMA foreign_keys=ON")
        migrations = sorted((root / "src-tauri/migrations").glob("*.sql"))
        for migration in migrations:
            if int(migration.name.split("_", 1)[0]) > 42: break
            db.executescript(migration.read_text())
        db.execute("CREATE TABLE _sqlx_migrations (version BIGINT PRIMARY KEY, description TEXT NOT NULL, installed_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP, success BOOLEAN NOT NULL, checksum BLOB NOT NULL, execution_time BIGINT NOT NULL)")
        for migration in migrations:
            version_text, description = migration.stem.split("_", 1)
            version = int(version_text)
            if version > 42: break
            db.execute("INSERT INTO _sqlx_migrations(version,description,success,checksum,execution_time) VALUES(?,?,1,?,0)",
                       (version, description.replace("_", " "), hashlib.sha384(migration.read_bytes()).digest()))
        db.execute("INSERT INTO products(id,name) VALUES('legacy-product','Legacy product')")
        db.execute("INSERT INTO modules(id,product_id,name,description) VALUES('legacy-area','legacy-product','Legacy area','Legacy description')")
        db.execute("INSERT INTO capabilities(id,module_id,name) VALUES('legacy-capability','legacy-area','Legacy capability')")
        db.execute("INSERT INTO work_items(id,product_id,module_id,capability_id,source_node_id,source_node_type,title,work_item_type) VALUES('legacy-work','legacy-product','legacy-area','legacy-capability','legacy-capability','capability','Legacy story','story')")
        db.execute("INSERT INTO model_providers(id,name,provider_type,base_url,auth_secret_ref) VALUES('legacy-provider','Legacy provider','openai_compatible','http://127.0.0.1:1/v1','ref:synthetic-legacy')")
        db.execute("INSERT INTO model_definitions(id,provider_id,name) VALUES('legacy-model','legacy-provider','legacy-coding-model')")
        db.execute("INSERT INTO agent_model_bindings(id,agent_id,model_id,priority) VALUES('legacy-binding','coding-agent','legacy-model',5)")
        assert not db.execute("PRAGMA foreign_key_check").fetchall()

def mcp(base, tool, action, arguments):
    payload = {"jsonrpc":"2.0", "id":1, "method":"tools/call", "params":{"name":tool,"arguments":{"action":action,"arguments":arguments}}}
    request = urllib.request.Request(base + "/api/mcp", data=json.dumps(payload).encode(), headers={"Content-Type":"application/json", "Accept":"application/json", "Authorization":"Bearer " + token})
    with opener.open(request, timeout=15) as response: reply = json.load(response)
    result = reply.get("result", {})
    if "error" in reply or result.get("isError"): raise RuntimeError("MCP action failed: " + action)
    return result["structuredContent"]["result"]

def check(base):
    try:
        opener.open(base + "/api/mobile/products", timeout=10)
        raise RuntimeError("Unauthenticated product request was accepted")
    except urllib.error.HTTPError as error:
        if error.code != 401: raise
    request = urllib.request.Request(base + "/api/mobile/products", headers={"Authorization":"Bearer " + token})
    with opener.open(request, timeout=10) as response: products = json.load(response)
    assert any(row["id"] == "legacy-product" and row["name"] == "Legacy product" for row in products)
    assert mcp(base,"aruvi_catalog","get_product",{"id":"legacy-product"})["name"] == "Legacy product"
    areas = mcp(base,"aruvi_catalog","list_product_areas",{"product_id":"legacy-product"})
    assert any(row["id"] == "legacy-area" and row["description"] == "Legacy description" for row in areas)
    capabilities = mcp(base,"aruvi_catalog","list_capabilities",{"product_area_id":"legacy-area"})
    assert any(row["id"] == "legacy-capability" for row in capabilities)
    item = mcp(base,"aruvi_work_items","get_work_item",{"id":"legacy-work"})
    assert item["product_area_id"] == "legacy-area" and item["capability_id"] == "legacy-capability" and item["work_item_type"] == "story"
    models = mcp(base,"aruvi_models","list_model_definitions",{})
    assert any(row["id"] == "legacy-model" and row["provider_id"] == "legacy-provider" for row in models)
    bindings = mcp(base,"aruvi_agents","list_agent_model_bindings",{})
    assert any(row["id"] == "legacy-binding" and row["model_id"] == "legacy-model" and row["agent_id"] == "coding-agent" for row in bindings)

with tempfile.TemporaryDirectory(prefix="aruvi-legacy-upgrade-", dir="/private/tmp") as temporary:
    workspace = Path(temporary)
    workspace.chmod(0o777)  # Disposable data must be writable by the container's unprivileged UID.
    for mode in (["native", "docker"] if args.image else ["native"]):
        data = workspace / mode
        data.mkdir(mode=0o777)
        data.chmod(0o777)
        db_path = data / "legacy.db"
        seed(db_path)
        db_path.chmod(0o666)
        container_name = "aruvi-legacy-upgrade-" + secrets.token_hex(8)
        for iteration in range(2):
            process = None
            try:
                if mode == "native":
                    with socket.socket() as reserve:
                        reserve.bind(("127.0.0.1", 0)); port = reserve.getsockname()[1]
                    env = {key:value for key,value in os.environ.items() if not key.startswith("ARUVI_")}
                    env.update(ARUVI_APP_DATA_DIR=str(data), ARUVI_DB_PATH=str(db_path), ARUVI_LLM_CONFIG_PATH=str(data / "llm.json"), ARUVI_WEBHOOK_HOST="127.0.0.1", ARUVI_WEBHOOK_PORT=str(port), ARUVI_MOBILE_API_TOKEN=token, ARUVI_MCP_API_TOKEN=token)
                    process = subprocess.Popen([str(server)], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                else:
                    subprocess.run(["docker","run","-d","--rm","--name",container_name,"--mount",f"type=bind,src={data},dst=/data","-p","127.0.0.1::8787","-e","ARUVI_DB_PATH=/data/legacy.db","-e","ARUVI_MOBILE_API_TOKEN="+token,"-e","ARUVI_MCP_API_TOKEN="+token,args.image], check=True, capture_output=True)
                    published = subprocess.check_output(["docker","port",container_name,"8787/tcp"], text=True).strip()
                    if not published.startswith("127.0.0.1:"): raise RuntimeError("Docker fixture is not loopback-only")
                    port = int(published.rsplit(":",1)[1])
                base = f"http://127.0.0.1:{port}"
                deadline = time.monotonic() + 45
                while True:
                    if process and process.poll() is not None: raise RuntimeError("Native fixture exited before health")
                    try:
                        with opener.open(base + "/health", timeout=2) as response:
                            if response.status == 200: break
                    except (OSError, urllib.error.URLError): pass
                    if time.monotonic() >= deadline: raise RuntimeError("Upgrade fixture did not become healthy")
                    time.sleep(0.2)
                check(base)
            finally:
                if process and process.poll() is None:
                    process.terminate(); process.wait(timeout=15)
                if mode == "docker": subprocess.run(["docker","rm","-f",container_name], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        with sqlite3.connect(db_path) as db:
            assert db.execute("PRAGMA quick_check").fetchone()[0] == "ok"
            assert not db.execute("PRAGMA foreign_key_check").fetchall()
            latest = max(int(path.name.split("_", 1)[0]) for path in (root / "src-tauri/migrations").glob("*.sql"))
            assert db.execute("SELECT MAX(version) FROM _sqlx_migrations").fetchone()[0] == latest
            assert db.execute("SELECT auth_secret_ref FROM model_providers WHERE id='legacy-provider'").fetchone()[0] == "ref:synthetic-legacy"
        print(f"{mode}: synthetic migration-42 product, hierarchy, work and model survived startup and restart; integrity passed", flush=True)
