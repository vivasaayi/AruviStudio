// Exercise the browser provider quick start with a real native Model Host and disposable Studio data.
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { randomBytes, randomUUID } from "node:crypto";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { chromium } from "@playwright/test";

const root = process.cwd();
const host = resolve(process.env.ARUVI_TEST_MODEL_HOST || "../aruvi-model-host-studio-validation");
const model = process.env.ARUVI_CODING_TEST_MODEL || "qwen25-coder-7b-q4";
const packaged = process.argv.includes("--packaged");
const nativeEnv = { ...process.env, ARUVI_MODEL_HOST_ENV: "native" };
const hostPlan = JSON.parse(execFileSync("bash", ["scripts/env", "worktree", "plan"], { cwd: host, env: nativeEnv, encoding: "utf8" }));
const hostStatus = JSON.parse(execFileSync("bash", ["scripts/env", "worktree", "status"], { cwd: host, env: nativeEnv, encoding: "utf8" }));
const endpoint = new URL(hostStatus.endpoint?.url || "");
if (!hostStatus.registered || endpoint.protocol !== "http:" || endpoint.hostname !== "127.0.0.1" || !endpoint.port) throw new Error("A running loopback native Model Host worktree is required");
const hostLine = readFileSync(`${hostPlan.state}/environment.env`, "utf8").split("\n").find((line) => line.startsWith("MODEL_HOST_TOKEN="));
if (!hostLine || !hostLine.slice("MODEL_HOST_TOKEN=".length)) throw new Error("Model Host credential is unavailable");
const hostToken = hostLine.slice("MODEL_HOST_TOKEN=".length);
const plan = JSON.parse(execFileSync("bash", ["scripts/env", "worktree", "plan"], { cwd: root, encoding: "utf8" }));
const image = process.env.ARUVI_TEST_APP_IMAGE || readFileSync(`${plan.state}/environment.env`, "utf8").split("\n").find((line) => line.startsWith("APP_IMAGE="))?.slice("APP_IMAGE=".length);
if (!image) throw new Error("Studio candidate image is unavailable");
const mobileToken = randomBytes(24).toString("hex"), mcpToken = randomBytes(24).toString("hex");
const name = `${plan.project}-quickstart-${randomUUID().slice(0, 12)}`;
let created = false, browser;
for (const signal of ["SIGINT", "SIGTERM"]) process.once(signal, () => {
  if (created) { try { execFileSync("docker", ["rm", "-f", name], { stdio: "ignore" }); } catch { /* preserve signal exit */ } }
  process.exit(signal === "SIGINT" ? 130 : 143);
});
try {
  execFileSync("docker", ["run", "--rm", "-d", "--name", name, "-p", "127.0.0.1::8787", "--tmpfs", "/data:mode=1777", "--tmpfs", "/repositories:mode=1777", "-e", `ARUVI_MOBILE_API_TOKEN=${mobileToken}`, "-e", `ARUVI_MCP_API_TOKEN=${mcpToken}`, "-e", "ARUVI_LLM_CONFIG_PATH=/data/llm-config.json", image], { stdio: "pipe" });
  created = true;
  const port = execFileSync("docker", ["port", name, "8787/tcp"], { encoding: "utf8" }).trim();
  const origin = `http://${port}`;
  let healthy = false;
  for (let attempt = 0; attempt < 120; attempt++) {
    try { if ((await fetch(`${origin}/health`)).ok) { healthy = true; break; } } catch { /* wait for fixture */ }
    await new Promise((resolveWait) => setTimeout(resolveWait, 250));
  }
  if (!healthy) throw new Error("Disposable Studio fixture did not become healthy");
  browser = await chromium.launch({ executablePath: process.env.ARUVI_CHROME_PATH || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome", headless: true });
  const page = await browser.newPage();
  if (!packaged) {
    const html = readFileSync("src-tauri/src/services/remote_app.html", "utf8");
    await page.route("**/remote", (route) => route.fulfill({ contentType: "text/html", body: html }));
  }
  await page.goto(`${origin}/remote`);
  await page.locator("#tokenInput").fill(mobileToken);
  await page.locator("#saveButton").click();
  await page.getByRole("button", { name: "Models", exact: true }).click();
  await page.getByRole("button", { name: "Use Aruvi Model Host" }).click();
  assert.equal(await page.locator("#providerUrl").inputValue(), "");
  await page.locator("#providerUrl").fill(`http://host.docker.internal:${endpoint.port}/v1`);
  await page.locator("#providerKey").fill(hostToken);
  const createdProvider = page.waitForResponse((response) => response.url().endsWith("/api/mobile/providers") && response.request().method() === "POST");
  await page.locator("#createProviderButton").click();
  const providerResponse = await createdProvider;
  if (!providerResponse.ok()) throw new Error(`Provider creation failed (${providerResponse.status()})`);
  const provider = await providerResponse.json();
  assert.equal(provider.name, "Aruvi Model Host");
  assert.equal(Object.hasOwn(provider, "auth_secret_ref"), false);
  assert.equal(JSON.stringify(provider).includes(hostToken), false);
  await page.locator("#modelProvider").selectOption(provider.id);
  const discovery = page.waitForResponse((response) => response.url().endsWith("/api/mobile/providers/discover") && response.request().method() === "POST", { timeout: 60000 });
  await page.locator("#discoverModelsButton").click();
  if (!(await discovery).ok()) throw new Error("Model Host discovery failed");
  await page.locator("#discoveredModels").selectOption(model, { timeout: 60000 });
  assert.equal(await page.locator("#newModelName").inputValue(), model);
  const createdModel = page.waitForResponse((response) => response.url().endsWith("/api/mobile/models") && response.request().method() === "POST");
  await page.locator("#createModelButton").click();
  const modelResponse = await createdModel;
  if (!modelResponse.ok()) throw new Error(`Model registration failed (${modelResponse.status()})`);
  const registered = await modelResponse.json();
  async function listThroughMcp(action) {
    const response = await fetch(`${origin}/api/mcp`, { method: "POST", headers: { Authorization: `Bearer ${mcpToken}`, "Content-Type": "application/json", Accept: "application/json, text/event-stream" }, body: JSON.stringify({ jsonrpc: "2.0", id: 1, method: "tools/call", params: { name: "aruvi_models", arguments: { action, arguments: {} } } }) });
    if (!response.ok) throw new Error(`MCP listing failed (${response.status})`);
    const rpc = await response.json();
    assert.ok(!rpc.result?.isError);
    assert.equal(JSON.stringify(rpc).includes(hostToken), false);
    return rpc.result.structuredContent.result;
  }
  assert.ok((await listThroughMcp("list_providers")).some((item) => item.id === provider.id));
  assert.ok((await listThroughMcp("list_model_definitions")).some((item) => item.id === registered.id && item.name === model));
  const card = page.locator("#modelList .tool-card").filter({ hasText: model }).filter({ hasText: "Aruvi Model Host" });
  await card.getByRole("button", { name: "Use for this browser" }).click();
  await page.getByRole("button", { name: "Chat", exact: true }).click();
  await page.locator("#chatComposer").fill("Write a JavaScript function sumEven(numbers) that returns the sum of even integers. Return only code.");
  const completion = page.waitForResponse((response) => response.url().endsWith("/api/mobile/chat/completions") && response.request().method() === "POST", { timeout: 120000 });
  await page.locator("#chatSendButton").click();
  const reply = await completion;
  if (!reply.ok()) throw new Error(`Model Host completion failed (${reply.status()})`);
  assert.match((await reply.json()).content, /sumEven/);
  console.log(`Disposable ${packaged ? "packaged" : "source-HTML"} quick start discovered a real Model Host model, registered it through the browser, listed it through MCP, and completed browser coding chat.`);
} finally {
  try { if (browser) await browser.close(); }
  finally { if (created) execFileSync("docker", ["rm", "-f", name], { stdio: "ignore" }); }
}
