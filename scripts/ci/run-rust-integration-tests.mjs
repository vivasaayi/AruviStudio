import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";

const workspaceRoot = process.cwd();
const integrationDir = path.join(workspaceRoot, "src-tauri", "tests");

if (!fs.existsSync(integrationDir)) {
  console.log("No Rust integration tests found in src-tauri/tests.");
  process.exit(0);
}

const testFiles = fs.readdirSync(integrationDir).filter((entry) => entry.endsWith(".rs"));
if (testFiles.length === 0) {
  console.log("No Rust integration tests found in src-tauri/tests.");
  process.exit(0);
}

const result = spawnSync("cargo", ["test", "--manifest-path", "src-tauri/Cargo.toml", "--tests"], {
  cwd: workspaceRoot,
  stdio: "inherit",
});

process.exit(result.status ?? 1);
