// Exercise browser MCP through a disposable TLS proxy with its real Host header.
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import https from "node:https";
import http from "node:http";
import os from "node:os";
import path from "node:path";
import { chromium } from "@playwright/test";

const root = process.cwd();
const plan = JSON.parse(execFileSync("bash", ["scripts/env", "worktree", "plan"], { encoding: "utf8" }));
const envFile = path.join(plan.state, "environment.env");
const values = Object.fromEntries(readFileSync(envFile, "utf8").split("\n").filter((line) => line.includes("=")).map((line) => {
  const index = line.indexOf("="); return [line.slice(0, index), line.slice(index + 1)];
}));
const temporary = mkdtempSync(path.join(os.tmpdir(), "aruvi-tls-proxy-"));
const suffix = String(process.pid), container = `${plan.project}-tls-mcp-${suffix}`;
const socket = `${plan.project}_tls_mcp_socket_${suffix}`, pdf = `${socket}_pdf`;
const env = { ...process.env, APP_IMAGE: values.APP_IMAGE, PRODUCT_SOURCE: root, TEST_SOCKET_NAME: socket, TEST_PDF_SOCKET_NAME: pdf, COMPOSE_IGNORE_ORPHANS: "true" };
const compose = ["compose", "--project-name", plan.project, "--project-directory", root, "--env-file", envFile, "-f", "deploy/compose.test.yaml"];
let browser, proxy, created = false;
let volumesOwned = false;
try {
  for (const [kind, name] of [["container", container], ["volume", socket], ["volume", pdf]]) {
    try { execFileSync("docker", [kind, "inspect", name], { stdio: "ignore" }); throw new Error(`Fixture ${kind} already exists: ${name}`); }
    catch (error) { if (!Number.isInteger(error.status)) throw error; }
  }
  volumesOwned = true;
  execFileSync("openssl", ["req", "-x509", "-newkey", "rsa:2048", "-nodes", "-keyout", `${temporary}/key.pem`, "-out", `${temporary}/cert.pem`, "-days", "1", "-subj", "/CN=studio.test", "-addext", "subjectAltName=DNS:studio.test"], { stdio: "ignore" });
  let target;
  proxy = https.createServer({ key: readFileSync(`${temporary}/key.pem`), cert: readFileSync(`${temporary}/cert.pem`) }, (incoming, outgoing) => {
    const upstream = http.request({ hostname: "127.0.0.1", port: target, path: incoming.url, method: incoming.method, headers: incoming.headers }, (response) => {
      outgoing.writeHead(response.statusCode, response.headers); response.pipe(outgoing);
    });
    upstream.on("error", () => { if (!outgoing.headersSent) outgoing.writeHead(502); outgoing.end(); });
    incoming.pipe(upstream);
  });
  await new Promise((resolve) => proxy.listen(0, "127.0.0.1", resolve));
  const origin = `https://studio.test:${proxy.address().port}`;
  execFileSync("docker", [...compose, "run", "--rm", "-d", "--name", container, "-p", "127.0.0.1::8787", "-e", `ARUVI_MCP_ALLOWED_ORIGINS=${origin}`, "fixture"], { env, stdio: "ignore" });
  created = true;
  const address = execFileSync("docker", ["port", container, "8787/tcp"], { encoding: "utf8" }).trim();
  assert.match(address, /^127\.0\.0\.1:\d+$/);
  target = Number(address.split(":").pop());
  let ready = false;
  for (let attempt = 0; attempt < 100; attempt++) {
    try { if ((await fetch(`http://127.0.0.1:${target}/health`)).ok) { ready = true; break; } } catch {}
    await new Promise((resolve) => setTimeout(resolve, 200));
  }
  assert.ok(ready, "disposable Studio fixture did not become healthy");
  browser = await chromium.launch({ executablePath: process.env.ARUVI_CHROME_PATH || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome", headless: true, args: ["--host-resolver-rules=MAP studio.test 127.0.0.1, MAP evil.test 127.0.0.1", "--no-proxy-server"] });
  const context = await browser.newContext({ ignoreHTTPSErrors: true });
  const page = await context.newPage();
  await page.goto(`${origin}/remote`);
  const result = await page.evaluate(async () => {
    const send = async (token) => (await fetch("/api/mcp", { method: "POST", headers: { "Content-Type": "application/json", ...(token ? { Authorization: `Bearer ${token}` } : {}) }, body: JSON.stringify({ jsonrpc: "2.0", id: 1, method: "ping" }) })).status;
    return [await send("workflow-mcp-test"), await send(null)];
  });
  assert.deepEqual(result, [200, 401]);
  const untrusted = await context.newPage();
  await untrusted.goto(`https://evil.test:${proxy.address().port}/remote`);
  const denied = await untrusted.evaluate(async () => (await fetch("/api/mcp", { method: "POST", headers: { "Content-Type": "application/json", Authorization: "Bearer workflow-mcp-test" }, body: JSON.stringify({ jsonrpc: "2.0", id: 2, method: "ping" }) })).status);
  assert.equal(denied, 403);
  console.log("Browser MCP succeeded through a real TLS proxy; missing token and untrusted origin were rejected.");
} finally {
  if (browser) await browser.close();
  if (proxy) await new Promise((resolve) => proxy.close(resolve));
  if (created) { try { execFileSync("docker", ["rm", "-f", container], { stdio: "ignore" }); } catch {} }
  if (volumesOwned) { try { execFileSync("docker", ["volume", "rm", socket, pdf], { stdio: "ignore" }); } catch {} }
  rmSync(temporary, { recursive: true, force: true });
}
