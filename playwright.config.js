import { defineConfig } from "@playwright/test";

const reportRoot = process.env.ARUVI_PLAYWRIGHT_REPORT_DIR || "reports/playwright";
const port = process.env.ARUVI_PLAYWRIGHT_PORT || "4173";
if (!/^\d+$/.test(port) || Number(port) < 1 || Number(port) > 65535) throw new Error("ARUVI_PLAYWRIGHT_PORT must be a valid port number");
const lightweight = process.env.ARUVI_PLAYWRIGHT_LIGHTWEIGHT === "1";

export default defineConfig({
  testDir: "./e2e",
  fullyParallel: true,
  retries: process.env.CI ? 2 : 0,
  outputDir: `${reportRoot}/artifacts`,
  reporter: [
    ["list"],
    ["html", { outputFolder: `${reportRoot}/html`, open: "never" }],
    ["junit", { outputFile: `${reportRoot}/junit.xml` }],
  ],
  use: {
    baseURL: `http://127.0.0.1:${port}`,
    trace: lightweight ? "off" : "on",
    screenshot: lightweight ? "off" : "on",
    video: lightweight ? "off" : "on",
    ...(process.env.ARUVI_PLAYWRIGHT_EXECUTABLE_PATH ? { launchOptions: { executablePath: process.env.ARUVI_PLAYWRIGHT_EXECUTABLE_PATH } } : {}),
    viewport: { width: 1600, height: 1000 },
  },
  webServer: {
    command: `npm run dev -- --host 127.0.0.1 --port ${port} --strictPort`,
    url: `http://127.0.0.1:${port}`,
    reuseExistingServer: !process.env.CI && process.env.ARUVI_PLAYWRIGHT_REUSE_SERVER !== "0",
    timeout: 120000,
  },
});
