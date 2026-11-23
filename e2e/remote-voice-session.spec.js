import { test, expect } from "@playwright/test";
import { readFile } from "node:fs/promises";

const html = await readFile("src-tauri/src/services/remote_app.html", "utf8");

async function setup(page, { pendingMic = false, pendingResume = false, pendingTranscription = false, transcriptionFailure = false, turns = 1 } = {}) {
  const calls = [];
  let releaseTranscription;
  const transcriptionGate = new Promise((resolve) => { releaseTranscription = resolve; });
  await page.route("**/remote", (route) => route.fulfill({ status: 200, contentType: "text/html", body: html }));
  await page.route("**/api/mobile/**", async (route) => {
    const path = new URL(route.request().url()).pathname;
    calls.push(path);
    if (path.endsWith("/speech/transcribe")) {
      if (pendingTranscription) await transcriptionGate;
      await route.fulfill(transcriptionFailure ? { status: 400, json: { error: "Speech provider unavailable" } } : { json: { transcript: "hello from voice" } }).catch(() => {});
    } else if (path.endsWith("/chat/completions")) {
      await route.fulfill({ json: { content: "hello back" } });
    } else if (path.endsWith("/providers")) {
      await route.fulfill({ json: [] });
    } else if (path.endsWith("/speech-defaults")) {
      await route.fulfill({ json: { provider: "", model: "", locale: "" } });
    } else {
      await route.fulfill({ json: [] });
    }
  });
  await page.addInitScript(({ pendingMic, pendingResume, turns }) => {
    localStorage.setItem("aruvi.remote.active_tab", "voice");
    localStorage.setItem("aruvi.remote.token", "fixture-token");
    window.fixtureMicCalls = 0; window.fixtureStops = 0; window.fixtureSpoken = 0;
    Object.defineProperty(navigator, "mediaDevices", { configurable: true, value: { getUserMedia: async () => {
      window.fixtureMicCalls++;
      const stream = { getTracks: () => [{ stop() { window.fixtureStops++; } }] };
      if (pendingMic && window.fixtureMicCalls === 1) return new Promise((resolve) => { window.fixtureReleaseMic = () => resolve(stream); });
      return stream;
    } } });
    window.AudioContext = class {
      constructor() { this.sampleRate = 16000; this.destination = {}; this.state = pendingResume ? "suspended" : "running"; }
      async resume() { if (pendingResume) await new Promise((resolve) => { window.fixtureReleaseResume = resolve; }); }
      createMediaStreamSource() { return { connect() {}, disconnect() {} }; }
      createScriptProcessor() { return { connect() {
        if (window.fixtureMicCalls > turns) return;
        setTimeout(() => {
          for (const chunk of [new Float32Array(8000).fill(0.5), new Float32Array(18000)]) {
            this.onaudioprocess({ inputBuffer: { getChannelData: () => chunk }, outputBuffer: { getChannelData: () => new Float32Array(chunk.length) } });
          }
        }, 30);
      }, disconnect() {} }; }
      async close() {}
    };
    window.SpeechSynthesisUtterance = class { constructor(text) { this.text = text; } };
    Object.defineProperty(window, "speechSynthesis", { configurable: true, value: {
      cancel() {}, speak(utterance) { window.fixtureSpoken++; window.fixtureFinishSpeech = () => utterance.onend(); }
    } });
  }, { pendingMic, pendingResume, turns });
  await page.goto("/remote");
  await expect(page.getByRole("button", { name: "Start voice conversation" })).toBeVisible();
  return { calls, releaseTranscription };
}

test("conversation sends a turn and waits for speech before listening again", async ({ page }) => {
  const { calls } = await setup(page);
  await page.getByRole("button", { name: "Start voice conversation" }).click();
  await expect(page.locator("#voiceConversation")).toContainText("hello back");
  expect(calls.filter((path) => path.endsWith("/speech/transcribe"))).toHaveLength(1);
  expect(calls.filter((path) => path.endsWith("/chat/completions"))).toHaveLength(1);
  expect(await page.evaluate(() => window.fixtureMicCalls)).toBe(1);
  await page.evaluate(() => window.fixtureFinishSpeech());
  await expect.poll(() => page.evaluate(() => window.fixtureMicCalls)).toBe(2);
  await page.getByRole("button", { name: "End voice conversation" }).click();
  await expect(page.getByRole("button", { name: "Start voice conversation" })).toBeEnabled();
  expect(await page.evaluate(() => window.fixtureStops)).toBe(2);
});

