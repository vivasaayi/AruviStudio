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
      } else if (command.action === "verify-conflict") {
        await page.locator("#databaseOverride").fill(command.draft);
        const other = await page.request.post(origin + "/api/mcp", {
          headers: {Authorization:`Bearer ${token}`, Accept:"application/json, text/event-stream"},
          data: {jsonrpc:"2.0", id:1, method:"tools/call", params:{name:"aruvi_settings", arguments:{action:"set_database_path_override", arguments:{db_path:command.external}}}},
        });
        expect((await other.json()).result.isError).not.toBeTruthy();
        const conflict = page.waitForResponse(response => response.url().endsWith("/api/mobile/database-settings") && response.request().method() === "POST");
        await page.locator("#saveDatabaseOverride").click();
        expect((await conflict).status()).toBe(409);
        await expect(page.locator("#databaseOverride")).toHaveValue(command.draft);
        await page.getByRole("button", {name:"Discard draft and reload"}).click();
        await expect(page.locator("#databaseOverride")).toHaveValue(command.external);
        await page.locator("#clearDatabaseOverride").click();
        await expect(page.locator("#databaseOverride")).toHaveValue("");
      } else if (command.action === "verify-draft") {
        let seen, release;
        const observed = new Promise(resolve => { seen = resolve; });
        const gate = new Promise(resolve => { release = resolve; });
        const pattern = "**/api/mobile/database-settings";
        const handler = async route => {
          if (route.request().method() !== "GET") return route.continue();
          const response = await route.fetch();
          const body = await response.json(); body.health.applied_migrations = 43210;
          seen(); await gate; await route.fulfill({response, json:body});
        };
        await page.route(pattern, handler);
        let timer;
        try {
          if (command.preexisting) await page.locator("#databaseOverride").fill(command.path);
          await page.locator("#databaseStatus").evaluate(element => { element.textContent = "Waiting for delayed refresh"; });
          await page.locator("#refreshDatabaseSettings").click();
          await Promise.race([observed, new Promise((_, reject) => { timer = setTimeout(() => reject(new Error("Database refresh was not requested")), 15000); })]);
          if (!command.preexisting) await page.locator("#databaseOverride").fill(command.path);
          const response = page.waitForResponse(result => result.url().endsWith("/api/mobile/database-settings") && result.request().method() === "GET");
          release();
          await response;
          await expect(page.locator("#databaseStatus")).toContainText("Migrations: 43210");
          await expect(page.locator("#databaseOverride")).toHaveValue(command.path);
        } finally { clearTimeout(timer); release(); await page.unroute(pattern, handler); }
      } else if (command.action === "verify-save-draft") {
        let seen, release, timer;
        const observed = new Promise(resolve => { seen = resolve; });
        const gate = new Promise(resolve => { release = resolve; });
        const pattern = "**/api/mobile/database-settings";
        const handler = async route => {
          if (route.request().method() !== "POST") return route.continue();
          const response = await route.fetch(); seen(); await gate; await route.fulfill({response});
        };
        await page.route(pattern, handler);
        try {
          await page.locator("#databaseOverride").fill(command.path);
          await page.locator("#saveDatabaseOverride").click();
          await Promise.race([observed, new Promise((_, reject) => { timer = setTimeout(() => reject(new Error("Database save was not requested")), 15000); })]);
          await page.locator("#databaseOverride").fill(command.draft);
          const refreshed = page.waitForResponse(result => result.url().endsWith("/api/mobile/database-settings") && result.request().method() === "GET");
          await page.locator("#databaseStatus").evaluate(element => { element.textContent = "Waiting for save refresh"; });
          release(); await refreshed;
          await expect(page.locator("#databaseStatus")).toContainText("Migrations:");
          await expect(page.locator("#databaseOverride")).toHaveValue(command.draft);
        } finally { clearTimeout(timer); release(); await page.unroute(pattern, handler); }
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
      const status = await page.locator("#databaseStatus").textContent().catch(() => "unavailable");
      const value = await page.locator("#databaseOverride").inputValue().catch(() => "unavailable");
      process.stdout.write(JSON.stringify({ok:false, error:(error.message || String(error)) + " | database status: " + status + " | override input: " + value}) + "\n");
    }
  }
} finally {
  await browser.close();
}
