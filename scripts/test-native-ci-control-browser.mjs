// Invoked only by the disposable real-CI fixture.
import { chromium, expect } from "@playwright/test";

const { ARUVI_TEST_CI_URL: url, ARUVI_TEST_CI_TOKEN: token, ARUVI_TEST_CI_RUN: run,
  ARUVI_TEST_CI_CONTROL: action, ARUVI_TEST_CI_BROWSER_EXECUTABLE: executablePath } = process.env;
if (![url, token, run].every(Boolean) || !["cancel", "retry"].includes(action)) throw new Error("Missing CI control fixture configuration");
const browser = await chromium.launch({ headless: true, executablePath: executablePath || undefined });
try {
  const page = await browser.newPage();
  page.setDefaultTimeout(15000);
  await page.goto(url + "/remote");
  await page.locator("#tokenInput").fill(token);
  await page.locator("#saveButton").click();
  await page.getByRole("button", { name: "CI", exact: true }).click();
  await expect(page.locator("#ciStatus")).toHaveText("Service: online");
  await page.getByRole("button", { name: new RegExp(`Run #${run}\\b`) }).click();
  page.once("dialog", (dialog) => dialog.accept());
  const response = page.waitForResponse((result) => result.url().endsWith(`/api/mobile/ci/runs/${run}/${action}`));
  await page.getByRole("button", { name: action === "cancel" ? "Cancel build" : "Retry build", exact: true }).click();
  const result = await response;
  if (!result.ok()) throw new Error(`CI ${action} request failed`);
  console.log(JSON.stringify(await result.json()));
} finally {
  await browser.close();
}
