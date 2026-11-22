// Drive a planner turn or repository analysis in the disposable real-model fixture.
import assert from "node:assert/strict";
import { chromium } from "@playwright/test";

let input = "";
for await (const chunk of process.stdin) input += chunk;
const fixture = JSON.parse(input);
function plannerDiagnostic(result) {
  return JSON.stringify({
    status: result.status,
    assistant_message: result.assistant_message,
    execution_errors: result.execution_errors,
    trace_events: (result.trace_events || []).filter((event) => ["model", "format_retry", "draft_retry", "error"].includes(event.stage)),
  }).slice(0, 5000);
}
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
  let completed;
  if (fixture.repository) {
    await page.locator("#refreshPlannerRepositoriesButton").click();
    await page.locator("#plannerRepositorySelect").selectOption(fixture.repository);
    completed = page.waitForResponse((response) => response.url().endsWith("/analyze-repository") && response.request().method() === "POST", { timeout: 240000 });
    await page.locator("#analyzePlannerRepositoryButton").click();
  } else if (fixture.existing_area) {
    await page.getByRole("button", { name: "Products", exact: true }).click();
    await page.locator("#productList .product-card").filter({ hasText: "Planner validation product" }).click();
    await page.getByRole("button", { name: "Load product tree" }).click();
    await page.locator("#productTree summary").filter({ hasText: "Inventory Management" }).click();
    await page.getByRole("button", { name: "Plan children for Inventory Management", exact: true }).click();
    await page.locator("#composer").waitFor({ state: "visible" });
    assert.equal(await page.evaluate(() => localStorage.getItem("aruvi.remote.selected_node")), fixture.existing_area);
    await page.locator("#composer").fill("Add one capability named Stock Tracking under the selected Inventory Management product area to track stock levels.");
    completed = page.waitForResponse((response) => response.url().endsWith("/turn") && response.request().method() === "POST", { timeout: 240000 });
    await page.locator("#sendButton").click();
  } else {
    await page.locator("#composer").fill("Add one product area named Inventory Management for tracking stock levels to this product.");
    completed = page.waitForResponse((response) => response.url().endsWith("/turn") && response.request().method() === "POST", { timeout: 240000 });
    await page.locator("#sendButton").click();
  }
  const response = await completed;
  assert.equal(response.request().postDataJSON().product_id, fixture.product);
  if (fixture.existing_area) assert.equal(response.request().postDataJSON().selected_draft_node_id, fixture.existing_area);
  const payload = await response.text();
  assert.equal(response.ok(), true, `Planner HTTP ${response.status()}: ${payload.slice(0, 1000)}`);
  let result = JSON.parse(payload);
  try {
    await page.locator("#draftTree").getByText(/inventory/i).first().waitFor({ timeout: 10000 });
  } catch {
    throw new Error(`Planner response did not show Inventory: ${plannerDiagnostic(result)}`);
  }
  if (fixture.selected_node) {
    const flatten = (nodes) => (nodes || []).flatMap((node) => [node, ...flatten(node.children)]);
    const initialNodes = flatten(result.draft_tree_nodes);
    assert.equal(initialNodes.filter((node) => node.node_type === "product_area").length, 1, "Initial draft must contain exactly one area");
    assert.equal(initialNodes.filter((node) => ["capability", "feature"].includes(node.node_type)).length, 0, "Initial draft must not contain unrequested children");
    await page.locator("#draftTree button").filter({ hasText: /inventory/i }).first().click();
    const selectedNode = await page.evaluate(() => localStorage.getItem("aruvi.remote.selected_node"));
    assert.ok(selectedNode, "Expected an inventory draft node to be selected");
    await page.locator("#composer").fill("Add one capability named Stock Tracking under the selected Inventory Management product area to track stock levels.");
    const turn = page.waitForResponse((response) => response.url().endsWith("/turn") && response.request().method() === "POST", { timeout: 240000 });
    await page.locator("#sendButton").click();
    const selectedResponse = await turn;
    assert.equal(selectedResponse.request().postDataJSON().selected_draft_node_id, selectedNode);
    assert.equal(selectedResponse.request().postDataJSON().product_id, fixture.product);
    const selectedPayload = await selectedResponse.text();
    assert.equal(selectedResponse.ok(), true, `Selected-node planner HTTP ${selectedResponse.status()}: ${selectedPayload.slice(0, 1000)}`);
    result = JSON.parse(selectedPayload);
    try {
      await page.locator("#draftTree").getByText(/stock/i).first().waitFor({ timeout: 10000 });
    } catch {
      throw new Error(`Selected-node response did not show Stock: ${plannerDiagnostic(result)}`);
    }
  }
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
