#!/usr/bin/env python3
"""Exercise client pin parity checks in an isolated Git repository."""
from pathlib import Path
import shutil
import subprocess
import tempfile


def git(root, *args):
    return subprocess.check_output(["git", "-C", str(root), *args], stderr=subprocess.PIPE).decode().strip()


with tempfile.TemporaryDirectory(prefix="aruvi-pin-check-") as directory:
    root = Path(directory)
    git(root, "init", "-q")
    git(root, "config", "user.name", "Pin Test")
    git(root, "config", "user.email", "pin-test@example.invalid")
    package = root / "crates/aruvi-model-client"
    (package / "src").mkdir(parents=True)
    (package / "Cargo.toml").write_text('[package]\nname = "aruvi-model-client"\nversion = "0.1.0"\n')
    source = package / "src/lib.rs"
    source.write_text("pub fn original() {}\n")
    (root / "LICENSE").write_text("Fixture license\n")
    git(root, "add", "crates", "LICENSE")
    git(root, "commit", "-qm", "fixture")
    revision = git(root, "rev-parse", "HEAD")
    url = "https://github.com/vivasaayi/AruviStudio.git"
    dependency = f'aruvi-model-client = {{ git = "{url}", rev = "{revision}" }}\n'
    (package / "README.md").write_text(dependency)
    consumer = root / "validation/model-client-consumer"
    consumer.mkdir(parents=True)
    (consumer / "Cargo.toml").write_text(dependency)
    (consumer / "Cargo.lock").write_text(
        f'[[package]]\nname = "aruvi-model-client"\nsource = "git+{url}?rev={revision}#{revision}"\n'
    )
    scripts = root / "scripts"
    scripts.mkdir()
    shutil.copyfile(Path(__file__).with_name("check-model-client-pin.py"), scripts / "check-model-client-pin.py")

    def check(expected, message):
        result = subprocess.run(["python3", str(scripts / "check-model-client-pin.py")], capture_output=True, text=True)
        assert (result.returncode == 0) == expected, (result.stdout, result.stderr)
        assert message in result.stdout + result.stderr

    check(True, "sources match")
    source.write_text("pub fn changed() {}\n")
    check(False, "Client source differs")
    source.write_text("pub fn original() {}\n")
    extra = package / "src/new.rs"
    extra.write_text("pub fn new() {}\n")
    check(False, "Client source files differ")
    extra.unlink()
    source.unlink()
    check(False, "Client source files differ")
    print("Client pin check accepts matching sources and rejects changed, added and removed files")
