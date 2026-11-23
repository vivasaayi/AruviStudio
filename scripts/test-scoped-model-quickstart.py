#!/usr/bin/env python3
"""Exercise browser quick start with a disposable scoped Model Host fixture.

Requires the Model Host scoped-access checkout and an already-built native host
binary. It does not modify the deployed host or Studio branch data.
"""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

studio = Path(__file__).resolve().parent.parent
host_repo = Path(os.environ.get("ARUVI_MODEL_HOST_TEST_REPO", studio.parent / "aruvi-model-host-scoped-integration")).resolve(strict=True)
host_binary = Path(os.environ.get("ARUVI_MODEL_HOST_TEST_BIN", host_repo / "target/debug/aruvi-model-host")).resolve(strict=True)
os.environ["MODEL_HOST_TEST_BIN"] = str(host_binary)
sys.path.insert(0, str(host_repo / "tests"))
from lifecycle import Lifecycle  # noqa: E402

fixture = Lifecycle("test_inference_token_cannot_manage_the_host")
fixture.setUp()
try:
    worker = fixture.root / "quickstart-worker.py"
    source = (host_repo / "tests/fake_worker.py").read_text()
    before = "'content': 'fixture response'"
    assert source.count(before) == 1, "Scoped fixture worker response changed"
    worker.write_text(source.replace(before, "'content': 'function sumEven(numbers) { return 0; }'"))
    worker.chmod(0o755)
    fixture.stop()
    fixture.environment["MODEL_HOST_LLAMA_BIN"] = str(worker)
    fixture.start()
    fixture.register()
    assert fixture.install()["installation"] == "installed"
    with tempfile.TemporaryDirectory(prefix="aruvi-scoped-quickstart-") as directory:
        root = Path(directory)
        (root / "scripts").mkdir()
        (root / "state").mkdir()
        env_file = root / "state/environment.env"
        env_file.write_text("MODEL_HOST_TOKEN=fixture-token\nMODEL_HOST_INFERENCE_TOKEN=fixture-inference-token\n")
        env_file.chmod(0o600)
        (root / "plan.json").write_text(json.dumps({"state": str(root / "state")}))
        (root / "status.json").write_text(json.dumps({"registered": True, "endpoint": {"url": fixture.url}}))
        shim = root / "scripts/env"
        shim.write_text('#!/bin/bash\nif [ "$2" = "plan" ]; then cat "$(dirname "$0")/../plan.json"; else cat "$(dirname "$0")/../status.json"; fi\n')
        shim.chmod(0o755)
        env = dict(os.environ, ARUVI_TEST_MODEL_HOST=str(root), ARUVI_CODING_TEST_MODEL="fixture")
        command = ["bash", "scripts/test", "host-model-quickstart", "--packaged", "--credential-scope", "inference"]
        success = subprocess.run(command, cwd=studio, env=env, capture_output=True, text=True, timeout=180)
        assert success.returncode == 0, "Scoped quick-start fixture failed: " + success.stderr.replace("fixture-inference-token", "<redacted>").replace("fixture-token", "<redacted>")
        assert "inference-credential quick start" in success.stdout
        assert "fixture-inference-token" not in success.stdout + success.stderr
        env_file.write_text("MODEL_HOST_TOKEN=fixture-token\n")
        missing = subprocess.run(command, cwd=studio, env=env, capture_output=True, text=True, timeout=30)
        assert missing.returncode != 0 and "inference credential is unavailable" in missing.stderr
        assert "fixture-token" not in missing.stdout + missing.stderr
        env_file.write_text("MODEL_HOST_TOKEN=fixture-token\nMODEL_HOST_INFERENCE_TOKEN=fixture-token\n")
        privileged = subprocess.run(command, cwd=studio, env=env, capture_output=True, text=True, timeout=30)
        assert privileged.returncode != 0 and "was not denied Model Host management access" in privileged.stderr
        assert "fixture-token" not in privileged.stdout + privileged.stderr
        print("Scoped packaged quick start passed; absent and management-capable credentials were rejected without fallback.")
finally:
    fixture.tearDown()
