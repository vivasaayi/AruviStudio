import { spawnSync } from "node:child_process";
import path from "node:path";

const workspaceRoot = process.cwd();

const result = spawnSync("npx", ["playwright", "test"], {
  cwd: workspaceRoot,
  stdio: "inherit",
});

spawnSync("node", [path.join(workspaceRoot, "scripts", "ci", "generate-test-dashboard.mjs")], {
  cwd: workspaceRoot,
  stdio: "inherit",
});

process.exit(result.status ?? 1);
