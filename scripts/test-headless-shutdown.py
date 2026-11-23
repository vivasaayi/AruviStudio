#!/usr/bin/env python3
"""Exercise SIGTERM and persisted state in an isolated headless server."""
import json
import os
from pathlib import Path
import signal
import socket
import sqlite3
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request

if len(sys.argv) != 2:
    raise SystemExit("Usage: python3 scripts/test-headless-shutdown.py /path/to/aruvi-studio-server")
binary = str(Path(sys.argv[1]).resolve(strict=True))
with tempfile.TemporaryDirectory(prefix="aruvi-shutdown-") as directory:
    root = Path(directory)
    with socket.socket() as listener:
        listener.bind(("127.0.0.1", 0))
        port = listener.getsockname()[1]
    base = f"http://127.0.0.1:{port}"
    environment = {key: value for key, value in os.environ.items() if not key.startswith("ARUVI_")}
    environment.update(ARUVI_APP_DATA_DIR=str(root), ARUVI_DB_PATH=str(root / "studio.db"),
        ARUVI_WEBHOOK_HOST="127.0.0.1", ARUVI_WEBHOOK_PORT=str(port),
        ARUVI_MOBILE_API_TOKEN="shutdown-fixture", ARUVI_MCP_API_TOKEN="shutdown-fixture")
    headers = {"Authorization": "Bearer shutdown-fixture", "Content-Type": "application/json"}
    process = None
    def start():
        child = subprocess.Popen([binary], env=environment, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        for _ in range(150):
            if child.poll() is not None: raise RuntimeError("Server exited during startup")
            try:
                with urllib.request.urlopen(base + "/health", timeout=1): return child
            except (OSError, urllib.error.URLError): time.sleep(0.1)
        child.kill(); child.wait(timeout=5)
        raise RuntimeError("Server did not become healthy")
    try:
        process = start()
        request = urllib.request.Request(base + "/api/mobile/products", data=json.dumps({"name":"Shutdown fixture"}).encode(), headers=headers)
        with urllib.request.urlopen(request, timeout=5) as response: product = json.load(response)
        process.send_signal(signal.SIGTERM)
        if process.wait(timeout=5) != 0: raise RuntimeError("SIGTERM did not produce a clean server exit")
        with sqlite3.connect(root / "studio.db") as db:
            if db.execute("PRAGMA quick_check").fetchone()[0] != "ok": raise RuntimeError("Database integrity check failed")
        process = start()
        with urllib.request.urlopen(urllib.request.Request(base + "/api/mobile/products", headers=headers), timeout=5) as response: products = json.load(response)
        if not any(item["id"] == product["id"] for item in products): raise RuntimeError("Product did not persist after SIGTERM")
        process.send_signal(signal.SIGINT)
        if process.wait(timeout=5) != 0: raise RuntimeError("SIGINT did not produce a clean server exit")
        print("Headless SIGTERM and SIGINT exit cleanly; database and product survive restart.")
    finally:
        if process is not None and process.poll() is None:
            process.kill(); process.wait(timeout=5)
