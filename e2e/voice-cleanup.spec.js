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
