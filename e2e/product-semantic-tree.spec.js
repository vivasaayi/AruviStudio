import { test, expect } from "./support/test.js";

test("product management exposes area, capability, feature, and work item tabs", async ({ page }) => {
  await page.goto("/products");

  await expect(page.getByRole("heading", { name: "Products" })).toBeVisible();
  await page.getByRole("button", { name: "Product Management" }).click();

  await expect(page.getByRole("button", { name: "Product Areas" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Capabilities" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Features" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Work Items" })).toBeVisible();
  await expect(page.getByText("Core Math Engine", { exact: true }).first()).toBeVisible();
  await expect(page.getByRole("button", { name: "+ Product Area" })).toBeVisible();

  await page.getByRole("button", { name: "Capabilities" }).click();
  await expect(page.getByText("Expression Evaluation", { exact: true }).first()).toBeVisible();
  await expect(page.getByRole("button", { name: "+ Capability" })).toBeVisible();

  await page.getByRole("button", { name: "Features" }).click();
  await expect(page.getByText("Scientific Mode Slice", { exact: true }).first()).toBeVisible();
  await expect(page.getByRole("button", { name: "+ Feature" })).toBeVisible();

  await page.getByRole("button", { name: "Work Items" }).click();
  await expect(page.getByText("Ship scientific mode slice checklist", { exact: true }).first()).toBeVisible();
  await expect(page.getByText("Story Details", { exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "Open Builder" })).toBeVisible();

  await page.getByRole("button", { name: "+ Story" }).click();
  await expect(page.getByLabel("Story title")).toBeVisible();
  await page.getByLabel("Story title").fill("Validate option payoff story");
  await page.getByLabel("Problem Statement").fill("Option payoff needs deterministic validation.");
  await page.getByLabel("Description").fill("Confirm option payoff output is deterministic.");
  await page.getByLabel("Acceptance Criteria").fill("Payoff output is reproducible for the same inputs.");
  await page.getByRole("button", { name: "Add Story" }).click();
  await expect(page.getByText("Validate option payoff story", { exact: true }).first()).toBeVisible();
  await expect(page.getByText("Option payoff needs deterministic validation.", { exact: true })).toBeVisible();

  await page.getByRole("button", { name: "Edit" }).last().click();
  await expect(page.getByLabel("Story title")).toBeVisible();
  await page.getByLabel("Story title").fill("Validate option payoff story edited");
  await page.getByRole("button", { name: "Save Story" }).click();
  await expect(page.getByText("Validate option payoff story edited", { exact: true }).first()).toBeVisible();

  await page.getByRole("button", { name: "+ Task" }).click();
  await expect(page.getByLabel("Task title")).toBeVisible();
  await page.getByLabel("Task title").fill("Add payoff fixture");
  await page.getByLabel("Description").fill("Create a fixture for deterministic option payoff output.");
  await page.getByRole("button", { name: "Add Task" }).click();
  await expect(page.getByText("Add payoff fixture", { exact: true })).toBeVisible();

  await page.getByRole("button", { name: "Edit" }).last().click();
  await expect(page.getByLabel("Task title")).toBeVisible();
  await page.getByLabel("Task title").fill("Add payoff fixture edited");
  await page.getByRole("button", { name: "Save Task" }).click();
  await expect(page.getByText("Add payoff fixture edited", { exact: true })).toBeVisible();

  await page.getByRole("button", { name: "Delete" }).last().click();
  await page.getByLabel("Type the title to confirm").fill("Add payoff fixture edited");
  await page.getByRole("checkbox").check();
  await page.getByRole("button", { name: "Delete task" }).click();
  await expect(page.getByText("Add payoff fixture edited", { exact: true })).toHaveCount(0);

  await page.getByRole("button", { name: "Delete" }).last().click();
  await page.getByLabel("Type the title to confirm").fill("Validate option payoff story edited");
  await page.getByRole("checkbox").check();
  await page.getByRole("button", { name: "Delete story" }).click();
  await expect(page.getByText("Validate option payoff story edited", { exact: true })).toHaveCount(0);
});

test("delivery builder shows owner badges and product-level ownership", async ({ page }) => {
  await page.goto("/work-items");

  await expect(page.getByRole("heading", { name: "Delivery / Builder" })).toBeVisible();
  await expect(page.getByText("Publish keyboard shortcuts guide")).toBeVisible();
  await expect(page.getByText("Owner: Product", { exact: true })).toBeVisible();
  await expect(page.getByText("Owner: Capability", { exact: true }).first()).toBeVisible();
  await expect(page.getByText("Calculator / Core Math Engine / Expression Evaluation", { exact: true })).toBeVisible();
  await expect(page.getByText("Owner: Feature", { exact: true }).first()).toBeVisible();
  await expect(page.getByText("Calculator / Core Math Engine / Expression Evaluation / Scientific Mode Slice", { exact: true }).first()).toBeVisible();
});

test("product management renders every child task for the selected story", async ({ page }) => {
  await page.goto("/products");

  await page.getByRole("button", { name: "Product Management" }).click();
  await page.getByRole("button", { name: "Work Items" }).click();
  await page.getByText("Make story task lists reliable after task creation", { exact: true }).click();

  await expect(page.getByText("Make story task lists reliable after task creation", { exact: true }).first()).toBeVisible();
  await expect(page.getByText("Load selected story tasks through get_sub_work_items", { exact: true })).toBeVisible();
  await expect(page.getByText("Add regression coverage for multi-task story display", { exact: true })).toBeVisible();
  await expect(page.getByText("Tasks", { exact: true })).toBeVisible();
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