test("stopping while microphone permission is pending releases the late stream", async ({ page }) => {
  const { calls } = await setup(page, { pendingMic: true });
  await page.getByRole("button", { name: "Start voice conversation" }).click();
  await expect.poll(() => page.evaluate(() => window.fixtureMicCalls)).toBe(1);
  await page.getByRole("button", { name: "End voice conversation" }).click();
  await expect(page.getByRole("button", { name: "Start voice conversation" })).toBeEnabled();
  await page.evaluate(() => window.fixtureReleaseMic());
  await expect.poll(() => page.evaluate(() => window.fixtureStops)).toBe(1);
  expect(calls.some((path) => path.endsWith("/speech/transcribe"))).toBe(false);
});

test("a late microphone stream cannot interfere with a restarted conversation", async ({ page }) => {
  await setup(page, { pendingMic: true });
  await page.getByRole("button", { name: "Start voice conversation" }).click();
  await expect.poll(() => page.evaluate(() => window.fixtureMicCalls)).toBe(1);
  await page.getByRole("button", { name: "End voice conversation" }).click();
  await page.getByRole("button", { name: "Start voice conversation" }).click();
  await expect.poll(() => page.evaluate(() => window.fixtureMicCalls)).toBe(2);
  await page.evaluate(() => window.fixtureReleaseMic());
  await expect.poll(() => page.evaluate(() => window.fixtureStops)).toBe(1);
  await page.getByRole("button", { name: "End voice conversation" }).click();
  await expect(page.getByRole("button", { name: "Start voice conversation" })).toBeEnabled();
  expect(await page.evaluate(() => window.fixtureStops)).toBe(2);
});

test("stopping during transcription prevents the late transcript from reaching chat", async ({ page }) => {
  const { calls, releaseTranscription } = await setup(page, { pendingTranscription: true });
  await page.getByRole("button", { name: "Start voice conversation" }).click();
  await expect.poll(() => calls.some((path) => path.endsWith("/speech/transcribe"))).toBe(true);
  await page.getByRole("button", { name: "End voice conversation" }).click();
  releaseTranscription();
  await expect(page.getByRole("button", { name: "Start voice conversation" })).toBeEnabled();
  expect(calls.some((path) => path.endsWith("/chat/completions"))).toBe(false);
});

test("stopping during audio initialization releases the microphone immediately", async ({ page }) => {
  const { calls } = await setup(page, { pendingResume: true });
  await page.getByRole("button", { name: "Start voice conversation" }).click();
  await expect.poll(() => page.evaluate(() => typeof window.fixtureReleaseResume)).toBe("function");
  await page.getByRole("button", { name: "End voice conversation" }).click();
  await expect.poll(() => page.evaluate(() => window.fixtureStops)).toBeGreaterThanOrEqual(1);
  await expect(page.getByRole("button", { name: "Start voice conversation" })).toBeEnabled();
  await page.evaluate(() => window.fixtureReleaseResume());
  expect(calls.some((path) => path.endsWith("/speech/transcribe"))).toBe(false);
});

test("a transcription failure ends the conversation without restarting capture", async ({ page }) => {
  const { calls } = await setup(page, { transcriptionFailure: true });
  await page.getByRole("button", { name: "Start voice conversation" }).click();
  await expect(page.locator("#voiceMeta")).toHaveText("Speech provider unavailable");
  await expect(page.getByRole("button", { name: "Start voice conversation" })).toBeEnabled();
  expect(await page.evaluate(() => window.fixtureMicCalls)).toBe(1);
  expect(calls.some((path) => path.endsWith("/chat/completions"))).toBe(false);
});

test("leaving Voice stops listening", async ({ page }) => {
  await setup(page, { pendingMic: true });
  await page.getByRole("button", { name: "Start voice conversation" }).click();
  await expect.poll(() => page.evaluate(() => window.fixtureMicCalls)).toBe(1);
  await page.getByRole("button", { name: "Chat", exact: true }).click();
  await page.evaluate(() => window.fixtureReleaseMic());
  await expect.poll(() => page.evaluate(() => window.fixtureStops)).toBe(1);
  await page.getByRole("button", { name: "Voice", exact: true }).click();
  await expect(page.getByRole("button", { name: "Start voice conversation" })).toBeEnabled();
});

test("ending while a reply is speaking does not reopen the microphone", async ({ page }) => {
  await setup(page);
  await page.getByRole("button", { name: "Start voice conversation" }).click();
  await expect.poll(() => page.evaluate(() => window.fixtureSpoken)).toBe(1);
  await page.getByRole("button", { name: "End voice conversation" }).click();
  await expect(page.getByRole("button", { name: "Start voice conversation" })).toBeEnabled();
  expect(await page.evaluate(() => window.fixtureMicCalls)).toBe(1);
  expect(await page.evaluate(() => window.fixtureStops)).toBe(1);
});

