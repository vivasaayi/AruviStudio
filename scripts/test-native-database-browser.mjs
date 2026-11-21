import { chromium, expect } from "@playwright/test";
import { createInterface } from "node:readline";

const { ARUVI_TEST_DB_CHROME: executablePath, ARUVI_TEST_DB_TOKEN: token } = process.env;
if (!executablePath || !token) throw new Error("Missing disposable browser fixture configuration");
const browser = await chromium.launch({executablePath, headless:true});
const page = await browser.newPage();
page.setDefaultTimeout(15000);
page.on("dialog", dialog => void dialog.accept());
const input = createInterface({input:process.stdin, crlfDelay:Infinity});
let origin = null;
try {
  for await (const line of input) {
    try {
      const command = JSON.parse(line);
      if (command.action === "close") { process.stdout.write('{"ok":true}\n'); break; }
      if (command.action === "open") {
        if (origin === null) {
          origin = command.base;
          await page.goto(origin + "/remote");
          await page.locator("#tokenInput").fill(token);
          await page.locator("#saveButton").click();
        } else {
          if (command.base !== origin) throw new Error("Browser origin changed during restart");
          await page.reload();
          await expect(page.locator("#tokenInput")).toHaveValue(token);
        }
        await page.getByRole("button", {name:"Routing", exact:true}).click();
        await expect(page.locator("#databaseStatus")).toContainText("Active: " + command.active);
        await expect(page.locator("#databaseStatus")).toContainText(command.environment ? "environment controls" : "saved override applies after restart");
        await expect(page.locator("#databaseOverride")).toHaveValue(command.override || "");
      } else if (command.action === "save" || command.action === "clear") {
        if (command.action === "save") await page.locator("#databaseOverride").fill(command.path);
        const response = page.waitForResponse(result => result.url().endsWith("/api/mobile/database-settings") && result.request().method() === "POST");
        await page.locator(command.action === "save" ? "#saveDatabaseOverride" : "#clearDatabaseOverride").click();
        if (!(await response).ok()) throw new Error("Database settings update failed");
        await expect(page.locator("#databaseStatus")).toContainText("Active: " + command.active);
        await expect(page.locator("#databaseOverride")).toHaveValue(command.action === "save" ? command.path : "");
      } else throw new Error("Unknown browser fixture command");
      process.stdout.write('{"ok":true}\n');
    } catch (error) {
      process.stdout.write(JSON.stringify({ok:false, error:error.message || String(error)}) + "\n");
    }
  }
} finally {
  await browser.close();
}
