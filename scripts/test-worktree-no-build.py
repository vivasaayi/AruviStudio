#!/usr/bin/env python3
"""Exercise no-build worktree startup without touching any Docker resources."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

root = Path(__file__).resolve().parent.parent
with tempfile.TemporaryDirectory(prefix="aruvi-no-build-") as temporary:
    base = Path(temporary)
    docker = base / "docker"
    log = base / "calls.jsonl"
    docker.write_text(f"#!{sys.executable}\n" + r'''
import json, os, sys
args = sys.argv[1:]
with open(os.environ["FAKE_DOCKER_LOG"], "a") as log:
    log.write(json.dumps({"args": args, "image": os.environ.get("APP_IMAGE"), "port": os.environ.get("PUBLISHED_PORT")}) + "\n")
if args[:2] == ["image", "inspect"]:
    sys.exit(1 if os.environ.get("FAKE_IMAGE_MISSING") else 0)
if "config" in args:
    project = args[args.index("--project-name") + 1]
    port = "12345" if os.environ.get("FAKE_UNSAFE_PORT") else os.environ.get("PUBLISHED_PORT")
    print(json.dumps({"services":{"app":{"ports":[{"published":port}]}},"volumes":{"data":{"name":project+"_data"}},"networks":{"default":{"name":project+"_default"}}}))
elif "port" in args:
    print("127.0.0.1:49152")
''')
    docker.chmod(0o755)
    env = {**os.environ, "PATH": str(base) + os.pathsep + os.environ["PATH"], "WORK_ENV_STATE_ROOT": str(base / "state"), "FAKE_DOCKER_LOG": str(log), "APP_IMAGE": "unexpected-parent-image", "PUBLISHED_PORT": "54321"}

    def run(**extra):
        log.write_text("")
        result = subprocess.run(["bash", "scripts/env", "worktree", "up", "--no-build"], cwd=root, env={**env, **extra}, text=True, capture_output=True)
        calls = [json.loads(line) for line in log.read_text().splitlines()]
        return result, calls

    result, calls = run()
    if result.returncode != 0:
        raise SystemExit("No-build fixture failed: " + result.stderr)
    up = [entry for entry in calls if "up" in entry["args"]]
    if len(up) != 1 or not {"--no-build", "--pull", "never", "--wait"}.issubset(up[0]["args"]):
        raise SystemExit("No-build startup did not use the expected Compose options")
    if any("build" in entry["args"] for entry in calls) or up[0]["port"] != "0" or up[0]["image"] == "unexpected-parent-image":
        raise SystemExit("No-build startup used an unsafe image, port or build command")
    env_file = next((base / "state").glob("*/environment.env"))
    secrets = [line.split("=", 1)[1] for line in env_file.read_text().splitlines() if line.startswith(("ARUVI_MOBILE_API_TOKEN=", "ARUVI_MCP_API_TOKEN="))]
    if len(secrets) != 2 or any(secret in result.stdout + result.stderr for secret in secrets):
        raise SystemExit("No-build startup exposed or failed to initialize credentials")

    for extra in ({"FAKE_IMAGE_MISSING": "1"}, {"FAKE_UNSAFE_PORT": "1"}):
        result, calls = run(**extra)
        if result.returncode == 0 or any("up" in entry["args"] for entry in calls):
            raise SystemExit("No-build startup did not reject a missing image or unsafe port")
print("No-build worktree startup: image, isolation, credentials and failure checks passed.")
