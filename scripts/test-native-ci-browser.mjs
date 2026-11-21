// Invoked by the disposable native CI fixture while both services are alive.
import { chromium, expect } from "@playwright/test";

const { ARUVI_TEST_CI_URL: url, ARUVI_TEST_CI_TOKEN: token, ARUVI_TEST_CI_RUN: run,
  ARUVI_TEST_CI_ARTIFACT: artifact, ARUVI_TEST_CI_COMMIT: commit, ARUVI_TEST_CI_BROWSER_COMMIT: browserCommit,
  ARUVI_TEST_CI_PRODUCT: product, ARUVI_TEST_CI_REPOSITORY: repository } = process.env;
if (![url, token, run, artifact, commit, browserCommit, product, repository].every(Boolean)) throw new Error("Missing CI fixture configuration");
const browser = await chromium.launch({ headless: true, executablePath: process.env.ARUVI_TEST_CI_BROWSER_EXECUTABLE || undefined });
try {
  const page = await browser.newPage();
  page.setDefaultTimeout(15000);
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
  await page.locator("#ciProduct").selectOption(product);
  await page.locator("#ciRepository").selectOption(repository);
  await page.locator("#ciTarget").selectOption("fixture");
  await page.locator("#ciRevisionType").selectOption("commit");
  await expect(page.locator("#ciRevision option").filter({ hasText: browserCommit.slice(0, 12) })).toHaveCount(1);
  await page.locator("#ciRevision").selectOption(browserCommit);
  const dialogs = [];
  page.on("dialog", (dialog) => { dialogs.push(dialog.message()); return dialog.accept(); });
  const queuedResponse = page.waitForResponse((response) => response.url().endsWith("/api/mobile/ci/bindings/queue"));
  await page.locator("#ciQueue").click();
  const queued = await queuedResponse;
  if (!queued.ok()) throw new Error("Browser CI queue request failed: " + await queued.text());
  const result = await queued.json();
  if (!result.run_id || String(result.run_id) === run) throw new Error("Browser did not queue a distinct run");
  await expect.poll(() => dialogs.length).toBe(2);
  await expect(page.locator("#ciRuns")).toContainText(`Run #${result.run_id}`);
  let finished;
  await expect.poll(async () => {
    const response = await page.request.get(`${url}/api/mobile/ci/runs/${result.run_id}`, { headers: { Authorization: `Bearer ${token}` } });
    if (!response.ok()) throw new Error("Cannot read browser-queued CI run");
    finished = await response.json();
    if (["failed", "blocked", "cancelled", "interrupted"].includes(finished.status)) throw new Error(`Browser-queued CI run ended: ${finished.status}`);
    return finished.status;
  }, { timeout: 300000, intervals: [500, 1000, 2000] }).toBe("passed");
  if (finished.commit !== browserCommit) throw new Error("Browser-queued run used a different commit");
  console.log("Real CI browser: inspection, artifact verification and a distinct completed browser-queued build passed.");
} finally {
  await browser.close();
}
