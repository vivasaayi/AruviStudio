// Exercise browser playback using the configured native Model Host fixture.
import assert from "node:assert/strict";
import { chromium } from "@playwright/test";
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";

const plan = JSON.parse(execFileSync("bash", ["scripts/env", "worktree", "plan"], { encoding: "utf8" }));
const env = `${plan.state}/environment.env`;
const line = readFileSync(env, "utf8").split("\n").find((value) => value.startsWith("ARUVI_MOBILE_API_TOKEN="));
if (!line) throw new Error("Mobile token is not configured");
const token = line.slice("ARUVI_MOBILE_API_TOKEN=".length);
const port = execFileSync("docker", ["compose", "--project-name", plan.project, "--project-directory", process.cwd(), "--env-file", env, "-f", "deploy/compose.worktree.yaml", "port", "app", "8787"], { encoding: "utf8" }).trim();
const providers = JSON.parse(execFileSync("scripts/mcp", ["call", "aruvi_models", '{"action":"list_providers","arguments":{}}'], { encoding: "utf8" })).result;
const provider = providers.find((item) => item.name === "Aruvi Model Host (Native Staging)");
if (!provider) throw new Error("Native Model Host provider is not configured");
const browser = await chromium.launch({ executablePath: process.env.ARUVI_CHROME_PATH || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome", headless: true });
try {
  const page = await browser.newPage();
  await page.goto(`http://${port}/remote`);
  await page.locator("#tokenInput").fill(token);
  await page.locator("#saveButton").click();
  await page.getByRole("button", { name: "Voice", exact: true }).click();
  await page.locator(`#speechProvider option[value="${provider.id}"]`).waitFor({ state: "attached" });
  await page.locator("#speechProvider").selectOption(provider.id);
  await page.locator("#speechModel").fill("piper-ljspeech");
  await page.locator("#speechText").fill("Hello from the shared model host.");
  await page.getByRole("button", { name: "Generate speech" }).click();
  await page.waitForFunction(() => document.querySelector("#voiceMeta").textContent === "Speech ready to play.", null, { timeout: 60000 });
  await page.waitForFunction(() => Number.isFinite(document.querySelector("#generatedSpeech").duration) && document.querySelector("#generatedSpeech").duration > 0, null, { timeout: 10000 });
  assert.equal(await page.locator("#generatedSpeech").isVisible(), true);
  console.log("Browser generated and loaded real WAV audio from native Model Host.");
} finally { await browser.close(); }
