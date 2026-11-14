// Exercise authenticated browser chat against the configured real coding model.
import assert from "node:assert/strict";
import { chromium } from "@playwright/test";
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";

const model = process.env.ARUVI_CODING_TEST_MODEL || "qwen25-coder-15b-q4";
const streaming = process.argv.includes("--stream");
const plan = JSON.parse(execFileSync("bash", ["scripts/env", "worktree", "plan"], { encoding: "utf8" }));
const env = `${plan.state}/environment.env`;
const line = readFileSync(env, "utf8").split("\n").find((value) => value.startsWith("ARUVI_MOBILE_API_TOKEN="));
if (!line) throw new Error("Mobile token is not configured");
const token = line.slice("ARUVI_MOBILE_API_TOKEN=".length);
const port = execFileSync("docker", ["compose", "--project-name", plan.project, "--project-directory", process.cwd(), "--env-file", env, "-f", "deploy/compose.worktree.yaml", "port", "app", "8787"], { encoding: "utf8" }).trim();
const providers = JSON.parse(execFileSync("scripts/mcp", ["call", "aruvi_models", '{"action":"list_providers","arguments":{}}'], { encoding: "utf8" })).result;
const provider = providers.find((item) => item.name === "Aruvi Model Host (Studio Coding Validation)");
if (!provider) throw new Error("Coding validation provider is not configured");
const browser = await chromium.launch({ executablePath: process.env.ARUVI_CHROME_PATH || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome", headless: true });
try {
  const page = await browser.newPage();
  await page.goto(`http://${port}/remote`);
  await page.locator("#tokenInput").fill(token);
  await page.locator("#saveButton").click();
  await page.getByRole("button", { name: "Models", exact: true }).click();
  const card = page.locator("#modelList .tool-card").filter({ hasText: model }).filter({ hasText: provider.name });
  await card.getByRole("button", { name: "Use for this browser" }).click();
  await page.getByRole("button", { name: "Chat", exact: true }).click();
  if (streaming) await page.getByLabel("Stream response").check();
  await page.locator("#chatComposer").fill("Write a JavaScript function sumEven(numbers) that returns the sum of even integers. Return only code.");
  const endpoint = streaming ? "/api/mobile/chat/stream" : "/api/mobile/chat/completions";
  const completed = page.waitForResponse((response) => response.url().endsWith(endpoint) && response.request().method() === "POST", { timeout: 120000 });
  await page.locator("#chatSendButton").click();
  const response = await completed;
  assert.equal(response.ok(), true, `Chat API returned ${response.status()}`);
  if (!streaming) { const result = await response.json(); assert.match(result.content, /sumEven/); }
  await page.locator("#chatConversation .bubble.assistant").getByText(/sumEven/).waitFor({ timeout: 120000 });
  if (streaming) await page.locator("#status.ok").getByText("Connected", { exact: true }).waitFor({ timeout: 120000 });
  console.log(`Authenticated browser ${streaming ? "streaming " : ""}chat returned real coding-model output.`);
} finally { await browser.close(); }
