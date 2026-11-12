#!/usr/bin/env python3
"""Create per-environment bearer tokens without exposing them in commands or output."""
import fcntl
import hashlib
import os
from pathlib import Path
import secrets
import sys

profile, action = sys.argv[1:]
if profile not in {"worktree", "staging", "production"}:
    raise SystemExit("invalid environment")
root = Path(__file__).resolve().parent.parent
suffix = "-" + hashlib.sha256(str(root).encode()).hexdigest()[:12] if profile == "worktree" else ""
state_root = Path(os.environ.get("WORK_ENV_STATE_ROOT", str(Path.home() / ".local/share/work-environments"))).expanduser().resolve()
state = state_root / f"aruvi-studio-{profile}{suffix}"
env_file = state / "environment.env"
if action in {"status", "logs", "stop", "destroy"} and not env_file.exists():
    raise SystemExit("environment has not been initialized")
state.mkdir(parents=True, exist_ok=True, mode=0o700)
os.chmod(state, 0o700)
lock_fd = os.open(state / ".token-lock", os.O_CREAT | os.O_RDWR, 0o600)
with os.fdopen(lock_fd, "r+") as lock:
    fcntl.flock(lock, fcntl.LOCK_EX)
    lines = env_file.read_text().splitlines() if env_file.exists() else []
    keys = {line.split("=", 1)[0] for line in lines if "=" in line and not line.startswith("#")}
    for key in ("ARUVI_MOBILE_API_TOKEN", "ARUVI_MCP_API_TOKEN"):
        if key not in keys:
            lines.append(f"{key}={secrets.token_hex(32)}")
    temp = state / f"environment.tokens.{os.getpid()}.tmp"
    fd = os.open(temp, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(fd, "w") as output:
        output.write("\n".join(lines) + "\n")
    os.replace(temp, env_file)
