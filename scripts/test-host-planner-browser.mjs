// Drive repository analysis in the disposable real-model fixture; configuration arrives on stdin.
import assert from "node:assert/strict";
import { chromium } from "@playwright/test";

let input = "";
for await (const chunk of process.stdin) input += chunk;
const fixture = JSON.parse(input);
const browser = await chromium.launch({ executablePath: process.env.ARUVI_CHROME_PATH || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome", headless: true });
try {
  const page = await browser.newPage();
  await page.addInitScript((config) => {
    localStorage.setItem("aruvi.remote.token", "workflow-mobile-test");
    localStorage.setItem("aruvi.remote.session", config.session);
    localStorage.setItem("aruvi.remote.selected_product", config.product);
  }, fixture);
  await page.goto(`http://${fixture.address}/remote`);
  await page.locator("#plannerProductSelect").waitFor({ state: "visible" });
  await page.waitForFunction(() => !document.querySelector("#plannerProductSelect").disabled);
  await page.locator("#plannerProductSelect").selectOption(fixture.product);
  await page.locator("#refreshPlannerRepositoriesButton").click();
  await page.locator("#plannerRepositorySelect").selectOption(fixture.repository);
  const completed = page.waitForResponse((response) => response.url().endsWith("/analyze-repository") && response.request().method() === "POST", { timeout: 240000 });
  await page.locator("#analyzePlannerRepositoryButton").click();
  const response = await completed;
  const payload = await response.text();
  assert.equal(response.ok(), true, `Analysis HTTP ${response.status()}: ${payload.slice(0, 1000)}`);
  const result = JSON.parse(payload);
  await page.locator("#draftTree").getByText(/inventory/i).first().waitFor({ timeout: 10000 });
  const downloadReady = page.waitForEvent("download");
  await page.getByRole("button", { name: "Download review packet" }).click();
  const download = await downloadReady;
  const chunks = [];
  for await (const chunk of await download.createReadStream()) chunks.push(chunk);
  const packet = Buffer.concat(chunks).toString("utf8");
  assert.match(packet, /inventory/i);
  assert.match(packet, /Approval Section/);
  process.stdout.write(JSON.stringify(result));
} finally { await browser.close(); }
