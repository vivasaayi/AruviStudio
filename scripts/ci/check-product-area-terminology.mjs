import { readdirSync, readFileSync, statSync } from "node:fs";
import path from "node:path";
import process from "node:process";

const repoRoot = process.cwd();
const scanRoots = ["src-tauri/src", "src", "e2e"];
const targetedFiles = [
  "mobile/App.tsx",
  "mobile/src/types.ts",
];
const ignoredDirs = new Set([
  ".git",
  "dist",
  "node_modules",
  "src-tauri/target",
  "target",
]);
const scannedExtensions = new Set([
  ".rs",
  ".ts",
  ".tsx",
  ".js",
  ".jsx",
  ".mjs",
]);
const legacyTerms = /\b(create_module|module_id|modules?|Module|Modules)\b/g;
const targetedLegacyTerms = /\b(create_module|module_id|modules?|module|Module|Modules|ModuleTree)\b/g;
const frontendLegacyWorkItemListTerms = /\b(listWorkItems)\b|["']list_work_items["']/g;

function isAllowedLegacyProductAreaTerm(normalizedPath, content, matchIndex) {
  if (normalizedPath === path.normalize("src-tauri/src/persistence/db/db_tests.rs")) {
    return true;
  }

  if (normalizedPath !== path.normalize("src-tauri/src/persistence/db.rs")) {
    return false;
  }

  const testModuleIndex = content.indexOf("mod tests {");
  return testModuleIndex >= 0 && matchIndex > testModuleIndex;
}

function isIgnored(relativePath) {
  return [...ignoredDirs].some(
    (ignoredDir) => relativePath === ignoredDir || relativePath.startsWith(`${ignoredDir}${path.sep}`),
  );
}

function* walk(relativeDir) {
  if (isIgnored(relativeDir)) {
    return;
  }

  const absoluteDir = path.join(repoRoot, relativeDir);
  for (const entry of readdirSync(absoluteDir)) {
    const relativePath = path.join(relativeDir, entry);
    if (isIgnored(relativePath)) {
      continue;
    }

    const absolutePath = path.join(repoRoot, relativePath);
    const stat = statSync(absolutePath);
    if (stat.isDirectory()) {
      yield* walk(relativePath);
    } else if (stat.isFile() && scannedExtensions.has(path.extname(entry))) {
      yield relativePath;
    }
  }
}

const violations = [];
const performanceViolations = [];
for (const root of scanRoots) {
  for (const filePath of walk(root)) {
    const normalizedPath = path.normalize(filePath);

    const content = readFileSync(path.join(repoRoot, normalizedPath), "utf8");
    for (const match of content.matchAll(legacyTerms)) {
      if (isAllowedLegacyProductAreaTerm(normalizedPath, content, match.index)) {
        continue;
      }
      const lineNumber = content.slice(0, match.index).split("\n").length;
      const line = content.split("\n")[lineNumber - 1].trim();
      violations.push(`${normalizedPath}:${lineNumber}: ${line}`);
    }

    if (normalizedPath.startsWith(`src${path.sep}`)) {
      for (const match of content.matchAll(frontendLegacyWorkItemListTerms)) {
        const lineNumber = content.slice(0, match.index).split("\n").length;
        const line = content.split("\n")[lineNumber - 1].trim();
        performanceViolations.push(`${normalizedPath}:${lineNumber}: ${line}`);
      }
    }
  }
}

for (const filePath of targetedFiles) {
  const normalizedPath = path.normalize(filePath);
  const content = readFileSync(path.join(repoRoot, normalizedPath), "utf8");
  for (const match of content.matchAll(targetedLegacyTerms)) {
    const lineNumber = content.slice(0, match.index).split("\n").length;
    const line = content.split("\n")[lineNumber - 1].trim();
    violations.push(`${normalizedPath}:${lineNumber}: ${line}`);
  }
}

if (violations.length > 0) {
  console.error("Legacy product hierarchy terminology found outside allowed migration tests.");
  console.error("Use product_area/Product Area in persisted values, APIs, UI, and MCP language.");
  for (const violation of violations) {
    console.error(`- ${violation}`);
  }
  process.exit(1);
}

if (performanceViolations.length > 0) {
  console.error("Legacy frontend work-item list usage found.");
  console.error("Use listWorkItemsPage/list_work_items_page so large Mayyam-scale products stay paged.");
  for (const violation of performanceViolations) {
    console.error(`- ${violation}`);
  }
  process.exit(1);
}

console.log("Product hierarchy terminology and performance guard check passed.");
