import { test, expect } from "./support/test.js";

for (const screen of ["chat", "planner"]) {
  for (const stopMode of ["automatic", "manual"]) {
    test(`${screen} ${stopMode} voice stop transcribes once and releases the microphone`, async ({ page }) => {
      await page.addInitScript(() => {
        const originalTimeout = window.setTimeout.bind(window);
        window.setTimeout = (callback, delay, ...args) => originalTimeout(callback, delay === 30000 ? 300 : delay, ...args);
        window.fixtureMicrophoneStops = 0;
        Object.defineProperty(navigator, "mediaDevices", { configurable: true, value: {
          getUserMedia: async () => ({ getTracks: () => [{ stop() { window.fixtureMicrophoneStops += 1; } }] }),
        } });
        window.AudioContext = class {
          constructor() { this.sampleRate = 16000; this.destination = {}; }
          async resume() {}
          async close() {}
          createMediaStreamSource() { return { connect() {}, disconnect() {} }; }
          createGain() { return { gain: { value: 1 }, connect() {}, disconnect() {} }; }
          createScriptProcessor() { return { connect() { originalTimeout(() => this.onaudioprocess({ inputBuffer: { getChannelData: () => new Float32Array([0.5, -0.5]) } }), 20); }, disconnect() {} }; }
        };
      });
      await page.goto(screen === "chat" ? "/chat" : "/");
      if (screen === "planner") await page.getByLabel("Planner product").selectOption("example-product-calculator");
      const start = screen === "chat" ? page.getByTestId("chat-mic-toggle") : page.getByRole("button", { name: "Start Voice Input" });
      await start.click();
      if (stopMode === "manual") {
        const stop = screen === "chat" ? page.getByTestId("chat-mic-toggle") : page.getByRole("button", { name: "Stop Recording" });
        await expect(stop).toContainText(screen === "chat" ? "Stop Listening" : "Stop Recording");
        await page.waitForTimeout(60);
        await stop.click();
      }
      await expect.poll(() => page.evaluate(() => window.__ARUVI_E2E__.getInvokeCalls("transcribe_audio_command").length)).toBe(1);
      await page.waitForTimeout(400);
      expect(await page.evaluate(() => window.__ARUVI_E2E__.getInvokeCalls("transcribe_audio_command").length)).toBe(1);
      expect(await page.evaluate(() => window.fixtureMicrophoneStops)).toBe(1);
      if (screen === "chat") await expect(page.getByTestId("chat-input")).toHaveValue("Add voice-driven planning for the selected node");
    });
  }
}
