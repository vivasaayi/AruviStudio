import { test, expect } from "./support/test.js";
import { writeFile } from "node:fs/promises";

test("desktop product exports include all product work items beyond a list page", async ({ page }) => {
  await page.goto("/product-overview");
  await expect(page.getByRole("heading", { name: "Product Book" })).toBeVisible();
  expect(await page.evaluate(() => window.__ARUVI_E2E__.getInvokeCallCount("list_product_export_work_items"))).toBe(0);
  await page.getByRole("button", { name: "Load delivery items" }).click();
  await expect(page.getByText("Publish keyboard shortcuts guide", { exact: true })).toBeVisible();
  await page.evaluate(() => { window.__ARUVI_E2E__.seedLargeBacklog(501); window.__ARUVI_E2E__.clearInvokeCalls(); });
  let expectedSources = 0;
  const renderedVariants = {};
  for (const [label, command] of [["Docs HTML", "export_product_overview_html"], ["Book HTML", "export_product_overview_html"], ["EPUB", "export_product_overview_epub"], ["PDF", "export_product_overview_pdf"]]) {
    const before = await page.evaluate((name) => window.__ARUVI_E2E__.getInvokeCallCount(name), command);
    await page.getByRole("button", { name: label, exact: true }).click();
    await expect.poll(() => page.evaluate((name) => window.__ARUVI_E2E__.getInvokeCallCount(name), command)).toBe(before + 1);
    const calls = await page.evaluate((name) => ({ source: window.__ARUVI_E2E__.getInvokeCalls("render_product_book"), exports: window.__ARUVI_E2E__.getInvokeCalls(name) }), command);
    expect(calls.source).toHaveLength(++expectedSources);
    expect(calls.source.at(-1).args).toMatchObject({
      format: label === "Docs HTML" ? "docs" : "book",
      renderMode: label === "PDF" ? "print" : label === "EPUB" ? "epub" : "web",
      trimPreset: "trade-6x9",
    });
    const args = calls.exports.at(-1).args;
    expect(args.html ?? args.request?.html).toContain("Virtual backlog story 501");
    renderedVariants[label] = args.html ?? args.request?.html;
    if (label === "EPUB" && process.env.ARUVI_EPUB_FIXTURE_PATH) {
      await writeFile(process.env.ARUVI_EPUB_FIXTURE_PATH, JSON.stringify(args));
    }
  }
  if (process.env.ARUVI_BOOK_PARITY_FIXTURE_PATH) {
    const sourceCalls = await page.evaluate(() => window.__ARUVI_E2E__.getInvokeCalls("render_product_book"));
    const productId = sourceCalls[0].args.productId;
    const source = await page.evaluate((id) => window.__ARUVI_E2E__.invoke("get_product_book_source", { productId: id }), productId);
    await writeFile(process.env.ARUVI_BOOK_PARITY_FIXTURE_PATH, JSON.stringify({ source, renderedVariants }));
  }
});

test("desktop PDF exports use the selected backend trim preset", async ({ page }) => {
  await page.goto("/product-overview");
  await expect(page.getByRole("heading", { name: "Product Book" })).toBeVisible();
  const trim = page.locator("select").filter({ has: page.locator('option[value="a5"]') });
  for (const [preset, width, height] of [["a5", "148mm", "210mm"], ["us-letter", "8.5in", "11in"]]) {
    await trim.selectOption(preset);
    const before = await page.evaluate(() => window.__ARUVI_E2E__.getInvokeCallCount("export_product_overview_pdf"));
    await page.getByRole("button", { name: "PDF", exact: true }).click();
    await expect.poll(() => page.evaluate(() => window.__ARUVI_E2E__.getInvokeCallCount("export_product_overview_pdf"))).toBe(before + 1);
    const calls = await page.evaluate(() => ({ render: window.__ARUVI_E2E__.getInvokeCalls("render_product_book"), exports: window.__ARUVI_E2E__.getInvokeCalls("export_product_overview_pdf") }));
    expect(calls.render.at(-1).args).toMatchObject({ trimPreset: preset, renderMode: "print" });
    expect(calls.exports.at(-1).args.request).toMatchObject({ page_width: width, page_height: height });
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
