// Exercise the current remote HTML with simulated HTTP responses, without modifying the server.
import assert from "node:assert/strict";
import { chromium } from "@playwright/test";
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";

const plan = JSON.parse(execFileSync("bash", ["scripts/env", "worktree", "plan"], { encoding: "utf8" }));
const port = execFileSync("docker", ["compose", "--project-name", plan.project, "--project-directory", process.cwd(), "--env-file", `${plan.state}/environment.env`, "-f", "deploy/compose.worktree.yaml", "port", "app", "8787"], { encoding: "utf8" }).trim();
const html = readFileSync("src-tauri/src/services/remote_app.html", "utf8");
const browser = await chromium.launch({ executablePath: process.env.ARUVI_CHROME_PATH || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome", headless: true });
try {
  const context = await browser.newContext();
  try {
    const page = await context.newPage();
    const requests = [];
    let productsUnavailable = false;
    let releaseTurn;
    let holdTurn = false;
    let notifyHeldTurn;
    const heldTurn = new Promise((resolve) => { notifyHeldTurn = resolve; });
    await page.addInitScript(() => { localStorage.setItem("aruvi.remote.token", "synthetic-token"); localStorage.setItem("aruvi.remote.selected_product", "product-a"); localStorage.setItem("aruvi.remote.selected_node", "old-node"); });
    await page.route("**/remote", (route) => route.fulfill({ contentType: "text/html", body: html }));
    await page.route("**/api/mobile/products", (route) => productsUnavailable ? route.fulfill({ status: 503, body: "unavailable" }) : route.fulfill({ json: [{ id: "product-a", name: "Product A" }, { id: "product-b", name: "Product B" }] }));
    await page.route("**/api/mobile/planner/sessions", (route) => route.fulfill({ json: { session_id: "session-1" } }));
    await page.route("**/api/mobile/planner/sessions/*/turn", async (route) => {
      requests.push(route.request().postDataJSON());
      if (holdTurn) await new Promise((resolve) => { releaseTurn = resolve; notifyHeldTurn(); });
      await route.fulfill({ json: { session_id: "session-1", assistant_message: "Ready", draft_tree_nodes: [{ id: "node-1", label: "Draft node" }] } });
    });
    await page.goto(`http://${port}/remote`);
    await page.locator("#plannerProductSelect").waitFor({ state: "visible" });
    await page.waitForFunction(() => !document.querySelector("#plannerProductSelect").disabled);
    assert.equal(await page.locator("#plannerProductSelect").inputValue(), "product-a");
    await page.locator("#plannerProductSelect").selectOption("product-b");
    await page.locator("#composer").fill("Plan this product");
    let turn = page.waitForResponse((response) => response.url().endsWith("/turn"));
    await page.locator("#sendButton").click();
    await turn;
    assert.equal(requests[0].product_id, "product-b");
    assert.equal(requests[0].selected_draft_node_id, null);
    await page.getByRole("button", { name: "Draft node" }).click();
    holdTurn = true;
    await page.locator("#composer").fill("Wait for this turn");
    turn = page.waitForResponse((response) => response.url().endsWith("/turn"));
    await page.locator("#sendButton").click();
    await page.waitForFunction(() => document.querySelector("#plannerProductSelect").disabled);
    assert.equal(await page.locator("#refreshPlannerProductsButton").isDisabled(), true);
    assert.equal(await page.getByRole("button", { name: "Draft node" }).isDisabled(), true);
    await Promise.race([heldTurn, new Promise((_, reject) => setTimeout(() => reject(new Error("Planner turn was not received")), 5000))]);
    assert.equal(requests[1].selected_draft_node_id, "node-1");
    releaseTurn();
    await turn;
    await page.waitForFunction(() => !document.querySelector("#plannerProductSelect").disabled);
    assert.equal(await page.locator("#plannerProductSelect").inputValue(), "product-b");
    assert.equal(await page.getByRole("button", { name: "Draft node" }).isDisabled(), false);
    holdTurn = false;
    await page.locator("#plannerProductSelect").selectOption("");
    await page.locator("#composer").fill("Plan without a product");
    turn = page.waitForResponse((response) => response.url().endsWith("/turn"));
    await page.locator("#sendButton").click();
    await turn;
    assert.equal(requests[2].product_id, null);
    productsUnavailable = true;
    await page.locator("#refreshPlannerProductsButton").click();
    await page.waitForFunction(() => document.querySelector("#plannerProductStatus").textContent.includes("Products unavailable"));
    await page.locator("#composer").fill("Wait for scope");
    await page.locator("#sendButton").click();
    assert.equal(requests.length, 3);
    await page.locator("#plannerProductSelect").selectOption("");
    turn = page.waitForResponse((response) => response.url().endsWith("/turn"));
    await page.locator("#sendButton").click(); await turn;
    assert.equal(requests[3].product_id, null);
    console.log("Planner preserves product scope during turns and supports explicitly unscoped turns.");
  } finally { await context.close(); }
} finally { await browser.close(); }
