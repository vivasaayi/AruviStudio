// Run a disposable native Studio and synthetic provider to check full SSE messages.
import { chromium } from "@playwright/test";
import { spawn } from "node:child_process";
import http from "node:http";
import net from "node:net";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

const binary = process.argv[2];
if (!binary) throw new Error("Supply the native aruvi-studio-server binary path");
const directory = await mkdtemp(join(tmpdir(), "aruvi-sse-full-"));
const fixture = http.createServer((request, response) => {
  if (request.method !== "POST" || request.url !== "/v1/chat/completions") { response.writeHead(404).end(); return; }
  const chunks = [];
  request.on("data", (chunk) => chunks.push(chunk));
  request.on("end", () => {
    const body = JSON.parse(Buffer.concat(chunks).toString());
    const last = body.messages.at(-1).content;
    response.writeHead(200, { "Content-Type": "text/event-stream" });
    response.write('data: {"choices":[{"delta":{"content":"First "}}]}\n\n');
    if (last.includes("conflict")) response.end('data: {"choices":[{"message":{"content":"different"}}]}\n\ndata: [DONE]\n\n');
    else response.end('data: {"choices":[{"delta":{"content":"second"},"message":{"content":"First second"}}]}\n\ndata: [DONE]\n\n');
  });
});
await new Promise((resolve) => fixture.listen(0, "127.0.0.1", resolve));
const fixturePort = fixture.address().port;
const reserved = net.createServer();
await new Promise((resolve) => reserved.listen(0, "127.0.0.1", resolve));
const port = reserved.address().port;
await new Promise((resolve) => reserved.close(resolve));
const env = Object.fromEntries(Object.entries(process.env).filter(([key]) => !key.startsWith("ARUVI_")));
Object.assign(env, { ARUVI_APP_DATA_DIR: directory, ARUVI_DB_PATH: join(directory, "studio.db"), ARUVI_LLM_CONFIG_PATH: join(directory, "llm.json"),
  ARUVI_WEBHOOK_HOST: "127.0.0.1", ARUVI_WEBHOOK_PORT: String(port), ARUVI_MOBILE_API_TOKEN: "fixture-mobile", ARUVI_MCP_API_TOKEN: "fixture-mcp" });
const child = spawn(binary, [], { env, stdio: "ignore" });
let browser;
try {
  const base = `http://127.0.0.1:${port}`;
  let ready = false;
  for (let i = 0; i < 100; i++) {
    if (child.exitCode !== null) throw new Error("Native Studio exited before readiness");
    try { if ((await fetch(base + "/health")).ok) { ready = true; break; } } catch {}
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  if (!ready) throw new Error("Native Studio did not become ready");
  const created = await fetch(base + "/api/mobile/providers", { method: "POST", headers: { Authorization: "Bearer fixture-mobile", "Content-Type": "application/json" },
    body: JSON.stringify({ name: "SSE fixture", base_url: `http://127.0.0.1:${fixturePort}/v1` }) });
  if (!created.ok) throw new Error("Unable to create fixture provider");
  const provider = await created.json();
  browser = await chromium.launch({ executablePath: process.env.ARUVI_CHROME_PATH || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome", headless: true });
  const page = await browser.newPage();
  await page.goto(base + "/remote");
  await page.locator("#tokenInput").fill("fixture-mobile");
  await page.locator("#providerInput").fill(provider.id);
  await page.locator("#modelInput").fill("fixture");
  await page.locator("#saveButton").click();
  await page.getByRole("button", { name: "Chat", exact: true }).click();
  await page.getByLabel("Stream response").check();
  const send = async (text) => { await page.locator("#chatComposer").fill(text); await page.locator("#chatSendButton").click(); };
  await send("matching full message");
  await page.waitForFunction(() => document.querySelector("#chatConversation .assistant:last-child")?.textContent === "First second");
  await send("conflict");
  await page.waitForFunction(() => document.getElementById("status").textContent.includes("full message conflicts"));
  console.log("Disposable native Studio browser accepted matching SSE messages once and rejected conflicting messages.");
} finally {
  if (browser) await browser.close();
  if (child.exitCode === null) { child.kill("SIGTERM"); await new Promise((resolve) => child.once("exit", resolve)); }
  await new Promise((resolve) => fixture.close(resolve));
  await rm(directory, { recursive: true, force: true });
}
