#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const args = process.argv.slice(2);

const usage = `
Usage:
  npm run version:rotate -- <patch|minor|major|X.Y.Z> [--commit] [--tag] [--no-backup] [--dry-run]

Examples:
  npm run version:rotate -- patch
  npm run version:rotate -- 0.2.0 --commit --tag
`;

const bumpArg = args.find((arg) => !arg.startsWith("--"));
const options = new Set(args.filter((arg) => arg.startsWith("--")));

if (!bumpArg || options.has("--help")) {
  console.log(usage.trim());
  process.exit(bumpArg ? 0 : 1);
}

const shouldCommit = options.has("--commit");
const shouldTag = options.has("--tag");
const shouldBackup = !options.has("--no-backup");
const dryRun = options.has("--dry-run");

if (shouldTag && !shouldCommit) {
  fail("Use --commit with --tag so the tag points at the version bump commit.");
}

const packageJsonPath = resolve(repoRoot, "package.json");
const packageLockPath = resolve(repoRoot, "package-lock.json");
const tauriConfigPath = resolve(repoRoot, "src-tauri", "tauri.conf.json");
const cargoTomlPath = resolve(repoRoot, "src-tauri", "Cargo.toml");
const cargoLockPath = resolve(repoRoot, "src-tauri", "Cargo.lock");
const versionFiles = [
  packageJsonPath,
  packageLockPath,
  tauriConfigPath,
  cargoTomlPath,
  cargoLockPath,
];

const packageJson = readJson(packageJsonPath);
const currentVersion = packageJson.version;
assertVersionsMatch(currentVersion);
const nextVersion = resolveNextVersion(currentVersion, bumpArg);
const tagName = `v${nextVersion}`;

if (currentVersion === nextVersion) {
  fail(`Version is already ${nextVersion}.`);
}

if (gitTagExists(tagName)) {
  fail(`Git tag already exists: ${tagName}`);
}

if (dryRun) {
  console.log(`Would rotate version ${currentVersion} -> ${nextVersion}`);
  console.log(`Would update ${versionFiles.map((file) => relative(file)).join(", ")}`);
  console.log(shouldBackup ? "Would run backup.sh" : "Would skip database backup");
  if (shouldCommit) {
    console.log(`Would commit: chore: release ${tagName}`);
  }
  if (shouldTag) {
    console.log(`Would create git tag: ${tagName}`);
  }
  process.exit(0);
}

if (shouldBackup) {
  execFileSync("bash", ["backup.sh"], {
    cwd: repoRoot,
    env: {
      ...process.env,
      ARUVI_BACKUP_LABEL: process.env.ARUVI_BACKUP_LABEL ?? `v${currentVersion}`,
    },
    stdio: "inherit",
  });
}

updateJsonFile(packageJsonPath, (json) => {
  json.version = nextVersion;
});

updateJsonFile(packageLockPath, (json) => {
  json.version = nextVersion;
  if (json.packages?.[""]) {
    json.packages[""].version = nextVersion;
  }
});

updateJsonFile(tauriConfigPath, (json) => {
  json.version = nextVersion;
});

replacePackageVersion(cargoTomlPath, "aruvi-studio", nextVersion);
if (existsSync(cargoLockPath)) {
  replacePackageVersion(cargoLockPath, "aruvi-studio", nextVersion);
}

console.log(`Version rotated: ${currentVersion} -> ${nextVersion}`);

if (shouldCommit) {
  execFileSync("git", ["add", ...versionFiles.map((file) => relative(file))], {
    cwd: repoRoot,
    stdio: "inherit",
  });
  execFileSync("git", ["commit", "-m", `chore: release ${tagName}`], {
    cwd: repoRoot,
    stdio: "inherit",
  });
}

if (shouldTag) {
  execFileSync("git", ["tag", "-a", tagName, "-m", `AruviStudio ${nextVersion}`], {
    cwd: repoRoot,
    stdio: "inherit",
  });
  console.log(`Created git tag: ${tagName}`);
}

function readJson(path) {
  return JSON.parse(readFileSync(path, "utf8"));
}

function updateJsonFile(path, mutate) {
  const json = readJson(path);
  mutate(json);
  writeFileSync(path, `${JSON.stringify(json, null, 2)}\n`);
}

function replacePackageVersion(path, packageName, version) {
  const source = readFileSync(path, "utf8");
  const pattern = new RegExp(
    `(\\[\\[package\\]\\]\\nname = "${escapeRegExp(packageName)}"\\nversion = ")[^"]+(")`,
    "m",
  );

  if (pattern.test(source)) {
    writeFileSync(path, source.replace(pattern, `$1${version}$2`));
    return;
  }

  const cargoTomlPattern = new RegExp(`(^name = "${escapeRegExp(packageName)}"\\nversion = ")[^"]+(")`, "m");
  if (cargoTomlPattern.test(source)) {
    writeFileSync(path, source.replace(cargoTomlPattern, `$1${version}$2`));
    return;
  }

  fail(`Could not find ${packageName} version in ${relative(path)}`);
}

function assertVersionsMatch(expectedVersion) {
  const versions = [
    ["package.json", expectedVersion],
    ["package-lock.json", readJson(packageLockPath).version],
    ["package-lock.json packages root", readJson(packageLockPath).packages?.[""]?.version],
    ["src-tauri/tauri.conf.json", readJson(tauriConfigPath).version],
    ["src-tauri/Cargo.toml", readPackageVersion(cargoTomlPath, "aruvi-studio")],
    ["src-tauri/Cargo.lock", readPackageVersion(cargoLockPath, "aruvi-studio")],
  ];

  const mismatches = versions.filter(([, version]) => version !== expectedVersion);
  if (mismatches.length > 0) {
    fail(
      [
        `Version files are out of sync with package.json (${expectedVersion}):`,
        ...mismatches.map(([label, version]) => `  ${label}: ${version ?? "missing"}`),
      ].join("\n"),
    );
  }
}

function readPackageVersion(path, packageName) {
  const source = readFileSync(path, "utf8");
  const lockMatch = source.match(
    new RegExp(`\\[\\[package\\]\\]\\nname = "${escapeRegExp(packageName)}"\\nversion = "([^"]+)"`, "m"),
  );
  if (lockMatch) {
    return lockMatch[1];
  }

  const tomlMatch = source.match(new RegExp(`^name = "${escapeRegExp(packageName)}"\\nversion = "([^"]+)"`, "m"));
  return tomlMatch?.[1] ?? null;
}

function resolveNextVersion(currentVersion, bump) {
  const match = currentVersion.match(/^(\d+)\.(\d+)\.(\d+)$/);
  if (!match) {
    fail(`Current version is not semver X.Y.Z: ${currentVersion}`);
  }

  const parts = match.slice(1).map(Number);
  if (bump === "major") {
    return `${parts[0] + 1}.0.0`;
  }
  if (bump === "minor") {
    return `${parts[0]}.${parts[1] + 1}.0`;
  }
  if (bump === "patch") {
    return `${parts[0]}.${parts[1]}.${parts[2] + 1}`;
  }
  if (/^\d+\.\d+\.\d+$/.test(bump)) {
    return bump;
  }

  fail(`Invalid version bump: ${bump}`);
}

function gitTagExists(tagName) {
  try {
    execFileSync("git", ["rev-parse", "-q", "--verify", `refs/tags/${tagName}`], {
      cwd: repoRoot,
      stdio: "ignore",
    });
    return true;
  } catch {
    return false;
  }
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function relative(path) {
  return path.slice(repoRoot.length + 1);
}

function fail(message) {
  console.error(message);
  process.exit(1);
}
