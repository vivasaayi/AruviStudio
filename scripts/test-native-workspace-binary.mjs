// Disposable native server and browser: verify saved binary workspace downloads.
import assert from "node:assert/strict";
import { mkdtemp, mkdir, readFile, realpath, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { execFileSync, spawn } from "node:child_process";
import net from "node:net";
import { chromium } from "@playwright/test";

const root = await mkdtemp(join(tmpdir(), "aruvi-binary-workspace-"));
let server, browser;
try {
  const workspace = join(root, "workspace"); await mkdir(workspace); execFileSync("git", ["init", "-q", workspace]);
  const actualRoot = await realpath(workspace), bytes = Buffer.from([0, 255, 17, 128, 1]);
  await writeFile(join(actualRoot, "fixture.bin"), bytes);
  const port = await new Promise((done) => { const socket = net.createServer(); socket.listen(0, "127.0.0.1", () => { const result = socket.address().port; socket.close(() => done(result)); }); });
  const token = "binary-workspace-test", base = `http://127.0.0.1:${port}`;
  server = spawn(resolve("src-tauri/target/debug/aruvi-studio-server"), [], { stdio:"ignore", env:{ ...process.env, ARUVI_APP_DATA_DIR:root, ARUVI_DB_PATH:join(root,"db.sqlite"), ARUVI_WEBHOOK_HOST:"127.0.0.1", ARUVI_WEBHOOK_PORT:String(port), ARUVI_MOBILE_API_TOKEN:token, ARUVI_MCP_API_TOKEN:token } });
  for (let i = 0;; i++) { if (server.exitCode !== null || i === 120) throw new Error("Native fixture did not start"); try { if ((await fetch(base + "/health")).ok) break; } catch {} await new Promise((done) => setTimeout(done, 250)); }
  const headers = { "Content-Type":"application/json", Authorization:`Bearer ${token}` };
  async function call(action, args) {
    const response = await fetch(base + "/api/mcp", { method:"POST", headers, body:JSON.stringify({jsonrpc:"2.0",id:1,method:"tools/call",params:{name:"aruvi_repositories",arguments:{action,arguments:args}}}) });
    const result = (await response.json()).result; assert(!result.isError, JSON.stringify(result)); return result.structuredContent.result;
  }
  const repo = await call("register_repository", { name:"Binary fixture", local_path:actualRoot, default_branch:"main" });
  const args = { repository_id:repo.id, relative_path:"fixture.bin", expected_root:actualRoot };
  assert.deepEqual(Buffer.from((await call("read_repository_binary", args)).content_base64, "base64"), bytes);
  const boundary = Buffer.alloc(8 * 1024 * 1024, 0xa5); await writeFile(join(actualRoot, "boundary.bin"), boundary);
  const bounded = await call("read_repository_binary", {...args, relative_path:"boundary.bin"});
  assert.equal(bounded.size, boundary.length); assert.deepEqual(Buffer.from(bounded.content_base64, "base64"), boundary);
  await writeFile(join(actualRoot, "too-large.bin"), Buffer.alloc(boundary.length + 1));
  const tooLarge = await fetch(base + "/api/mobile/repository-inspect", { method:"POST", headers, body:JSON.stringify({action:"read_repository_binary",...args,relative_path:"too-large.bin"}) });
  assert.equal(tooLarge.status, 400);
  const list = await fetch(base + "/api/mcp", { method:"POST", headers, body:JSON.stringify({jsonrpc:"2.0",id:2,method:"tools/list"}) });
  assert((await list.json()).result.tools.some((tool) => tool.name === "repositories.files.read_binary"));
  const firstClass = await fetch(base + "/api/mcp", { method:"POST", headers, body:JSON.stringify({jsonrpc:"2.0",id:3,method:"tools/call",params:{name:"repositories.files.read_binary",arguments:{repositoryId:repo.id,relativePath:"fixture.bin",expectedRoot:actualRoot}}}) });
  assert.deepEqual(Buffer.from((await firstClass.json()).result.structuredContent.result.content_base64, "base64"), bytes);
  const body = JSON.stringify({ action:"read_repository_binary", ...args });
  assert.equal((await fetch(base + "/api/mobile/repository-inspect", { method:"POST", headers:{"Content-Type":"application/json"}, body })).status, 401);
  assert.equal((await fetch(base + "/api/mobile/repository-inspect", { method:"POST", headers, body:JSON.stringify({action:"read_repository_binary",...args,expected_root:"/stale"}) })).status, 409);
  browser = await chromium.launch({ executablePath:process.env.ARUVI_CHROME_PATH || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome", headless:true });
  const page = await browser.newPage({ acceptDownloads:true }); await page.goto(base + "/remote");
  await page.locator("#tokenInput").fill(token); await page.locator("#saveButton").click();
  await page.getByRole("button", { name:"Workspaces", exact:true }).click();
  await page.locator("#repositoryList .tool-card").filter({hasText:"Binary fixture"}).getByRole("button", {name:"Browse workspace"}).click();
  await page.getByRole("button", {name:"Download saved fixture.bin"}).waitFor();
  await page.getByLabel("Filter workspace files").fill("boundary.bin");
  assert.equal(await page.getByRole("button", {name:"Download saved fixture.bin", includeHidden:true}).isVisible(), false);
  assert.equal(await page.getByRole("button", {name:"Download saved boundary.bin"}).isVisible(), true);
  await page.getByLabel("Filter workspace files").fill("");
  const event = page.waitForEvent("download"); await page.getByRole("button", {name:"Download saved fixture.bin"}).click();
  const download = await event; assert.equal(download.suggestedFilename(), "fixture.bin");
  assert.deepEqual(await readFile(await download.path()), bytes);
  console.log("Binary download passed browser, MCP, stale-root and authorization checks.");
} finally {
  if (browser) await browser.close();
  if (server && server.exitCode === null && server.signalCode === null) { server.kill(); await new Promise((done) => server.once("exit", done)); }
  await rm(root, { recursive:true, force:true });
}
