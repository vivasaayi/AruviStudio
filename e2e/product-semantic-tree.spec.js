import { test, expect } from "./support/test.js";

test("product management supports capability navigation and review deep links", async ({ page }) => {
  await page.goto("/products");

  await expect(page.getByRole("heading", { name: "Products" })).toBeVisible();
  await page.getByRole("button", { name: "Product Management" }).click();
  await expect(page.getByPlaceholder("Search nodes")).toBeVisible();
  await expect(page.getByRole("button", { name: /^Product$/ })).toBeVisible();
  await expect(page.getByRole("button", { name: /^Management Tree$/ })).toBeVisible();
  await expect(page.getByRole("button", { name: /^References$/ })).toBeVisible();

  await page.getByText("Expression Evaluation").first().click();
  await expect(page.getByText("Selected Node", { exact: true })).toBeVisible();
  await expect(page.getByText("Features for the selected capability are listed below.")).toBeVisible();
  await expect(page.getByRole("button", { name: "+ Feature" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Edit Capability" })).toBeVisible();

  await page.getByRole("button", { name: /^References$/ }).first().click();
  await expect(page.getByText("Attached Context", { exact: true })).toBeVisible();
  await expect(page.getByText("No references are attached to this scope yet.")).toBeVisible();
  await page.getByPlaceholder("Architecture note, standard, evidence packet").fill("Expression Evaluation Contract");
  await page.getByPlaceholder("Relevant context, constraints, or evidence").fill("Evaluation must remain deterministic across capability slices.");
  await page.getByRole("button", { name: "Add Reference" }).click();
  await expect(page.getByText("Expression Evaluation Contract", { exact: true })).toBeVisible();

  await page.getByRole("button", { name: /^Product$/ }).click();
  await page.getByRole("button", { name: "Open In Book" }).click();

  await expect(page).toHaveURL(/\/product-overview#capability-calc-expression-evaluation$/);
  await expect(page.getByRole("heading", { name: "Product Overview" })).toBeVisible();
  await expect(page.locator("#capability-calc-expression-evaluation")).toBeVisible();
  await expect(page.locator("#capability-calc-expression-evaluation")).toContainText("Expression Evaluation");
});

test("delivery builder shows owner badges and product-level ownership", async ({ page }) => {
  await page.goto("/work-items");

  await expect(page.getByRole("heading", { name: "Delivery / Builder" })).toBeVisible();
  await expect(page.getByText("Publish keyboard shortcuts guide")).toBeVisible();
  await expect(page.getByText("Owner: Product", { exact: true })).toBeVisible();
  await expect(page.getByText("Owner: Capability", { exact: true }).first()).toBeVisible();
  await expect(page.getByText("Calculator / Core Math Engine / Expression Evaluation", { exact: true })).toBeVisible();
  await expect(page.getByText("Owner: Feature", { exact: true })).toBeVisible();
  await expect(page.getByText("Calculator / Core Math Engine / Expression Evaluation / Scientific Mode Slice", { exact: true })).toBeVisible();
});

test("portfolio and products expose cross-product capability dependencies", async ({ page }) => {
  await page.goto("/portfolio");

  await expect(page.getByRole("heading", { name: "Portfolio" })).toBeVisible();
  await expect(page.getByText("Shared Platforms", { exact: true })).toBeVisible();
  await expect(page.getByText("WiFi Platform", { exact: true }).first()).toBeVisible();
  await expect(page.getByText("Calculator", { exact: true }).first()).toBeVisible();
  await expect(page.getByText("depends on WiFi Platform", { exact: true })).toBeVisible();

  await page.getByTestId("nav-products").click();
  await page.getByRole("button", { name: "Dependencies" }).first().click();
  await expect(page.getByText("platform · active", { exact: true })).toBeVisible();
  await expect(page.getByText("depends on WiFi Platform / Connectivity Services / Secure Device Pairing", { exact: true })).toBeVisible();
});

test("portfolio manage tab edits strategy hierarchy with modals and double-confirm delete", async ({ page }) => {
  await page.goto("/portfolio");

  await expect(page.getByRole("heading", { name: "Portfolio" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Summary" })).toBeVisible();
  await page.getByRole("button", { name: "Manage" }).click();

  await expect(page.getByText("Strategy Hierarchy")).toBeVisible();
  await page.getByRole("button", { name: "Add Strategic Area" }).click();
  await expect(page.getByText("Add Strategy Node")).toBeVisible();
  await page.getByLabel("Strategy node name").fill("Connected Devices");
  await page.getByLabel("Owner or hat").fill("Founder");
  await page.getByLabel("Strategy node description").fill("Devices and wearables strategy.");
  await page.getByRole("button", { name: "Save Node" }).click();

  await expect(page.getByText("Connected Devices", { exact: true }).first()).toBeVisible();
  await page.getByLabel("Product to link").selectOption({ label: "Calculator" });
  await page.getByRole("button", { name: "Link Product" }).click();
  await expect(page.getByText("Linked Products", { exact: true })).toBeVisible();
  await expect(page.getByText("Calculator", { exact: true }).first()).toBeVisible();

  await page.getByRole("button", { name: "Add child to Connected Devices" }).click();
  await page.getByLabel("Strategy node name").fill("Wearables");
  await page.getByRole("button", { name: "Save Node" }).click();
  await expect(page.getByText("Wearables", { exact: true }).first()).toBeVisible();

  await page.getByRole("button", { name: "Edit Wearables" }).click();
  await page.getByLabel("Strategy node name").fill("Wearable Computing");
  await page.getByRole("button", { name: "Save Node" }).click();
  await expect(page.getByText("Wearable Computing", { exact: true }).first()).toBeVisible();

  await page.getByRole("button", { name: "Delete Wearable Computing" }).click();
  await expect(page.getByText("Delete Strategy Node")).toBeVisible();
  await expect(page.getByRole("button", { name: "Delete Node" })).toBeDisabled();
  await page.getByLabel("I understand this removes the selected strategy branch.").check();
  await page.locator("input").last().fill("Wearable Computing");
  await expect(page.getByRole("button", { name: "Delete Node" })).toBeEnabled();
  await page.getByRole("button", { name: "Delete Node" }).click();
  await expect(page.getByText("Wearable Computing", { exact: true })).toHaveCount(0);
});
