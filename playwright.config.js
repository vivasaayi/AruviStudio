import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  fullyParallel: true,
  retries: process.env.CI ? 2 : 0,
  outputDir: "reports/playwright/artifacts",
  reporter: [
    ["list"],
    ["html", { outputFolder: "reports/playwright/html", open: "never" }],
    ["junit", { outputFile: "reports/playwright/junit.xml" }],
  ],
  use: {
    baseURL: "http://127.0.0.1:4173",
    trace: "on",
    screenshot: "on",
    video: "on",
    viewport: { width: 1600, height: 1000 },
  },
  webServer: {
    command: "npm run dev -- --host 127.0.0.1 --port 4173",
    url: "http://127.0.0.1:4173",
    reuseExistingServer: !process.env.CI,
    timeout: 120000,
  },
});
