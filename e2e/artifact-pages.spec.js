import { test, expect } from "@playwright/test";

test("desktop artifact modal pages and reports a changed artifact", async ({ page }) => {
  await page.goto("/e2e/support/artifact-modal.html");
  await page.getByRole("button", { name: "Read in pages" }).click();
  await expect(page.getByText("first page", { exact: true })).toBeVisible();
  await expect(page.getByText("Bytes 0–10 of 21")).toBeVisible();
  await page.getByRole("button", { name: "Next page" }).click();
  await expect(page.getByText("second page", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Previous page" }).click();
  await expect(page.getByText("first page", { exact: true })).toBeVisible();
  await page.evaluate(() => window.changeArtifactVersion());
  await page.getByRole("button", { name: "Next page" }).click();
  await expect(page.getByRole("alert")).toContainText("Artifact changed");
  await page.getByRole("button", { name: "Read in pages" }).click();
  await expect(page.getByRole("alert")).toHaveCount(0);
});
