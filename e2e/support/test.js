import path from "node:path";
import { fileURLToPath } from "node:url";
import { test as base, expect } from "@playwright/test";

const currentDir = path.dirname(fileURLToPath(import.meta.url));
const mockTauriPath = path.join(currentDir, "mock-tauri.js");

export const test = base.extend({
  page: async ({ page }, use) => {
    await page.addInitScript({ path: mockTauriPath });
    await use(page);
  },
});

export { expect };
