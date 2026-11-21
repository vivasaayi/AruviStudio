#!/usr/bin/env python3
"""Ensure the documented client pin matches the tested consumer and lockfile."""
from pathlib import Path
import re
import subprocess
import sys

root = Path(__file__).resolve().parent.parent
url = "https://github.com/vivasaayi/AruviStudio.git"
dependency = re.compile(
    r'aruvi-model-client\s*=\s*\{\s*git\s*=\s*"'
    + re.escape(url)
    + r'"\s*,\s*rev\s*=\s*"([0-9a-f]{40})"\s*\}'
)
readme = (root / "crates/aruvi-model-client/README.md").read_text()
manifest = (root / "validation/model-client-consumer/Cargo.toml").read_text()
lock = (root / "validation/model-client-consumer/Cargo.lock").read_text()
readme_pins, manifest_pins = dependency.findall(readme), dependency.findall(manifest)
if len(readme_pins) != 1 or len(manifest_pins) != 1 or readme_pins != manifest_pins:
    sys.exit("Client README and consumer manifest must contain the same single Git pin")
revision = manifest_pins[0]
source = f'git+{url}?rev={revision}#{revision}'
packages = re.findall(r'(?ms)^\[\[package\]\]\n(.*?)(?=^\[\[package\]\]|\Z)', lock)
matches = [package for package in packages if re.search(r'^name = "aruvi-model-client"$', package, re.M)]
if len(matches) != 1 or not re.search(r'^source = "' + re.escape(source) + r'"$', matches[0], re.M):
    sys.exit("Client consumer lockfile does not match the documented Git pin")

# A matching dependency string does not establish that the package used by
# Studio is still the package consumed at that revision. Compare its manifest
# and all source files, including untracked additions. README.md is excluded
# because updating its own pin necessarily changes it after the pinned commit.
package = "crates/aruvi-model-client"
try:
    tracked = subprocess.check_output(
        ["git", "ls-tree", "-r", "--name-only", revision, "--", "LICENSE", f"{package}/Cargo.toml", f"{package}/src"],
        cwd=root, stderr=subprocess.PIPE,
    ).decode().splitlines()
except subprocess.CalledProcessError:
    sys.exit(f"Client pin {revision} is not available locally; fetch that revision before checking source parity")
if not {"LICENSE", f"{package}/Cargo.toml"}.issubset(tracked):
    sys.exit("Pinned client manifest or license was not found")
local = {"LICENSE", f"{package}/Cargo.toml"}
local.update(str(path.relative_to(root)) for path in (root / package / "src").rglob("*") if path.is_file())
if set(tracked) != local:
    sys.exit("Client source files differ from the pinned consumer; update and test the pin")
for path in tracked:
    pinned = subprocess.check_output(["git", "show", f"{revision}:{path}"], cwd=root)
    if not (root / path).is_file() or (root / path).read_bytes() != pinned:
        sys.exit(f"Client source differs from the pinned consumer: {path}; update and test the pin")
print("Client README, consumer manifest, and lockfile pin agree:", revision)
print("Client manifest, license and sources match the pinned consumer")
