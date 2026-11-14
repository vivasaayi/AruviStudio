import { test, expect } from "./support/test.js";

test("desktop product exports include all product work items beyond a list page", async ({ page }) => {
  await page.goto("/product-overview");
  await expect(page.getByRole("heading", { name: "Product Book" })).toBeVisible();
  expect(await page.evaluate(() => window.__ARUVI_E2E__.getInvokeCallCount("list_product_export_work_items"))).toBe(0);
  await page.getByRole("button", { name: "Load delivery items" }).click();
  await expect(page.getByText("Publish keyboard shortcuts guide", { exact: true })).toBeVisible();
  await page.evaluate(() => { window.__ARUVI_E2E__.seedLargeBacklog(501); window.__ARUVI_E2E__.clearInvokeCalls(); });
  let expectedSources = 0;
  for (const [label, command] of [["Docs HTML", "export_product_overview_html"], ["Book HTML", "export_product_overview_html"], ["EPUB", "export_product_overview_epub"], ["PDF", "export_product_overview_pdf"]]) {
    const before = await page.evaluate((name) => window.__ARUVI_E2E__.getInvokeCallCount(name), command);
    await page.getByRole("button", { name: label, exact: true }).click();
    await expect.poll(() => page.evaluate((name) => window.__ARUVI_E2E__.getInvokeCallCount(name), command)).toBe(before + 1);
    const calls = await page.evaluate((name) => ({ source: window.__ARUVI_E2E__.getInvokeCalls("list_product_export_work_items"), exports: window.__ARUVI_E2E__.getInvokeCalls(name) }), command);
    expect(calls.source).toHaveLength(++expectedSources);
    const args = calls.exports.at(-1).args;
    expect(args.html ?? args.request?.html).toContain("Virtual backlog story 501");
  }
});

test("product book reports work-item load failure and does not export incomplete data", async ({ page }) => {
  await page.goto("/product-overview");
  await expect(page.getByRole("heading", { name: "Product Book" })).toBeVisible();
  await page.evaluate(() => { window.__ARUVI_E2E__.setProductExportFailure(true); window.__ARUVI_E2E__.clearInvokeCalls(); });
  await page.getByRole("button", { name: "Load delivery items" }).click();
  await expect(page.getByText(/Unable to load product work items/)).toBeVisible();
  await page.getByRole("button", { name: "Docs HTML", exact: true }).click();
  await expect(page.getByText(/Export failed:.*Synthetic product work-item read failure/)).toBeVisible();
  expect(await page.evaluate(() => window.__ARUVI_E2E__.getInvokeCallCount("export_product_overview_html"))).toBe(0);
  await page.evaluate(() => window.__ARUVI_E2E__.setProductExportFailure(false));
  await page.getByRole("button", { name: "Retry loading delivery items" }).click();
  await expect(page.getByText("Publish keyboard shortcuts guide", { exact: true })).toBeVisible();
});
