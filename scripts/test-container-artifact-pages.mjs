import { chromium } from "@playwright/test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { closeSync, openSync, readFileSync, writeSync } from "node:fs";

const input = JSON.parse(readFileSync(0, "utf8"));
const browser = await chromium.launch({ executablePath: process.env.ARUVI_CHROME_PATH || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome", headless: true });
try {
  const page = await browser.newPage();
  await page.goto(input.base + "/remote");
  await page.locator("#tokenInput").fill(input.token);
  await page.locator("#saveButton").click();
  await page.getByRole("button", { name: "Work", exact: true }).click();
  await page.locator("#workProductSelect").selectOption(input.product);
  await page.locator("button.work-card").filter({ hasText: "Artifact fixture" }).click();
  const card = page.getByRole("button", { name: "Show artifact large", exact: true }).locator("..");
  await card.getByRole("button", { name: "Read in pages", exact: true }).click();
  await card.getByText("Bytes 0–65536 of 2097153", { exact: true }).waitFor();
  assert.equal((await card.locator("pre").textContent()).length, 65536);
  await card.getByRole("button", { name: "Next page", exact: true }).click();
  await card.getByText("Bytes 65536–131072 of 2097153", { exact: true }).waitFor();
  await card.getByRole("button", { name: "Previous page", exact: true }).click();
  await card.getByText("Bytes 0–65536 of 2097153", { exact: true }).waitFor();
  if (input.filePath) {
    const fd = openSync(input.filePath, "r+");
    try { writeSync(fd, "y", 0); } finally { closeSync(fd); }
  } else execFileSync("docker", ["exec", input.container, "sh", "-c", "printf y | dd of=/data/artifacts/large.txt bs=1 seek=0 conv=notrunc 2>/dev/null"]);
  await card.getByRole("button", { name: "Next page", exact: true }).click();
  await card.getByText(/Artifact changed; restart reading from the first page/).waitFor();
  console.log("Browser paged a large artifact and rejected a changed version.");
} finally { await browser.close(); }
