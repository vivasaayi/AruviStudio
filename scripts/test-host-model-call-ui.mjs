// Exercise read-only model-call history in the running worktree's browser UI.
import { chromium } from "@playwright/test";
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";

const plan = JSON.parse(execFileSync("bash", ["scripts/env", "worktree", "plan"], { encoding: "utf8" }));
const envFile = `${plan.state}/environment.env`;
const line = readFileSync(envFile, "utf8").split("\n").find((item) => item.startsWith("ARUVI_MOBILE_API_TOKEN="));
if (!line) throw new Error("Mobile token is not configured");
const token = line.slice("ARUVI_MOBILE_API_TOKEN=".length);
const address = execFileSync("docker", ["compose", "--project-name", plan.project, "--project-directory", process.cwd(), "--env-file", envFile, "-f", "deploy/compose.worktree.yaml", "port", "app", "8787"], { encoding: "utf8" }).trim();
if (!address.startsWith("127.0.0.1:")) throw new Error("Unexpected worktree address");
const browser = await chromium.launch({ executablePath: process.env.ARUVI_CHROME_PATH || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome", headless: true });
try {
  const page = await browser.newPage();
  await page.goto(`http://${address}/remote`);
  await page.locator("#tokenInput").fill(token);
  await page.locator("#saveButton").click();
  await page.getByRole("button", { name: "Activity", exact: true }).click();
  await page.getByRole("button", { name: "Load older calls" }).waitFor({ state: "visible" });
  const response = page.waitForResponse((item) => item.url().includes("/api/mobile/model-calls?") && item.url().includes("before_id="));
  await page.getByRole("button", { name: "Load older calls" }).click();
  const olderResponse = await response;
  if (!olderResponse.ok()) throw new Error("Older page request failed");
  const older = await olderResponse.json();
  if (!Array.isArray(older) || older.length < 1) throw new Error("Older page was empty");
  await page.waitForFunction((count) => document.getElementById("modelCallSummary").textContent.includes(`${100 + count} loaded · auto-refresh paused`), older.length);
  await page.locator("#refreshModelCallsButton").click();
  await page.waitForFunction(() => document.getElementById("modelCallSummary").textContent.includes("100 loaded · filter applies to loaded calls"));
  console.log(`Live browser model-call history loaded ${older.length} older records and returned to the latest page.`);
} finally {
  await browser.close();
}
