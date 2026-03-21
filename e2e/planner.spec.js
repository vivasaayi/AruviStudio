import { test, expect } from "./support/test.js";

test("planner supports deterministic create, refine, and commit flow", async ({ page }) => {
  await page.goto("/");

  await expect(page.getByRole("heading", { name: "Interactive Planner" })).toBeVisible();

  await page.getByTestId("planner-input").fill("I want to build a hotel management system");
  await page.getByTestId("planner-send").click();

  await page.getByTestId("planner-view-draft").click();
  await expect(page.getByTestId("draft-node-draft-product-hotel-management-system")).toBeVisible();
  await expect(page.getByTestId("draft-node-draft-module-reservations-booking")).toBeVisible();
  await expect(page.getByTestId("draft-node-draft-module-guest-management")).toBeVisible();

  await page.getByTestId("draft-node-draft-product-hotel-management-system").click();
  await page.getByTestId("draft-node-rename-input").fill("Boutique Hotel Management System");
  await page.getByTestId("draft-node-rename-save").click();
  await expect(page.getByText("Boutique Hotel Management System").first()).toBeVisible();

  await page.getByTestId("draft-node-add-child-type").selectOption("module");
  await page.getByTestId("draft-node-add-child-name").fill("Concierge Experience");
  await page.getByTestId("draft-node-add-child-summary").fill("Handle concierge requests and premium guest experiences.");
  await page.getByTestId("draft-node-add-child-save").click();
  await expect(page.getByText("Concierge Experience").first()).toBeVisible();

  await page.getByTestId("draft-node-delete").click();
  await expect(page.getByTestId("draft-node-draft-module-concierge-experience")).toHaveCount(0);

  await page.getByText("Boutique Hotel Management System").first().click();
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

  await page.getByTestId("draft-commit").click();
  await expect(page.getByText("Committed draft plan.")).toBeVisible();

  await page.getByTestId("nav-products").click();
  await expect(page.getByRole("heading", { name: "Product Workspace" })).toBeVisible();
  await expect(page.getByText("Boutique Hotel Management System").first()).toBeVisible();
});