test("conversation completes multiple turns without manual recording", async ({ page }) => {
  const { calls } = await setup(page, { turns: 2 });
  await page.getByRole("button", { name: "Start voice conversation" }).click();
  await expect.poll(() => page.evaluate(() => window.fixtureSpoken)).toBe(1);
  await page.evaluate(() => window.fixtureFinishSpeech());
  await expect.poll(() => page.evaluate(() => window.fixtureSpoken)).toBe(2);
  expect(calls.filter((path) => path.endsWith("/speech/transcribe"))).toHaveLength(2);
  expect(calls.filter((path) => path.endsWith("/chat/completions"))).toHaveLength(2);
  await page.getByRole("button", { name: "End voice conversation" }).click();
  await expect(page.getByRole("button", { name: "Start voice conversation" })).toBeEnabled();
});

test("leaving Voice discards a manual recording and closes the microphone", async ({ page }) => {
  const { calls } = await setup(page);
  await page.locator("#voiceRecordButton").click();
  await expect(page.locator("#voiceRecordButton")).toHaveText("Stop");
  await page.getByRole("button", { name: "Chat", exact: true }).click();
  await expect.poll(() => page.evaluate(() => window.fixtureStops)).toBe(1);
  expect(calls.some((path) => path.endsWith("/speech/transcribe"))).toBe(false);
  await page.getByRole("button", { name: "Voice", exact: true }).click();
  await expect(page.locator("#voiceRecordButton")).toHaveText("Record");
});

test("leaving during a manual microphone permission request closes its late stream", async ({ page }) => {
  const { calls } = await setup(page, { pendingMic: true });
  await page.locator("#voiceRecordButton").click();
  await expect.poll(() => page.evaluate(() => window.fixtureMicCalls)).toBe(1);
  await page.getByRole("button", { name: "Chat", exact: true }).click();
  await page.evaluate(() => window.fixtureReleaseMic());
  await expect.poll(() => page.evaluate(() => window.fixtureStops)).toBe(1);
  expect(calls.some((path) => path.endsWith("/speech/transcribe"))).toBe(false);
});

test("leaving during manual transcription discards a late result", async ({ page }) => {
  const { calls, releaseTranscription } = await setup(page, { pendingTranscription: true });
  await page.getByLabel("Review transcript before sending").uncheck();
  await page.locator("#voiceRecordButton").click();
  await expect(page.locator("#voiceRecordButton")).toHaveText("Stop");
  await page.locator("#voiceRecordButton").click();
  await expect.poll(() => calls.some((path) => path.endsWith("/speech/transcribe"))).toBe(true);
  await page.getByRole("button", { name: "Chat", exact: true }).click();
  releaseTranscription();
  await page.getByRole("button", { name: "Voice", exact: true }).click();
  await expect(page.locator("#voiceMeta")).toHaveText("Recording discarded.");
  expect(calls.some((path) => path.endsWith("/chat/completions"))).toBe(false);
});

test("a late manual microphone grant cannot disrupt a new recording", async ({ page }) => {
  await setup(page, { pendingMic: true });
  await page.locator("#voiceRecordButton").click();
  await expect.poll(() => page.evaluate(() => window.fixtureMicCalls)).toBe(1);
  await page.getByRole("button", { name: "Chat", exact: true }).click();
  await page.getByRole("button", { name: "Voice", exact: true }).click();
  await page.locator("#voiceRecordButton").click();
  await expect(page.locator("#voiceRecordButton")).toHaveText("Stop");
  await page.evaluate(() => window.fixtureReleaseMic());
  await expect.poll(() => page.evaluate(() => window.fixtureStops)).toBe(1);
  await page.getByRole("button", { name: "Chat", exact: true }).click();
  await expect.poll(() => page.evaluate(() => window.fixtureStops)).toBe(2);
});

test("page hide closes a manual microphone capture", async ({ page }) => {
  const { calls } = await setup(page);
  await page.locator("#voiceRecordButton").click();
  await expect(page.locator("#voiceRecordButton")).toHaveText("Stop");
  await page.evaluate(() => window.dispatchEvent(new PageTransitionEvent("pagehide")));
  await expect.poll(() => page.evaluate(() => window.fixtureStops)).toBe(1);
  expect(calls.some((path) => path.endsWith("/speech/transcribe"))).toBe(false);
});
