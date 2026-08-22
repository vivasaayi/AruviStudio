#!/usr/bin/env node

import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const repoRoot = process.cwd();
const tag = process.argv[2] ?? "";
const tagMatch = tag.match(/^v(\d+\.\d+\.\d+)$/);

if (!tagMatch) {
  fail(`Release tag must use vX.Y.Z format; received: ${tag || "<empty>"}`);
}

const expectedVersion = tagMatch[1];
const packageJson = readJson("package.json");
const packageLock = readJson("package-lock.json");
const tauriConfig = readJson("src-tauri/tauri.conf.json");
const cargoToml = readText("src-tauri/Cargo.toml");
const cargoLock = readText("src-tauri/Cargo.lock");

const versions = [
  ["package.json", packageJson.version],
  ["package-lock.json", packageLock.version],
  ["package-lock.json root package", packageLock.packages?.[""]?.version],
  ["src-tauri/tauri.conf.json", tauriConfig.version],
  ["src-tauri/Cargo.toml", readCargoVersion(cargoToml)],
  ["src-tauri/Cargo.lock", readCargoVersion(cargoLock)],
];

const mismatches = versions.filter(([, version]) => version !== expectedVersion);
if (mismatches.length > 0) {
  fail([
    `Release tag ${tag} does not match every version file:`,
    ...mismatches.map(([file, version]) => `  ${file}: ${version ?? "missing"}`),
  ].join("\n"));
}

console.log(`Release version validated: ${tag}`);

function readJson(relativePath) {
  return JSON.parse(readText(relativePath));
}

function readText(relativePath) {
  return readFileSync(resolve(repoRoot, relativePath), "utf8");
}

function readCargoVersion(source) {
  const lockMatch = source.match(/\[\[package\]\]\nname = "aruvi-studio"\nversion = "([^"]+)"/m);
  if (lockMatch) {
    return lockMatch[1];
  }

  return source.match(/^name = "aruvi-studio"\nversion = "([^"]+)"/m)?.[1] ?? null;
}

function fail(message) {
  console.error(message);
  process.exit(1);
}
