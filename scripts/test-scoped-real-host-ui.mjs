// Disposable Studio browser check; the synthetic token is supplied on stdin.
import { chromium } from "@playwright/test";

let input = "";
for await (const chunk of process.stdin) input += chunk;
const fixture = JSON.parse(input);
const endpoint = new URL(fixture.address);
if (endpoint.protocol !== "http:" || endpoint.hostname !== "127.0.0.1" || endpoint.username || endpoint.password || endpoint.pathname !== "/") throw new Error("Expected a loopback fixture");
const browser = await chromium.launch({ executablePath: process.env.ARUVI_CHROME_PATH || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome", headless: true });
try {
  const page = await browser.newPage();
  await page.goto(new URL("/remote", endpoint).href);
  await page.locator("#tokenInput").fill(fixture.token);
  await page.locator("#providerInput").fill(fixture.provider);
  await page.locator("#modelInput").fill(fixture.model);
  await page.locator("#saveButton").click();
  await page.getByRole("button", { name: "Chat", exact: true }).click();
  await page.getByLabel("Stream response").check();
  await page.locator("#chatComposer").fill("Reply with exactly the word READY.");
  await page.locator("#chatSendButton").click();
  await page.waitForFunction(() => {
    const messages = document.querySelectorAll("#chatConversation .bubble.assistant");
    const text = messages[messages.length - 1]?.textContent || "";
    return text.toLowerCase().includes("ready") && !text.includes("Response incomplete") &&
      document.querySelector("#status")?.textContent === "Connected" &&
      !document.querySelector("#chatSendButton")?.disabled;
  }, null, { timeout: 180000 });
  console.log("Disposable browser streaming completed with the scoped credential.");
} finally {
  await browser.close();
}
