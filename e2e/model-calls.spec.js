import { test, expect } from "./support/test.js";

test("desktop loads older model calls and refresh returns to latest", async ({ page }) => {
  await page.addInitScript(() => {
    const original = window.__ARUVI_E2E__.invoke;
    window.__callPageRequests = [];
    const first = Array.from({ length: 100 }, (_, i) => ({
      id: `call-${String(999 - i).padStart(3, "0")}`, source_kind: "desktop_chat", source_id: "shared", session_id: "shared", source_label: "Current", model_name: "fixture", provider_name: "Fixture", status: "completed", call_index: 100 - i, created_at: "2026-01-02 00:00:00",
    }));
    window.__ARUVI_E2E__.invoke = async (command, args) => {
      if (command !== "list_model_calls") return original(command, args);
      window.__callPageRequests.push(args);
      if (!args.beforeId) return first;
      if (window.__callPageRequests.filter((item) => item.beforeId).length === 1) throw new Error("fixture failure");
      return [{ ...first[0], id: "old-call", session_id: "older", source_label: "Older call", created_at: "2026-01-01 00:00:00" }];
    };
  });
  await page.goto("/calls");
  await page.getByRole("button", { name: "Load older calls" }).click();
  await expect(page.getByRole("alert")).toContainText("fixture failure");
  await expect(page.getByText(/100 loaded.*auto-refresh paused/)).toBeVisible();
  await page.getByRole("button", { name: "Load older calls" }).click();
  await expect(page.getByText("Older call").first()).toBeVisible();
  await expect(page.getByText(/101 loaded.*auto-refresh paused/)).toBeVisible();
  const requests = await page.evaluate(() => window.__callPageRequests.filter((item) => item.beforeId));
  expect(requests).toHaveLength(2);
  expect(requests[0]).toMatchObject({ beforeCreatedAt: "2026-01-02 00:00:00", beforeId: "call-900" });
  await page.getByRole("button", { name: "Refresh", exact: true }).click();
  await expect(page.getByText(/100 loaded · filter applies to loaded calls/)).toBeVisible();
  await expect(page.getByText("Older call", { exact: true })).toHaveCount(0);
});
