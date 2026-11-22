import { test, expect } from "./support/test.js";

async function setup(page, failure) {
  await page.addInitScript((mode) => {
    if (mode === "preflight" || mode === "transcription") window.__ARUVI_E2E__.invoke("set_setting", { key: mode === "preflight" ? "e2e.voice_preflight_fail" : "e2e.voice_transcription_fail", value: "true" });
    if (mode === "pending-transcription") window.__ARUVI_E2E__.invoke("set_setting", { key: "e2e.voice_transcription_delay", value: "true" });
    const originalTimeout = window.setTimeout.bind(window);
    window.setTimeout = (callback, delay, ...args) => originalTimeout(callback, delay === 18000 ? 400 : delay, ...args);
    window.fixtureMicCalls = 0; window.fixtureMicStops = 0; window.fixtureContextCloses = 0;
    Object.defineProperty(navigator, "mediaDevices", { configurable: true, value: {
      getUserMedia: async () => {
        window.fixtureMicCalls += 1;
        const stream = { getTracks: () => [{ stop() { window.fixtureMicStops += 1; } }] };
        if (mode === "pending-mic" && window.fixtureMicCalls === 1) return new Promise((resolve) => { window.fixtureResolveMic = () => resolve(stream); });
        return stream;
      },
    } });
    window.AudioContext = class {
      constructor() { this.sampleRate = 16000; this.destination = {}; }
      resume() {
        if (mode === "pending-resume") return new Promise((resolve) => { window.fixtureResolveResume = resolve; });
        return Promise.resolve();
      }
      async close() { window.fixtureContextCloses += 1; }
      createMediaStreamSource() { return { connect() {}, disconnect() {} }; }
      createGain() { return { gain: { value: 1 }, connect() {}, disconnect() {} }; }
      createScriptProcessor() { return { connect() { originalTimeout(() => this.onaudioprocess({ inputBuffer: { getChannelData: () => new Float32Array([0.5, -0.5]) } }), 20); }, disconnect() {} }; }
    };
  }, failure);
  await page.goto("/voice-chat");
  await expect(page.getByTestId("voice-chat-model-picker")).not.toHaveValue("");
}

test("voice chat uses server transcription defaults without saved frontend settings", async ({ page }) => {
  await setup(page);
  await page.getByTestId("voice-chat-start").click();
  await expect(page.getByText("server-transcriber (resolved at start)")).toBeVisible();
  await expect.poll(() => page.evaluate(() => window.__ARUVI_E2E__.getInvokeCalls("transcribe_audio_command").length)).toBe(1);
  const args = await page.evaluate(() => window.__ARUVI_E2E__.getInvokeCalls("transcribe_audio_command")[0].args);
  expect(args.providerId).toBeNull(); expect(args.modelName).toBeNull(); expect(args.locale).toBeNull(); expect(args.fallbackLocale).toBe("en-US");
  await page.getByTestId("voice-chat-stop").click();
});

test("voice chat checks configuration before opening the microphone", async ({ page }) => {
  await setup(page, "preflight");
  await page.getByTestId("voice-chat-start").click();
  await expect(page.getByText(/speech transcription provider is required/)).toBeVisible();
  expect(await page.evaluate(() => window.fixtureMicCalls)).toBe(0);
  expect(await page.evaluate(() => window.__ARUVI_E2E__.getInvokeCalls("transcribe_audio_command").length)).toBe(0);
});

test("ending voice chat discards a pending microphone capture", async ({ page }) => {
  await setup(page);
  await page.getByTestId("voice-chat-start").click();
  await expect.poll(() => page.evaluate(() => window.fixtureMicCalls)).toBe(1);
  await page.getByTestId("voice-chat-stop").click();
  await expect.poll(() => page.evaluate(() => window.fixtureMicStops)).toBe(1);
  await page.waitForTimeout(600);
  expect(await page.evaluate(() => window.__ARUVI_E2E__.getInvokeCalls("transcribe_audio_command").length)).toBe(0);
});

test("voice chat stops after a transcription error instead of reopening the microphone", async ({ page }) => {
  await setup(page, "transcription");
  await page.getByTestId("voice-chat-start").click();
  await expect(page.getByText(/Synthetic transcription failure/)).toBeVisible();
  await expect(page.getByTestId("voice-chat-start")).toBeVisible();
  await page.waitForTimeout(800);
  expect(await page.evaluate(() => window.fixtureMicCalls)).toBe(1);
  expect(await page.evaluate(() => window.__ARUVI_E2E__.getInvokeCalls("transcribe_audio_command").length)).toBe(1);
});

test("a delayed microphone permission from a stopped session cannot join a restarted session", async ({ page }) => {
  await setup(page, "pending-mic");
  await page.getByTestId("voice-chat-start").click();
  await expect.poll(() => page.evaluate(() => window.fixtureMicCalls)).toBe(1);
  await page.getByTestId("voice-chat-stop").click();
  await page.getByTestId("voice-chat-start").click();
  await expect.poll(() => page.evaluate(() => window.fixtureMicCalls)).toBe(2);
  await page.evaluate(() => window.fixtureResolveMic());
  await expect.poll(() => page.evaluate(() => window.fixtureMicStops)).toBeGreaterThanOrEqual(1);
  await expect.poll(() => page.evaluate(() => window.__ARUVI_E2E__.getInvokeCalls("transcribe_audio_command").length)).toBe(1);
  await page.getByTestId("voice-chat-stop").click();
});

test("stopping voice chat releases the microphone while audio resume is pending", async ({ page }) => {
  await setup(page, "pending-resume");
  await page.getByTestId("voice-chat-start").click();
  await expect.poll(() => page.evaluate(() => typeof window.fixtureResolveResume)).toBe("function");
  await page.getByTestId("voice-chat-stop").click();
  await expect.poll(() => page.evaluate(() => window.fixtureMicStops)).toBe(1);
  await expect.poll(() => page.evaluate(() => window.fixtureContextCloses)).toBe(1);
  await page.evaluate(() => window.fixtureResolveResume());
  await page.waitForTimeout(300);
  expect(await page.evaluate(() => window.__ARUVI_E2E__.getInvokeCalls("transcribe_audio_command").length)).toBe(0);
});

test("stopping during transcription does not send a late transcript to the chat model", async ({ page }) => {
  await setup(page, "pending-transcription");
  await page.getByTestId("voice-chat-start").click();
  await expect.poll(() => page.evaluate(() => typeof window.fixtureResolveTranscription)).toBe("function");
  await page.getByTestId("voice-chat-stop").click();
  await page.evaluate(() => window.fixtureResolveTranscription());
  await page.waitForTimeout(300);
  expect(await page.evaluate(() => window.__ARUVI_E2E__.getInvokeCalls("start_model_chat_stream").length)).toBe(0);
});
