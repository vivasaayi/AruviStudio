import { test, expect } from "@playwright/test";
import { readFile } from "node:fs/promises";
import { createHash } from "node:crypto";

test("browser and MCP inspect a configured CI socket", async ({ page, request }) => {
  if (process.env.ARUVI_TEST_REMOTE_HTML_SOURCE === "1") {
    const html = await readFile("src-tauri/src/services/remote_app.html", "utf8");
    await page.route("**/remote", (route) => route.fulfill({ contentType: "text/html", body: html }));
  }
  const token = process.env.ARUVI_MOBILE_API_TOKEN;
  const headers = { Authorization: `Bearer ${token}` };
  async function post(path, data) { const response = await request.post(path, { headers, data }); expect(response.ok(), await response.text()).toBeTruthy(); return response.json(); }
  await expect.poll(async () => (await request.get("/api/mobile/ci/status", { headers })).status(), { timeout: 15000 }).toBe(200);
  expect((await request.get("/api/mobile/ci/targets")).status()).toBe(401);
  expect((await request.get("/api/mobile/ci/artifacts/88/readiness")).status()).toBe(401);
  expect((await request.get("/api/mobile/ci/artifacts/88/data/fast-chunk?offset=0")).status()).toBe(401);
  expect((await request.post("/api/mobile/ci/runs/44/feedback", { data: { verdict: "works" } })).status()).toBe(401);
  const product = await post("/api/mobile/products", { name: "CI fixture product" });
  const repository = await post("/api/mobile/repositories", { name: "CI fixture workspace", local_path: "/repositories/ci-fixture" });
  const selection = { product_id: product.id, repository_id: repository.id, target_id: "fixture-preview" };
  expect((await request.get(`/api/mobile/ci/repositories/${repository.id}/branches`)).status()).toBe(401);
  expect((await request.post("/api/mobile/ci/bindings/save", { headers, data: selection })).status()).toBe(400);
  await post("/api/mobile/repository-attachments", { product_id: product.id, repository_id: repository.id });
  expect((await request.post("/api/mobile/ci/bindings/queue", { headers, data: { ...selection, branch: "main" } })).status()).toBe(400);
  expect((await request.post("/api/mobile/ci/bindings/save", { headers, data: { ...selection, target_id: "mismatch-preview" } })).status()).toBe(400);
  await post("/api/mobile/ci/bindings/save", { ...selection, target_id: "unapproved-preview" });
  expect((await request.post("/api/mobile/ci/bindings/queue", { headers, data: { ...selection, target_id: "unapproved-preview", branch: "main" } })).status()).toBe(400);
  await page.addInitScript(() => {
    window.__ciArtifact = {bytes:[], closed:false, aborted:false};
    window.showSaveFilePicker = async () => ({createWritable: async () => ({
      write: async bytes => window.__ciArtifact.bytes.push(...bytes),
      close: async () => { window.__ciArtifact.closed = true; },
      abort: async () => { window.__ciArtifact.aborted = true; },
    })});
  });
  await page.goto("/remote");
  expect(await page.evaluate(() => window.isSecureContext)).toBe(true);
  await page.locator("#tokenInput").fill(token);
  await page.locator("#saveButton").click();
  await page.getByRole("button", { name: "CI", exact: true }).click();
  await expect(page.locator("#ciStatus")).toHaveText("Service: online");
  await expect(page.locator("#ciTargets")).toContainText("fixture-preview");
  await expect(page.locator("#ciRuns")).toContainText("failed");
  await page.locator("#ciProduct").selectOption(product.id);
  await page.locator("#ciRepository").selectOption(repository.id);
  await page.locator("#ciTarget").selectOption("mismatch-preview");
  let mismatchMessage = "";
  page.once("dialog", (dialog) => { mismatchMessage = dialog.message(); return dialog.accept(); });
  await page.locator("#ciAssign").click();
  await expect.poll(() => mismatchMessage).toMatch(/repository/i);
  await page.locator("#ciTarget").selectOption("fixture-preview");
  const assigned = page.waitForResponse((r) => r.url().endsWith("/api/mobile/ci/bindings/save"));
  await page.locator("#ciAssign").click();
  expect((await assigned).ok()).toBeTruthy();
  await expect(page.locator("#ciBindings")).toContainText("fixture-preview");
  await expect(page.locator("#ciRevision option[value='main']")).toHaveCount(1);
  await page.locator("#ciRevision").selectOption("main");
  const dialogs = [];
  const acceptQueueDialogs = (dialog) => { dialogs.push(dialog.message()); return dialog.accept(); };
  page.on("dialog", acceptQueueDialogs);
  const queued = page.waitForResponse((r) => r.url().endsWith("/api/mobile/ci/bindings/queue"));
  await page.locator("#ciQueue").click();
  const queueResult = await (await queued).json();
  expect(queueResult.commit).toMatch(/^[0-9a-f]{40}$/);
  await expect.poll(() => dialogs.length).toBe(2);
  expect(dialogs[1]).toContain("run #45");
  page.off("dialog", acceptQueueDialogs);
  await page.locator("#ciRevisionType").selectOption("commit");
  await expect(page.locator("#ciRevision option")).toHaveCount(2);
  await page.locator("#ciRevision").selectOption(queueResult.commit);
  await expect(page.locator("#ciBindings")).toContainText("CI fixture product");
  await page.getByRole("button", { name: /Run #42/ }).click();
  await page.getByRole("button", { name: /Verify artifact #88/ }).click();
  await expect(page.locator("#ciRunDetails")).toContainText("fixture verified");
  const downloadEvent = page.waitForEvent("download");
  await page.getByRole("button", {name:"Download artifact #88"}).click();
  const download = await downloadEvent;
  const expected = Buffer.alloc(70001, 42);
  expect(download.suggestedFilename()).toBe("package.bin");
  expect(await readFile(await download.path())).toEqual(expected);
  await page.getByRole("button", {name:"Save large artifact #88"}).click();
  await expect.poll(() => page.evaluate(() => window.__ciArtifact.closed)).toBe(true);
  const saved = await page.evaluate(() => window.__ciArtifact);
  expect(saved.aborted).toBe(false);
  expect(createHash("sha256").update(Buffer.from(saved.bytes)).digest("hex")).toBe(createHash("sha256").update(expected).digest("hex"));
  await page.getByRole("button", { name: "Log: build" }).click();
  await expect(page.locator("#ciRunDetails")).toContainText("build output");
  page.once("dialog", (dialog) => dialog.accept());
  await page.getByRole("button", { name: "Retry build" }).click();
  await expect(page.locator("#ciRunDetails")).toContainText("Retry queued as run #44");
  await page.getByRole("button", { name: /Run #43/ }).click();
  await page.getByRole("button", { name: "Log: build" }).click();
  await expect(page.locator("#ciRunDetails")).toContainText("live two");
  page.once("dialog", (dialog) => dialog.accept());
  await page.getByRole("button", { name: "Cancel build" }).click();
  await expect(page.locator("#ciRunDetails")).toContainText("Cancellation requested");
  await page.getByRole("button", { name: /Run #44/ }).click();
  await page.getByLabel("CI feedback verdict").selectOption("needs_changes");
  await page.getByLabel("CI feedback notes").fill("Please revise fixture");
  await page.getByRole("button", { name: "Record feedback" }).click();
  await expect(page.locator("#ciRunDetails")).toContainText("Please revise fixture");
  expect((await request.post("/api/mobile/ci/runs/42/feedback", { headers, data: { verdict: "works", notes: "invalid" } })).status()).toBe(400);
  expect((await request.post("/api/mobile/ci/runs/43/cancel")).status()).toBe(401);
  const response = await request.post("/api/mcp", {
    headers: { Authorization: `Bearer ${process.env.ARUVI_MCP_API_TOKEN}`, Accept: "application/json, text/event-stream" },
    data: { jsonrpc: "2.0", id: 1, method: "tools/call", params: { name: "aruvi_ci", arguments: { action: "list_targets", arguments: {} } } },
  });
  const body = await response.json();
  expect(body.result.isError).not.toBeTruthy();
  expect(body.result.structuredContent.result[0].id).toBe("fixture-preview");
  const logResponse = await request.post("/api/mcp", {
    headers: { Authorization: `Bearer ${process.env.ARUVI_MCP_API_TOKEN}`, Accept: "application/json, text/event-stream" },
    data: { jsonrpc: "2.0", id: 2, method: "tools/call", params: { name: "aruvi_ci", arguments: { action: "get_logs", arguments: { id: 42, stage: "build", offset: 0 } } } },
  });
  expect((await logResponse.json()).result.structuredContent.result.text).toBe("build output\n");
  const artifactResponse = await request.post("/api/mcp", {
    headers: { Authorization: `Bearer ${process.env.ARUVI_MCP_API_TOKEN}`, Accept: "application/json, text/event-stream" },
    data: { jsonrpc: "2.0", id: 3, method: "tools/call", params: { name: "aruvi_ci", arguments: { action: "artifact_readiness", arguments: { id: 88 } } } },
  });
  expect((await artifactResponse.json()).result.structuredContent.result.manual_promotion_only).toBe(true);
  const feedbackResponse = await request.post("/api/mcp", {
    headers: { Authorization: `Bearer ${process.env.ARUVI_MCP_API_TOKEN}`, Accept: "application/json, text/event-stream" },
    data: { jsonrpc: "2.0", id: 4, method: "tools/call", params: { name: "aruvi_ci", arguments: { action: "list_feedback", arguments: { id: 44 } } } },
  });
  const feedback = (await feedbackResponse.json()).result.structuredContent.result;
  expect(feedback[0].commit_sha).toBe("a".repeat(40));
  expect(feedback[0].notes).toBe("Please revise fixture");
  expect((await request.get("/api/mobile/ci/runs/0", { headers })).status()).toBe(400);
  const binding = (await (await request.get("/api/mobile/ci/bindings", { headers })).json()).find((item) => item.target_id === "fixture-preview");
  expect(binding).toBeTruthy();
  await post("/api/mobile/ci/bindings/delete", { id: binding.id });
  expect((await request.post("/api/mobile/ci/bindings/queue", { headers, data: { ...selection, branch: "main" } })).status()).toBe(400);
});
