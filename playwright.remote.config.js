import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e-remote",
  reporter: "list",
  use: {
    baseURL: process.env.ARUVI_REMOTE_URL || "http://127.0.0.1:8787",
    browserName: "chromium",
    launchOptions: process.env.ARUVI_CHROME_PATH ? { executablePath: process.env.ARUVI_CHROME_PATH } : {},
    actionTimeout: 10000,
    viewport: { width: 1440, height: 900 },
    trace: "retain-on-failure",
  },
});
