#!/usr/bin/env python3
"""Start this worktree with its existing image, without a build or pull."""
import json
import os
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parent.parent
config = json.loads((root / "deploy/product.json").read_text())
if config.get("database_service", "db") or config.get("migration_command") or config.get("seed_command"):
    raise SystemExit("No-build startup does not support configured database or lifecycle hooks; use the standard launcher.")

# The shared launcher validates dynamic ports and project-scoped resources and
# returns a credential-free summary. It also prepares the private environment.
summary = json.loads(subprocess.check_output(["bash", str(root / "scripts/env"), "worktree", "config"], cwd=root, text=True))
if summary.get("environment") != "worktree" or Path(summary["source"]).resolve() != root:
    raise SystemExit("Unexpected worktree identity from the environment launcher.")
env_file = Path(summary["state"]) / "environment.env"
values = {}
for line in env_file.read_text().splitlines():
    if line and not line.startswith("#"):
        key, value = line.split("=", 1)
        values[key] = value
if values.get("APP_IMAGE") != summary.get("image"):
    raise SystemExit("Worktree image changed during validation; retry.")
env = {**os.environ, **values}
subprocess.run(["docker", "image", "inspect", summary["image"]], env=env, stdout=subprocess.DEVNULL, check=True)
base = ["docker", "compose", "--project-name", summary["project"], "--project-directory", str(root), "--env-file", str(env_file), "-f", summary["compose"]]
subprocess.run(base + ["up", "-d", "--no-build", "--pull", "never", "--wait"], cwd=root, env=env, check=True)
subprocess.run(base + ["port", config.get("app_service", "app"), str(config["app_port"])], cwd=root, env=env, check=True)
