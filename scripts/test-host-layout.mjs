// Read-only layout smoke check against this worktree's running app on macOS.
import { chromium } from "@playwright/test";
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";

const plan = JSON.parse(execFileSync("bash", ["scripts/env", "worktree", "plan"], { encoding: "utf8" }));
const envFile = `${plan.state}/environment.env`;
const tokenLine = readFileSync(envFile, "utf8").split("\n").find((line) => line.startsWith("ARUVI_MOBILE_API_TOKEN="));
if (!tokenLine) throw new Error("Worktree mobile token is not configured");
const token = tokenLine.slice("ARUVI_MOBILE_API_TOKEN=".length);
const port = execFileSync("docker", ["compose", "--project-name", plan.project, "--project-directory", process.cwd(), "--env-file", envFile, "-f", "deploy/compose.worktree.yaml", "port", "app", "8787"], { encoding: "utf8" }).trim();
const browser = await chromium.launch({ executablePath: process.env.ARUVI_CHROME_PATH || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome", headless: true });
const sections = ["Planner", "Products", "Workspaces", "Work", "Chat", "Voice", "Models", "Agents", "Teams", "Skills", "Routing", "CI", "Activity"];
const problems = [];
try {
  for (const viewport of [{ width: 390, height: 844 }, { width: 1280, height: 900 }]) {
    const context = await browser.newContext({ viewport });
    try {
      const page = await context.newPage();
      page.on("pageerror", (error) => problems.push(`${viewport.width}: ${error.message}`));
      await page.goto(`http://${port}/remote`);
      await page.locator("#tokenInput").fill(token);
      await page.locator("#saveButton").click();
      for (const name of sections) {
        await page.getByRole("button", { name, exact: true }).click();
        await page.locator(`#tab-${name.toLowerCase()}`).waitFor({ state: "visible" });
        const width = await page.evaluate(() => ({ scroll: document.documentElement.scrollWidth, client: document.documentElement.clientWidth }));
        if (width.scroll > width.client + 1) problems.push(`${viewport.width} ${name}: horizontal overflow ${width.scroll - width.client}px`);
      }
      console.log(`${viewport.width}x${viewport.height}: checked ${sections.length} sections`);
    } finally { await context.close(); }
  }
} finally { await browser.close(); }
if (problems.length) { console.error(problems.join("\n")); process.exitCode = 1; }
else console.log("No page errors or document-level horizontal overflow.");
