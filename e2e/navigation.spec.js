import { test, expect } from "./support/test.js";

const routes = [
  { key: "planner", path: "/planner", heading: null },
  { key: "products", path: "/products", heading: "Products" },
  { key: "work-items", path: "/work-items", heading: "Delivery / Builder" },
  { key: "ide", path: "/ide", heading: "IDE Workspace" },
  { key: "repositories", path: "/repositories", heading: "Workspaces" },
  { key: "agents", path: "/agents", heading: "Agent Management" },
  { key: "models", path: "/models", heading: "Model Providers" },
  { key: "chat", path: "/chat", heading: "Direct Chat" },
  { key: "voice-chat", path: "/voice-chat", heading: "Voice Chat" },
  { key: "settings", path: "/settings", heading: "Settings" },
];

test("top-level navigation renders every main workspace route", async ({ page }) => {
  await page.goto("/");
  await expect(page).toHaveURL(/\/planner$/);
  await expect(page.getByTestId("planner-input")).toBeVisible();

  for (const route of routes) {
    await page.getByTestId(`nav-${route.key}`).click();
    await expect(page).toHaveURL(new RegExp(`${route.path.replace("/", "\\/")}$`));
    if (route.heading) {
      await expect(page.getByRole("heading", { name: route.heading })).toBeVisible();
    } else {
      await expect(page.getByTestId("planner-input")).toBeVisible();
    }
  }
});

test("scope-aware utility routes avoid full product tree loads without capability context", async ({ page }) => {
  await page.addInitScript(() => {
    window.localStorage.setItem(
      "aruvi-workspace",
      JSON.stringify({
        state: {
          activeProductId: "example-product-calculator",
          activeProductAreaId: null,
          activeCapabilityId: null,
          activeNodeId: null,
          activeNodeType: null,
          activeWorkItemId: null,
          activeRepoId: null,
          activeWorkspacePath: null,
        },
        version: 0,
      }),
    );
  });
  await page.goto("/");
  await expect(page.getByTestId("planner-input")).toBeVisible();
  await page.waitForTimeout(200);
  const plannerCalls = await page.evaluate(() => ({
    productAreas: window.__ARUVI_E2E__.getInvokeCallCount("list_product_areas"),
    fullTree: window.__ARUVI_E2E__.getInvokeCallCount("get_product_tree"),
    workItemPages: window.__ARUVI_E2E__.getInvokeCallCount("list_work_items_page"),
  }));
  expect(plannerCalls.productAreas).toBeGreaterThan(0);
  expect(plannerCalls.fullTree).toBe(0);
  expect(plannerCalls.workItemPages).toBeGreaterThan(0);

  await page.evaluate(() => window.__ARUVI_E2E__.clearInvokeCalls());
  await page.getByTestId("nav-work-items").click();
  await expect(page.getByRole("heading", { name: "Delivery / Builder" })).toBeVisible();
  await page.waitForTimeout(200);
  const workItemCalls = await page.evaluate(() => ({
    productAreas: window.__ARUVI_E2E__.getInvokeCallCount("list_product_areas"),
    fullTree: window.__ARUVI_E2E__.getInvokeCallCount("get_product_tree"),
    workItemPages: window.__ARUVI_E2E__.getInvokeCallCount("list_work_items_page"),
  }));
  expect(workItemCalls.productAreas).toBeGreaterThan(0);
  expect(workItemCalls.fullTree).toBe(0);
  expect(workItemCalls.workItemPages).toBeGreaterThan(0);

  await page.evaluate(() => window.__ARUVI_E2E__.clearInvokeCalls());
  await page.getByTestId("nav-ide").click();
  await expect(page.getByRole("heading", { name: "IDE Workspace" })).toBeVisible();
  await page.waitForTimeout(200);
  const ideCalls = await page.evaluate(() => ({
    productAreas: window.__ARUVI_E2E__.getInvokeCallCount("list_product_areas"),
    fullTree: window.__ARUVI_E2E__.getInvokeCallCount("get_product_tree"),
  }));
  expect(ideCalls.productAreas).toBeGreaterThan(0);
  expect(ideCalls.fullTree).toBe(0);

  await page.evaluate(() => window.__ARUVI_E2E__.clearInvokeCalls());
  await page.getByTestId("nav-repositories").click();
  await expect(page.getByRole("heading", { name: "Workspaces" })).toBeVisible();
  await page.waitForTimeout(200);
  const workspaceCalls = await page.evaluate(() => ({
    productAreas: window.__ARUVI_E2E__.getInvokeCallCount("list_product_areas"),
    fullTree: window.__ARUVI_E2E__.getInvokeCallCount("get_product_tree"),
  }));
  expect(workspaceCalls.productAreas).toBeGreaterThan(0);
  expect(workspaceCalls.fullTree).toBe(0);

  await page.evaluate(() => window.__ARUVI_E2E__.clearInvokeCalls());
  await page.getByTestId("nav-agents").click();
  await expect(page.getByRole("heading", { name: "Agent Management" })).toBeVisible();
  await page.waitForTimeout(200);
  const agentCalls = await page.evaluate(() => ({
    productAreas: window.__ARUVI_E2E__.getInvokeCallCount("list_product_areas"),
    fullTree: window.__ARUVI_E2E__.getInvokeCallCount("get_product_tree"),
  }));
  expect(agentCalls.productAreas).toBeGreaterThan(0);
  expect(agentCalls.fullTree).toBe(0);
});

