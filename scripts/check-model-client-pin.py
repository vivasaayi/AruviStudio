#!/usr/bin/env python3
"""Ensure the documented client pin matches the tested consumer and lockfile."""
from pathlib import Path
import re
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
print("Client README, consumer manifest, and lockfile pin agree:", revision)
