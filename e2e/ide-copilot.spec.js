import { test, expect } from "./support/test.js";

test("desktop Copilot sends structured inputs to the Rust stream command", async ({ page }) => {
  await page.addInitScript(() => {
    let nextId = 1;
    const callbacks = new Map(), listeners = new Map();
    window.__TAURI_INTERNALS__ = {
      transformCallback(callback) { const id = nextId++; callbacks.set(id, callback); return id; },
      async invoke(command, args) {
        if (command === "plugin:event|listen") { listeners.set(args.handler, { event: args.event, handler: callbacks.get(args.handler) }); return args.handler; }
        if (command === "plugin:event|unlisten") { listeners.delete(args.eventId); return; }
        throw new Error(`Unexpected Tauri command: ${command}`);
      },
    };
    window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener(_event, id) { listeners.delete(id); } };
    window.fixtureEmitCopilot = (event, payload) => {
      for (const [id, entry] of listeners) if (entry.event === event) entry.handler({ event, id, payload });
    };
  });
  await page.goto("/ide");
  await page.getByRole("button", { name: "Propose Patch" }).click();
  await page.getByPlaceholder("Describe the exact change; Copilot will return a structured patch proposal...").fill("Create the fixture");
  await page.getByRole("button", { name: "Generate Proposal" }).click();
  await expect.poll(() => page.evaluate(() => window.__ARUVI_E2E__.getInvokeCalls("start_desktop_copilot_stream").length)).toBe(1);
  const request = await page.evaluate(() => window.__ARUVI_E2E__.getInvokeCalls("start_desktop_copilot_stream")[0].args.request);
  expect(request.patch_mode).toBe(true);
  expect(request.file).toBeNull();
  expect(request.history).toEqual([{ role: "user", content: "Create the fixture" }]);
  expect(request.system_prompt).toContain("Aruvi Copilot");
  await page.evaluate((id) => {
    window.fixtureEmitCopilot("chat_stream_chunk", { stream_id: id, delta: '{"type":"patch_proposal","summary":"Fixture","patches":[{"path":"a.txt","patch":"@@ -0,0 +1 @@\\n+hello"}]}' });
    window.fixtureEmitCopilot("chat_stream_done", { stream_id: id });
  }, request.stream_id);
  await expect(page.getByText("Error: Select a workspace before reviewing a patch proposal.")).toBeVisible();
});
