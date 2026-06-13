import { test, expect } from "./support/test.js";

test("planner supports deterministic create, refine, packet, and apply flow", async ({ page }) => {
  await page.goto("/");

  await expect(page.getByTestId("planner-input")).toBeVisible();
  await page.getByLabel("Planner product").selectOption("example-product-calculator");

  await page.getByTestId("planner-input").fill("Design input, history, reporting, and expression modules for this product");
  await page.getByTestId("planner-send").click();

  await page.getByTestId("planner-view-draft").click();
  await expect(page.getByTestId("draft-node-draft-product-calculator")).toBeVisible();
  await expect(page.getByTestId("draft-node-draft-module-input-experience")).toBeVisible();
  await expect(page.getByTestId("draft-node-draft-module-calculation-history")).toBeVisible();

  await page.getByTestId("draft-node-draft-module-input-experience").click();
  await page.getByTestId("draft-node-rename-input").fill("Input Workspace");
  await page.getByTestId("draft-node-rename-save").click();
  await expect(page.getByText("Input Workspace").first()).toBeVisible();

  await page.getByTestId("draft-node-draft-product-calculator").click();
  await page.getByTestId("draft-node-add-child-type").selectOption("module");
  await page.getByTestId("draft-node-add-child-name").fill("Concierge Experience");
  await page.getByTestId("draft-node-add-child-summary").fill("Handle concierge requests and premium guest experiences.");
  await page.getByTestId("draft-node-add-child-save").click();
  await expect(page.getByText("Concierge Experience").first()).toBeVisible();

  await page.getByTestId("draft-node-delete").click();
  await expect(page.getByTestId("draft-node-draft-module-concierge-experience")).toHaveCount(0);

  await page.getByTestId("draft-node-draft-product-calculator").click();
  await page.getByTestId("planner-input").fill("Add email and WhatsApp notifications to this product");
  await page.getByTestId("planner-send").click();
  await expect(page.getByTestId("draft-node-draft-module-notifications-messaging")).toBeVisible();

  await page.getByTestId("draft-node-draft-module-notifications-messaging").click();
  await page.getByTestId("planner-input").fill("Enhance this module with guest notification preferences and outbound delivery tracking");
  await page.getByTestId("planner-send").click();
  await expect(page.getByTestId("draft-node-draft-capability-outbound-delivery-tracking")).toBeVisible();

  await page.getByTestId("draft-node-draft-capability-outbound-delivery-tracking").click();
  await page.getByTestId("planner-input").fill("Add work items to implement this capability");
  await page.getByTestId("planner-send").click();
  await expect(page.getByTestId("draft-node-draft-work_item-implement-delivery-audit-timeline")).toBeVisible();

  await page.getByTestId("draft-node-draft-work_item-implement-delivery-audit-timeline").click();
  await page.getByTestId("planner-input").fill("Revise this work item to include WhatsApp consent capture");
  await page.getByTestId("planner-send").click();
  await expect(page.getByText("Implement Delivery Audit Timeline and Consent Handling").first()).toBeVisible();

  await page.getByRole("button", { name: "Generate Packet" }).click();
  await expect(page.getByText("Packet exported to /tmp/aruvi-e2e/")).toBeVisible();

  await page.getByTestId("draft-commit").click();
  await expect(page.getByText("Applied design to catalog.").first()).toBeVisible();

  await page.getByTestId("nav-products").click();
  await expect(page.getByRole("heading", { name: "Products" })).toBeVisible();
});

test("planner can reverse engineer a registered repository into a design tree", async ({ page }) => {
  await page.goto("/");

  await expect(page.getByTestId("planner-input")).toBeVisible();
  await page.getByLabel("Planner product").selectOption("example-product-calculator");
  await page.getByRole("button", { name: "Reverse engineer repository" }).click();
  await expect(page.getByText("Reverse Engineer Repository")).toBeVisible();

  await page.getByPlaceholder("/absolute/path/to/repository").fill("/tmp/aruvi-studio");
  await page.getByRole("button", { name: "Register Repo" }).click();
  await expect(page.getByText('Registered repository "aruvi-studio".')).toBeVisible();

  await page.getByRole("button", { name: "Analyze Repo Into Design" }).click();
  await expect(page.getByText("Staged Plan Tree")).toBeVisible();

  await expect(page.getByTestId("draft-node-draft-product-calculator")).toBeVisible();
  await expect(page.getByText("Interactive Planner").first()).toBeVisible();
  await expect(page.getByText("Repository Intelligence").first()).toBeVisible();
});

test("planner voice commands can select, switch views, and apply the staged design", async ({ page }) => {
  await page.goto("/");
  await page.getByLabel("Planner product").selectOption("example-product-calculator");

  await page.getByTestId("planner-input").fill("Design input, history, reporting, and expression modules for this product");
  await page.getByTestId("planner-send").click();

  await expect(page.getByTestId("draft-node-draft-product-calculator")).toBeVisible();

  await page.evaluate(async () => {
    await window.__ARUVI_E2E__?.runPlannerVoiceTranscript?.("select module calculation history");
  });
  await expect(page.getByTestId("draft-node-rename-input")).toHaveValue("Calculation History");

  await page.evaluate(async () => {
    await window.__ARUVI_E2E__?.runPlannerVoiceTranscript?.("view conversation");
  });
  await expect(page.getByText("Switched back to the planner conversation.")).toBeVisible();

  await page.evaluate(async () => {
    await window.__ARUVI_E2E__?.runPlannerVoiceTranscript?.("apply design");
  });
  await expect(page.getByText("Applied design to catalog.").first()).toBeVisible();
});
