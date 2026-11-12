// Check browser-origin MCP through the actual published worktree port.
import assert from "node:assert/strict";
import { chromium } from "@playwright/test";
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { request as httpRequest } from "node:http";

const plan = JSON.parse(execFileSync("bash", ["scripts/env", "worktree", "plan"], { encoding: "utf8" }));
const envFile = `${plan.state}/environment.env`;
const line = readFileSync(envFile, "utf8").split("\n").find((value) => value.startsWith("ARUVI_MCP_API_TOKEN="));
if (!line) throw new Error("MCP token is not configured");
const token = line.slice("ARUVI_MCP_API_TOKEN=".length);
const address = execFileSync("docker", ["compose", "--project-name", plan.project, "--project-directory", process.cwd(), "--env-file", envFile, "-f", "deploy/compose.worktree.yaml", "port", "app", "8787"], { encoding: "utf8" }).trim();
const base = `http://${address}`;
const body = JSON.stringify({ jsonrpc: "2.0", id: 1, method: "ping" });
const browser = await chromium.launch({ executablePath: process.env.ARUVI_CHROME_PATH || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome", headless: true });
try {
  const page = await browser.newPage();
  await page.goto(`${base}/remote`);
  const response = await page.evaluate(async ({ token, body }) => {
    const result = await fetch("/api/mcp", { method: "POST", headers: { "Content-Type": "application/json", Authorization: `Bearer ${token}` }, body });
    return { status: result.status, data: await result.json() };
  }, { token, body });
  assert.equal(response.status, 200);
  assert.deepEqual(response.data.result, {});
  const unauthorized = await fetch(`${base}/api/mcp`, { method: "POST", headers: { "Content-Type": "application/json", Origin: base }, body });
  assert.equal(unauthorized.status, 401);
  const crossOrigin = await fetch(`${base}/api/mcp`, { method: "POST", headers: { "Content-Type": "application/json", Authorization: `Bearer ${token}`, Origin: "http://127.0.0.1:1" }, body });
  assert.equal(crossOrigin.status, 403);
  const nullOrigin = await fetch(`${base}/api/mcp`, { method: "POST", headers: { "Content-Type": "application/json", Authorization: `Bearer ${token}`, Origin: "null" }, body });
  assert.equal(nullOrigin.status, 403);
  const untrustedHost = await fetch(`${base}/api/mcp`, { method: "POST", headers: { "Content-Type": "application/json", Authorization: `Bearer ${token}`, Host: "untrusted.example:55010", Origin: "http://untrusted.example:55010" }, body });
  assert.equal(untrustedHost.status, 403);
  const duplicateOriginStatus = await new Promise((resolve, reject) => {
    const req = httpRequest(`${base}/api/mcp`, { method: "POST", headers: { "Content-Type": "application/json", Authorization: `Bearer ${token}`, Origin: [base, base] } }, (res) => { res.resume(); res.on("end", () => resolve(res.statusCode)); });
    req.on("error", reject);
    req.end(body);
  });
  assert.ok([400, 403].includes(duplicateOriginStatus), `Duplicate Origin returned ${duplicateOriginStatus}`);
  console.log("Published-port browser MCP works; missing tokens and cross-origin requests are rejected.");
} finally { await browser.close(); }
