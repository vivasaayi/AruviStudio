// Invoked by the disposable native CI fixture while both services are alive.
import { chromium, expect } from "@playwright/test";

const { ARUVI_TEST_CI_URL: url, ARUVI_TEST_CI_TOKEN: token, ARUVI_TEST_CI_RUN: run,
  ARUVI_TEST_CI_ARTIFACT: artifact, ARUVI_TEST_CI_COMMIT: commit } = process.env;
if (![url, token, run, artifact, commit].every(Boolean)) throw new Error("Missing CI fixture configuration");
const browser = await chromium.launch({ headless: true, executablePath: process.env.ARUVI_TEST_CI_BROWSER_EXECUTABLE || undefined });
try {
  const page = await browser.newPage();
  await page.goto(url + "/remote");
  await page.locator("#tokenInput").fill(token);
  await page.locator("#saveButton").click();
  await page.getByRole("button", { name: "CI", exact: true }).click();
  await expect(page.locator("#ciStatus")).toHaveText("Service: online");
  await expect(page.locator("#ciTargets")).toContainText("fixture");
  await page.getByRole("button", { name: new RegExp(`Run #${run}\\b`) }).click();
  const details = page.locator("#ciRunDetails");
  await expect(details).toContainText(commit);
  await expect(details).toContainText('"status": "passed"');
  for (const stage of ["secrets", "fmt", "clippy", "rust-tests", "package", "verify-package"]) {
    await expect(details.getByRole("button", { name: `Log: ${stage}`, exact: true })).toBeVisible();
  }
  const logResponse = page.waitForResponse((response) => response.url().includes(`/ci/runs/${run}/logs?`) && response.url().includes("stage=rust-tests"));
  await details.getByRole("button", { name: "Log: rust-tests", exact: true }).click();
  const log = await logResponse;
  if (!log.ok()) throw new Error("CI log request failed");
  const chunk = await log.json();
  if (!chunk.text || typeof chunk.next_offset !== "number") throw new Error("CI test log is empty or malformed");
  await expect(details).toContainText(chunk.text.trim().slice(0, 80));
  const readinessResponse = page.waitForResponse((response) => response.url().endsWith(`/ci/artifacts/${artifact}/readiness`));
  await details.getByRole("button", { name: new RegExp(`^Verify artifact #${artifact}\\b`) }).click();
  const readiness = await readinessResponse;
  if (!readiness.ok()) throw new Error("CI artifact verification failed");
  const verified = await readiness.json();
  if (verified.run_id !== Number(run) || verified.eligible !== false) throw new Error("Unexpected artifact readiness");
  await expect(details).toContainText('"eligible": false');
  console.log("Real CI browser: target, run, stages, log and artifact verification passed.");
} finally {
  await browser.close();
}
