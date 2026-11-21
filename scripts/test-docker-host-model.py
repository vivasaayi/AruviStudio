#!/usr/bin/env python3
"""Verify Studio in Docker Desktop can reach an OpenAI-style Mac host fixture."""
import http.server
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
values = dict(line.split("=", 1) for line in (Path(plan["state"]) / "environment.env").read_text().splitlines()
              if line and not line.startswith("#") and "=" in line)
image = os.environ.get("ARUVI_TEST_APP_IMAGE") or values["APP_IMAGE"]
name = f'{plan["project"]}-host-model-{uuid.uuid4().hex[:16]}'
token = secrets.token_urlsafe(32)
owned = False
opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))

class ModelFixture(http.server.BaseHTTPRequestHandler):
    def log_message(self, *_args):
        pass

    def respond(self, status, body):
        body = json.dumps(body).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        if self.path == "/v1/models":
            self.respond(200, {"data": [{"id": "synthetic-host-model"}]})
        else:
            self.respond(404, {"error": "not found"})

    def do_POST(self):
        try:
            size = int(self.headers.get("Content-Length", "0"))
            if self.path != "/v1/chat/completions" or not 0 < size <= 65536:
                return self.respond(400, {"error": "invalid request"})
            body = json.loads(self.rfile.read(size))
            if body.get("model") != "synthetic-host-model" or not body.get("messages"):
                return self.respond(400, {"error": "invalid fixture model"})
            self.respond(200, {"choices": [{"message": {"role": "assistant", "content": "host fixture response"},
                                             "finish_reason": "stop"}]})
        except (ValueError, json.JSONDecodeError):
            self.respond(400, {"error": "invalid JSON"})

server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), ModelFixture)
if server.server_port == 8787:
    server.server_close()
    raise RuntimeError("Host fixture unexpectedly selected the Studio port")
thread = threading.Thread(target=server.serve_forever, daemon=True)
thread.start()

def docker(*args):
    return subprocess.check_output(["docker", *args], cwd=root, text=True).strip()

try:
    if subprocess.run(["docker", "inspect", name], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode == 0:
        raise RuntimeError("Disposable container name already exists")
    with tempfile.TemporaryDirectory(prefix="aruvi-host-model-env-") as temporary:
        env_file = Path(temporary) / "environment.env"
        env_file.write_text(f"ARUVI_MOBILE_API_TOKEN={token}\nARUVI_MCP_API_TOKEN={token}\n")
        env_file.chmod(0o600)
        docker("create", "--name", name, "--restart", "no", "--env-file", str(env_file),
               "-e", "HTTP_PROXY=", "-e", "HTTPS_PROXY=", "-e", "ALL_PROXY=", "-e", "NO_PROXY=",
               "-e", "http_proxy=", "-e", "https_proxy=", "-e", "all_proxy=", "-e", "no_proxy=",
               "--tmpfs", "/data:mode=1777", "--tmpfs", "/repositories:mode=1777",
               "-p", "127.0.0.1::8787", image)
        owned = True
        docker("start", name)
        address = docker("port", name, "8787/tcp").splitlines()[0]
        if not address.startswith("127.0.0.1:"):
            raise RuntimeError("Studio fixture did not bind loopback")
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
                raise RuntimeError("Studio fixture did not become healthy")
            time.sleep(.25)

        def call(action, arguments, expect_error=False):
            payload = json.dumps({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {
                "name": "aruvi_models", "arguments": {"action": action, "arguments": arguments},
            }}).encode()
            request = urllib.request.Request(base + "/api/mcp", data=payload,
                headers={"Authorization": "Bearer " + token, "Content-Type": "application/json", "Accept": "application/json"})
            with opener.open(request, timeout=20) as response:
                result = json.load(response)["result"]
            if expect_error:
                if not result.get("isError"):
                    raise RuntimeError("Expected the container-loopback control to fail")
                return None
            if result.get("isError"):
                raise RuntimeError("Model fixture MCP action failed: " + action)
            return result["structuredContent"]["result"]

        loopback = call("create_provider", {"name": "Container loopback control", "base_url": f"http://127.0.0.1:{server.server_port}/v1"})
        call("discover_provider_models", {"provider_id": loopback["id"]}, expect_error=True)
        provider = call("create_provider", {"name": "Docker host fixture", "base_url": f"http://host.docker.internal:{server.server_port}/v1"})
        models = call("discover_provider_models", {"provider_id": provider["id"]})
        if not any(model["id"] == "synthetic-host-model" for model in models["models"]):
            raise RuntimeError("Host fixture model was not discovered")
        reply = call("run_model_chat_completion", {"provider_id": provider["id"], "model": "synthetic-host-model",
            "messages": [{"role": "user", "content": "fixture"}], "max_tokens": 16})
        if reply.get("content") != "host fixture response":
            raise RuntimeError("Host fixture completion did not match")
    print("Docker Studio reached a loopback Mac host fixture for model discovery and MCP completion.")
finally:
    server.shutdown()
    server.server_close()
    thread.join()
    if owned and subprocess.run(["docker", "rm", "--force", name], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode != 0:
        print("Failed to remove disposable host-model fixture", file=sys.stderr)
        if sys.exc_info()[0] is None:
            raise RuntimeError("Host-model fixture cleanup failed")
