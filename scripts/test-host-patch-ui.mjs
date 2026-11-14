// Run inside the disposable fixture created by test-host-file-copilot.
import assert from "node:assert/strict";
import { chromium } from "@playwright/test";

let input = "";
for await (const chunk of process.stdin) input += chunk;
const { address, provider, model } = JSON.parse(input);
const browser = await chromium.launch({ executablePath: process.env.ARUVI_CHROME_PATH || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome", headless: true });
try {
  const page = await browser.newPage();
  await page.goto(`http://${address}/remote`);
  await page.locator("#tokenInput").fill("workflow-mobile-test");
  await page.locator("#providerInput").fill(provider);
  await page.locator("#modelInput").fill(model);
  await page.locator("#saveButton").click();
  await page.getByRole("button", { name: "Workspaces", exact: true }).click();
  const row = page.locator("#repositoryList .tool-card").filter({ hasText: "File fixture" });
  await row.getByRole("button", { name: "Browse workspace" }).click();
  await page.locator("#repositoryTree").getByRole("button", { name: "math.js" }).click();
  await page.getByLabel("Selected file contents").waitFor();
  await page.getByLabel("Selected file contents").fill("function unsavedAnswerFortyTwo() { return 42; }\n");
  await page.getByLabel("Copilot question").fill("Name the function defined in this file. Reply with its exact name.");
  await page.getByRole("button", { name: "Ask about unsaved file" }).click();
  await page.locator("#status").getByText("Copilot reply ready", { exact: true }).waitFor({ timeout: 120000 });
  assert.match(await page.locator("#repositoryCopilotResult").textContent(), /unsavedAnswerFortyTwo/i);
  assert.equal(await page.getByRole("button", { name: "Propose changes" }).isDisabled(), true);
  await page.getByRole("button", { name: "Discard file edits" }).click();
  assert.match(await page.getByLabel("Selected file contents").inputValue(), /multiplyBySeven/);
  console.log("Browser discussed an unsaved file with the real coding model without saving it.");
  await page.getByLabel("Stream Copilot response").check();
  await page.getByLabel("Copilot question").fill("Change the function to multiply by 9 instead of 7. Keep its name. Return a single patch for math.js.");
  await page.getByRole("button", { name: "Propose changes" }).click();
  const review = page.locator("#repositoryPatchReview");
  await review.getByText("math.js", { exact: false }).first().waitFor({ timeout: 120000 });
  assert.match(await review.textContent(), /\* 9/);
  assert.match(await page.getByLabel("Selected file contents").inputValue(), /\* 7/);
  await page.getByRole("button", { name: "Apply reviewed changes" }).click();
  await page.locator("#status").getByText("Applied 1 reviewed patch(es)", { exact: true }).waitFor({ timeout: 120000 });
  await page.waitForFunction(() => document.querySelector("#repositoryFileContent").value.includes("* 9"), null, { timeout: 120000 });
  console.log("Browser reviewed and applied the real coding-model proposal.");
} finally { await browser.close(); }
