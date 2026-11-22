import { test, expect } from "./support/test.js";

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    window.pendingMicRequests = 0;
    window.pendingMicStops = 0;
    Object.defineProperty(navigator, "mediaDevices", { configurable: true, value: {
      getUserMedia: () => {
        window.pendingMicRequests++;
        return new Promise((resolve) => { window.grantPendingMic = () => resolve({ getTracks: () => [{ stop() { window.pendingMicStops++; } }] }); });
      },
    } });
  });
});

test("direct chat closes a microphone granted after navigation", async ({ page }) => {
  await page.goto("/chat");
  await page.getByTestId("chat-mic-toggle").click();
  await expect(page.getByTestId("chat-mic-toggle")).toHaveText("Cancel microphone request");
  await expect.poll(() => page.evaluate(() => window.pendingMicRequests)).toBe(1);
  await page.getByTestId("nav-products").click();
  await page.evaluate(() => window.grantPendingMic());
  await expect.poll(() => page.evaluate(() => window.pendingMicStops)).toBe(1);
  expect(await page.evaluate(() => window.__ARUVI_E2E__.getInvokeCalls("transcribe_audio_command").length)).toBe(0);
});

test("direct chat can cancel a pending microphone request", async ({ page }) => {
  await page.goto("/chat");
  await page.getByTestId("chat-mic-toggle").click();
  await expect(page.getByTestId("chat-mic-toggle")).toHaveText("Cancel microphone request");
  await page.getByTestId("chat-mic-toggle").click();
  await expect(page.getByTestId("chat-mic-toggle")).toHaveText("Start Listening");
  await page.evaluate(() => window.grantPendingMic());
  await expect.poll(() => page.evaluate(() => window.pendingMicStops)).toBe(1);
});

test("planner closes a microphone granted after navigation", async ({ page }) => {
  await page.goto("/");
  await page.getByLabel("Planner product").selectOption("example-product-calculator");
  await page.getByRole("button", { name: "Start Voice Input" }).click();
  await expect(page.getByRole("button", { name: "Stop Recording" })).toBeVisible();
  await expect.poll(() => page.evaluate(() => window.pendingMicRequests)).toBe(1);
  await page.getByTestId("nav-products").click();
  await page.evaluate(() => window.grantPendingMic());
  await expect.poll(() => page.evaluate(() => window.pendingMicStops)).toBe(1);
  expect(await page.evaluate(() => window.__ARUVI_E2E__.getInvokeCalls("transcribe_audio_command").length)).toBe(0);
});

test("planner does not submit a transcription that finishes after navigation", async ({ page }) => {
  await page.addInitScript(() => {
    window.__ARUVI_E2E__.invoke("set_setting", { key: "e2e.voice_transcription_delay", value: "true" });
    window.__ARUVI_E2E__.invoke("set_setting", { key: "speech.review_before_send", value: "false" });
    Object.defineProperty(navigator, "mediaDevices", { configurable: true, value: {
      getUserMedia: async () => ({ getTracks: () => [{ stop() {} }] }),
    } });
    window.AudioContext = class {
      constructor() { this.sampleRate = 16000; this.destination = {}; }
      async resume() {} async close() {}
      createMediaStreamSource() { return { connect() {}, disconnect() {} }; }
      createGain() { return { gain: { value: 1 }, connect() {}, disconnect() {} }; }
      createScriptProcessor() { return { connect() { setTimeout(() => this.onaudioprocess({ inputBuffer: { getChannelData: () => new Float32Array([0.5, -0.5]) } }), 20); }, disconnect() {} }; }
    };
  });
  await page.goto("/");
  await page.getByLabel("Planner product").selectOption("example-product-calculator");
  const record = async () => {
    await page.evaluate(() => { window.fixtureResolveTranscription = undefined; });
    await page.getByRole("button", { name: "Start Voice Input" }).click();
    await expect(page.getByRole("button", { name: "Stop Recording" })).toBeVisible();
    await page.waitForTimeout(60);
    await page.getByRole("button", { name: "Stop Recording" }).click();
    await expect.poll(() => page.evaluate(() => typeof window.fixtureResolveTranscription)).toBe("function");
  };
  await record();
  await page.evaluate(() => window.fixtureResolveTranscription());
  await expect.poll(() => page.evaluate(() => window.__ARUVI_E2E__.getInvokeCalls("submit_planner_voice_turn_command").length)).toBe(1);
  await expect(page.getByRole("button", { name: "Start Voice Input" })).toBeEnabled();
  await record();
  const before = await page.evaluate(() => ({
    sessions: window.__ARUVI_E2E__.getInvokeCalls("create_planner_session_command").length,
    turns: window.__ARUVI_E2E__.getInvokeCalls("submit_planner_voice_turn_command").length,
  }));
  await page.getByTestId("nav-products").click();
  await expect(page.getByRole("heading", { name: "Products" })).toBeVisible();
  await page.evaluate(() => window.fixtureResolveTranscription());
  await page.waitForTimeout(300);
  expect(await page.evaluate(() => ({
    sessions: window.__ARUVI_E2E__.getInvokeCalls("create_planner_session_command").length,
    turns: window.__ARUVI_E2E__.getInvokeCalls("submit_planner_voice_turn_command").length,
  }))).toEqual(before);
});
