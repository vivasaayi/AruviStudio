import { test, expect } from "./support/test.js";

const routes = [
  { key: "planner", path: "/planner", heading: null },
  { key: "products", path: "/products", heading: "Product Workspace" },
  { key: "work-items", path: "/work-items", heading: "Work Item Workspace" },
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
