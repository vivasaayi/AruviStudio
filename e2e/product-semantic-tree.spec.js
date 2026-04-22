import { test, expect } from "./support/test.js";

test("product workspace supports semantic tree navigation and book deep links", async ({ page }) => {
  await page.goto("/products");

  await expect(page.getByRole("heading", { name: "Product Workspace" })).toBeVisible();
  await expect(page.getByPlaceholder("Search nodes")).toBeVisible();
  await expect(page.getByRole("button", { name: /^Book$/ })).toBeVisible();
  await expect(page.getByRole("button", { name: /^Structure$/ })).toBeVisible();
  await expect(page.getByRole("button", { name: /^Delivery$/ })).toBeVisible();

  await page.getByText("Expression Evaluation").first().click();
  await expect(page.getByText("Selected Node", { exact: true })).toBeVisible();
  await expect(page.getByText("Children for the selected capability are listed below.")).toBeVisible();
  await expect(page.getByRole("button", { name: "Edit Capability" })).toBeVisible();

  await page.getByRole("button", { name: /^Delivery$/ }).click();
  await expect(page.getByText("Owner Scope")).toBeVisible();
  await expect(page.getByText("Implement expression precedence resolution")).toBeVisible();

  await page.getByRole("button", { name: /^Book$/ }).click();
  await page.getByRole("button", { name: "Open In Book" }).click();

  await expect(page).toHaveURL(/\/product-overview#capability-calc-expression-evaluation$/);
  await expect(page.getByRole("heading", { name: "Product Overview" })).toBeVisible();
  await expect(page.locator("#capability-calc-expression-evaluation")).toBeVisible();
  await expect(page.locator("#capability-calc-expression-evaluation")).toContainText("Expression Evaluation");
});

test("work item workspace shows semantic owner badges and product-level ownership", async ({ page }) => {
  await page.goto("/work-items");

  await expect(page.getByRole("heading", { name: "Work Item Workspace" })).toBeVisible();
  await expect(page.getByText("Publish keyboard shortcuts guide")).toBeVisible();
  await expect(page.getByText("Owner: Product", { exact: true })).toBeVisible();
  await expect(page.getByText("Owner: Capability", { exact: true })).toBeVisible();
  await expect(page.getByText("Calculator / Core Math Engine / Expression Evaluation", { exact: true })).toBeVisible();
  await expect(page.getByText("Owner: Rollout", { exact: true })).toBeVisible();
  await expect(page.getByText("Calculator / Core Math Engine / Expression Evaluation / Scientific Mode Rollout", { exact: true })).toBeVisible();
});
