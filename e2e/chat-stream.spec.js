import { test, expect } from "./support/test.js";

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    let nextId = 1;
    const callbacks = new Map();
    const listeners = new Map();
    window.__TAURI_INTERNALS__ = {
      transformCallback(callback) { const id = nextId++; callbacks.set(id, callback); return id; },
      async invoke(command, args) {
        if (command === "plugin:event|listen") { listeners.set(args.handler, { event: args.event, handler: callbacks.get(args.handler) }); return args.handler; }
        if (command === "plugin:event|unlisten") { listeners.delete(args.eventId); return; }
        throw new Error(`Unexpected Tauri command: ${command}`);
      },
    };
    window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener(_event, id) { listeners.delete(id); } };
    window.fixtureEmitStream = (event, payload) => {
      for (const [id, entry] of listeners) if (entry.event === event) entry.handler({ event, id, payload });
    };
    const bridge = window.__ARUVI_E2E__;
    const original = bridge.invoke;
    bridge.invoke = async (command, args) => {
      const result = await original(command, args);
      if (command === "cancel_model_chat_stream") {
        queueMicrotask(() => window.fixtureEmitStream("chat_stream_error", { stream_id: args.streamId, error: "Stream cancelled by user" }));
        return true;
      }
      return result;
    };
  });
  await page.goto("/chat");
});

for (const outcome of ["provider error", "user stop"]) {
  test(`${outcome} marks partial output incomplete and excludes the failed exchange from history`, async ({ page }) => {
    await page.getByTestId("chat-input").fill("first question");
    await page.getByTestId("chat-send").click();
    await expect(page.getByTestId("chat-stop")).toBeEnabled();
    const firstId = await page.evaluate(() => window.__ARUVI_E2E__.getInvokeCalls("start_model_chat_stream")[0].args.request.stream_id);
    await page.evaluate((id) => window.fixtureEmitStream("chat_stream_chunk", { stream_id: id, delta: "partial answer" }), firstId);
    await expect(page.getByText("partial answer")).toBeVisible();
    if (outcome === "user stop") await page.getByTestId("chat-stop").click();
    else await page.evaluate((id) => window.fixtureEmitStream("chat_stream_error", { stream_id: id, error: "Provider truncated response" }), firstId);
    await expect(page.getByText(/Response incomplete/)).toBeVisible();
    await expect(page.getByTestId("chat-send")).toBeEnabled();
    if (outcome === "user stop") expect(await page.evaluate(() => window.__ARUVI_E2E__.getInvokeCalls("cancel_model_chat_stream").length)).toBe(1);
    await page.getByTestId("chat-input").fill("second question");
    await page.getByTestId("chat-send").click();
    await expect.poll(() => page.evaluate(() => window.__ARUVI_E2E__.getInvokeCalls("start_model_chat_stream").length)).toBe(2);
    const messages = await page.evaluate(() => window.__ARUVI_E2E__.getInvokeCalls("start_model_chat_stream")[1].args.request.messages);
    expect(messages.some((message) => message.content === "first question" || message.content === "partial answer")).toBe(false);
    expect(messages.some((message) => message.content === "second question")).toBe(true);
  });
}

test("completed exchanges remain in later model history", async ({ page }) => {
  await page.getByTestId("chat-input").fill("completed question");
  await page.getByTestId("chat-send").click();
  await expect(page.getByTestId("chat-stop")).toBeEnabled();
  const id = await page.evaluate(() => window.__ARUVI_E2E__.getInvokeCalls("start_model_chat_stream")[0].args.request.stream_id);
  await page.evaluate((streamId) => {
    window.fixtureEmitStream("chat_stream_chunk", { stream_id: streamId, delta: "complete answer" });
    window.fixtureEmitStream("chat_stream_done", { stream_id: streamId });
  }, id);
  await expect(page.getByTestId("chat-send")).toBeEnabled();
  await page.getByTestId("chat-input").fill("follow up");
  await page.getByTestId("chat-send").click();
  await expect.poll(() => page.evaluate(() => window.__ARUVI_E2E__.getInvokeCalls("start_model_chat_stream").length)).toBe(2);
  const messages = await page.evaluate(() => window.__ARUVI_E2E__.getInvokeCalls("start_model_chat_stream")[1].args.request.messages);
  expect(messages.map((message) => message.content)).toEqual(expect.arrayContaining(["completed question", "complete answer", "follow up"]));
});

test("timeout cancels the request and marks its partial answer incomplete", async ({ page }) => {
  await page.evaluate(() => {
    const original = window.setTimeout.bind(window);
    window.setTimeout = (callback, delay, ...args) => original(callback, delay === 120000 ? 300 : delay, ...args);
  });
  await page.getByTestId("chat-input").fill("slow question");
  await page.getByTestId("chat-send").click();
  await expect(page.getByTestId("chat-stop")).toBeEnabled();
  const id = await page.evaluate(() => window.__ARUVI_E2E__.getInvokeCalls("start_model_chat_stream")[0].args.request.stream_id);
  await page.evaluate((streamId) => window.fixtureEmitStream("chat_stream_chunk", { stream_id: streamId, delta: "partial answer" }), id);
  await expect(page.getByText(/Response incomplete/)).toBeVisible();
  await expect(page.getByText("Model stream timed out.")).toBeVisible();
  await expect.poll(() => page.evaluate(() => window.__ARUVI_E2E__.getInvokeCalls("cancel_model_chat_stream").length)).toBe(1);
});

test("leaving chat cancels its active stream", async ({ page }) => {
  await page.getByTestId("chat-input").fill("question");
  await page.getByTestId("chat-send").click();
  await expect(page.getByTestId("chat-stop")).toBeEnabled();
  await page.getByTestId("nav-products").click();
  await expect(page.getByRole("heading", { name: "Products" })).toBeVisible();
  await expect.poll(() => page.evaluate(() => window.__ARUVI_E2E__.getInvokeCalls("cancel_model_chat_stream").length)).toBe(1);
});

test("leaving during asynchronous stream startup cancels after registration", async ({ page }) => {
  await page.evaluate(() => {
    const bridge = window.__ARUVI_E2E__;
    const original = bridge.invoke;
    bridge.invoke = (command, args) => {
      if (command !== "start_model_chat_stream") return original(command, args);
      const started = original(command, args);
      return new Promise((resolve) => { window.fixtureFinishStart = async () => resolve(await started); });
    };
  });
  await page.getByTestId("chat-input").fill("question");
  await page.getByTestId("chat-send").click();
  await expect.poll(() => page.evaluate(() => window.__ARUVI_E2E__.getInvokeCalls("start_model_chat_stream").length)).toBe(1);
  await page.getByTestId("nav-products").click();
  await expect(page.getByRole("heading", { name: "Products" })).toBeVisible();
  await page.evaluate(() => window.fixtureFinishStart());
  await expect.poll(() => page.evaluate(() => window.__ARUVI_E2E__.getInvokeCalls("cancel_model_chat_stream").length)).toBe(2);
});
