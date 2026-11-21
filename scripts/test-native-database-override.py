#!/usr/bin/env python3
"""Check database override and environment precedence in disposable native servers."""
import argparse
from contextlib import closing
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

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("server", type=Path, help="Built native aruvi-studio-server binary")
parser.add_argument("--browser-executable", type=Path, help="Also exercise Routing with one persistent headless browser session")
args = parser.parse_args()
binary = args.server.resolve(strict=True)
opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
token = secrets.token_urlsafe(32)
process = None
browser = None

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
    with socket.socket() as reserve:
        reserve.bind(("127.0.0.1", 0))
        port = reserve.getsockname()[1]

    def start(forced=None):
        global process
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

    def browser_action(action, **fields):
        if browser is None: return
        browser.stdin.write(json.dumps({"action": action, **fields}) + "\n")
        browser.stdin.flush()
        line = browser.stdout.readline()
        if not line: raise RuntimeError("Disposable browser exited unexpectedly")
        result = json.loads(line)
        if not result.get("ok"): raise RuntimeError("Browser database fixture failed: " + result.get("error", "unknown error"))

    def browser_open(base, active, override=None, environment=False):
        browser_action("open", base=base, active=str(active), override=str(override) if override else None, environment=environment)

    try:
        if args.browser_executable:
            browser_env = {key: value for key, value in os.environ.items() if not key.startswith("ARUVI_")}
            browser_env.update(ARUVI_TEST_DB_CHROME=str(args.browser_executable.resolve(strict=True)), ARUVI_TEST_DB_TOKEN=token)
            browser = subprocess.Popen(["node", str(Path(__file__).resolve().parent / "test-native-database-browser.mjs")], env=browser_env,
                                       stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True, bufsize=1)
        base = start()
        try:
            call(base, "/api/mobile/database-settings", authenticated=False)
            raise RuntimeError("Unauthenticated database settings request was accepted")
        except urllib.error.HTTPError as error:
            if error.code != 401: raise
        settings(base, default_db, None, False)
        browser_open(base, default_db)
        call(base, "/api/mobile/products", "POST", {"name": "Default database fixture"})
        stop()

        # Reconstruct the immediately preceding schema in this disposable file.
        # Migration 53 adds only this nullable column; all earlier SQLx migration
        # records are retained exactly as created by the current server.
        with closing(sqlite3.connect(default_db)) as database:
            assert database.execute("SELECT MAX(version) FROM _sqlx_migrations").fetchone()[0] == 53
            database.execute("PRAGMA foreign_keys = ON")
            database.execute("ALTER TABLE planner_channel_bindings DROP COLUMN product_id")
            database.execute("DELETE FROM _sqlx_migrations WHERE version = 53")
            database.execute("INSERT INTO planner_sessions(id) VALUES (?)", ("legacy-session",))
            database.execute("INSERT INTO planner_channel_bindings(id,channel,remote_user_id,remote_conversation_id,planner_session_id) VALUES (?,?,?,?,?)", ("legacy-binding", "twilio_whatsapp", "user", "conversation", "legacy-session"))
            database.commit()
            assert not database.execute("PRAGMA foreign_key_check").fetchall()
            with closing(sqlite3.connect(root / "before-migration.db")) as backup:
                database.backup(backup)
                assert backup.execute("PRAGMA quick_check").fetchone()[0] == "ok"

        base = start()
        settings(base, default_db, None, False)
        browser_open(base, default_db)
        assert "Default database fixture" in names(base)
        with closing(sqlite3.connect(f"file:{default_db}?mode=ro", uri=True)) as database:
            assert database.execute("SELECT MAX(version) FROM _sqlx_migrations").fetchone()[0] == 53
            assert database.execute("SELECT planner_session_id, product_id FROM planner_channel_bindings WHERE id = ?", ("legacy-binding",)).fetchone() == ("legacy-session", None)
            assert database.execute("PRAGMA quick_check").fetchone()[0] == "ok"
            assert not database.execute("PRAGMA foreign_key_check").fetchall()
        if browser: browser_action("verify-conflict", draft=str(root / "draft.db"), external=str(root / "concurrent.db"))
        browser_action("verify-draft", path=str(override_db))
        browser_action("verify-save-draft", path=str(override_db), draft=str(root / "unsaved.db"))
        browser_action("verify-draft", path=str(root / "unsaved.db"), preexisting=True)
        if browser: browser_action("save", path=str(override_db), active=str(default_db))
        else: call(base, "/api/mobile/database-settings", "POST", {"path": str(override_db)})
        settings(base, default_db, override_db, False)
        stop()

        base = start()
        settings(base, override_db, override_db, False)
        browser_open(base, override_db, override_db)
        assert "Default database fixture" not in names(base)
        call(base, "/api/mobile/products", "POST", {"name": "Override database fixture"})
        stop()

        base = start()
        settings(base, override_db, override_db, False)
        browser_open(base, override_db, override_db)
        assert "Override database fixture" in names(base)
        if browser: browser_action("clear", active=str(override_db))
        else: call(base, "/api/mobile/database-settings", "POST", {"path": None})
        stop()

        base = start()
        settings(base, default_db, None, False)
        browser_open(base, default_db)
        assert "Default database fixture" in names(base) and "Override database fixture" not in names(base)
        call(base, "/api/mobile/database-settings", "POST", {"path": str(override_db)})
        stop()

        base = start(forced_db)
        settings(base, forced_db, override_db, True)
        browser_open(base, forced_db, override_db, True)
        assert "Default database fixture" not in names(base) and "Override database fixture" not in names(base)
        stop()

        base = start()
        settings(base, override_db, override_db, False)
        browser_open(base, override_db, override_db)
        assert "Override database fixture" in names(base)
        stop()
        print("Database migration, override, restart persistence, clear, environment precedence and authorization passed" + (" with browser checks." if browser else "."))
    finally:
        try:
            stop()
        finally:
            if browser is not None:
                if browser.poll() is None:
                    try:
                        browser_action("close")
                        browser.wait(timeout=10)
                    except (BrokenPipeError, RuntimeError, subprocess.TimeoutExpired):
                        browser.kill()
                        browser.wait()