test("scope-aware utility routes use targeted capability reads with capability context", async ({ page }) => {
  await page.addInitScript(() => {
    window.localStorage.setItem(
      "aruvi-workspace",
      JSON.stringify({
        state: {
          activeProductId: "example-product-calculator",
          activeProductAreaId: "calc-core-math-engine",
          activeCapabilityId: "calc-expression-evaluation",
          activeNodeId: "calc-expression-evaluation",
          activeNodeType: "capability",
          activeWorkItemId: null,
          activeRepoId: null,
          activeWorkspacePath: null,
        },
        version: 0,
      }),
    );
  });

  await page.goto("/ide");
  await expect(page.getByRole("heading", { name: "IDE Workspace" })).toBeVisible();
  await page.waitForTimeout(200);
  const ideCalls = await page.evaluate(() => ({
    capability: window.__ARUVI_E2E__.getInvokeCallCount("get_capability"),
    fullTree: window.__ARUVI_E2E__.getInvokeCallCount("get_product_tree"),
  }));
  expect(ideCalls.capability).toBeGreaterThan(0);
  expect(ideCalls.fullTree).toBe(0);

  await page.evaluate(() => window.__ARUVI_E2E__.clearInvokeCalls());
  await page.getByTestId("nav-repositories").click();
  await expect(page.getByRole("heading", { name: "Workspaces" })).toBeVisible();
  await page.waitForTimeout(200);
  const workspaceCalls = await page.evaluate(() => ({
    capability: window.__ARUVI_E2E__.getInvokeCallCount("get_capability"),
    fullTree: window.__ARUVI_E2E__.getInvokeCallCount("get_product_tree"),
  }));
  expect(workspaceCalls.capability).toBeGreaterThan(0);
  expect(workspaceCalls.fullTree).toBe(0);

  await page.evaluate(() => window.__ARUVI_E2E__.clearInvokeCalls());
  await page.getByTestId("nav-agents").click();
  await expect(page.getByRole("heading", { name: "Agent Management" })).toBeVisible();
  await page.getByRole("button", { name: "Assignments" }).click();
  await page.locator("select").filter({ has: page.locator("option[value='capability']") }).first().selectOption("capability");
  await page.waitForTimeout(200);
  const agentCalls = await page.evaluate(() => ({
    productCapabilities: window.__ARUVI_E2E__.getInvokeCallCount("list_product_capabilities"),
    fullTree: window.__ARUVI_E2E__.getInvokeCallCount("get_product_tree"),
  }));
  expect(agentCalls.productCapabilities).toBeGreaterThan(0);
  expect(agentCalls.fullTree).toBe(0);
});
