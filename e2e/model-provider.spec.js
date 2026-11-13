import { test, expect } from "./support/test.js";

test("desktop provider setup sends Anthropic protocol", async ({ page }) => {
  await page.goto("/models");
  await page.evaluate(() => {
    const original = window.__ARUVI_E2E__.invoke;
    window.__anthropicCreate = null;
    window.__ARUVI_E2E__.invoke = async (command, args) => {
      if (command === "create_provider") {
        window.__anthropicCreate = args;
        return { id: "test", name: args.name, protocol: args.protocol, provider_type: args.providerType,
          base_url: args.baseUrl, auth_secret_ref: null, enabled: true, created_at: "", updated_at: "" };
      }
      return original(command, args);
    };
  });
  await page.getByRole("button", { name: "+ Add Provider" }).click();
  await page.getByLabel("Provider name").fill("Claude test");
  await page.getByLabel("Provider type").selectOption("anthropic");
  await page.getByLabel("Provider base URL").fill("https://api.anthropic.com");
  await page.getByRole("button", { name: "Add Provider", exact: true }).click();
  await expect.poll(() => page.evaluate(() => window.__anthropicCreate)).toMatchObject({
    name: "Claude test", providerType: "openai_compatible", protocol: "anthropic", baseUrl: "https://api.anthropic.com",
  });
});

test("desktop discovers and selects paged provider models", async ({ page }) => {
  await page.goto("/models");
  await page.evaluate(() => {
    const original = window.__ARUVI_E2E__.invoke;
    window.__ARUVI_E2E__.invoke = async (command, args) => {
      if (command === "discover_provider_models") {
        return args.cursor
          ? { models: [{ id: "second-model", display_name: "Second", metadata: {} }], next_cursor: null }
          : { models: [{ id: "first-model", display_name: "First", metadata: {} }], next_cursor: "first-model" };
      }
      return original(command, args);
    };
  });
  await page.getByRole("button", { name: "+ Add Model" }).click();
  await page.getByLabel("Provider for model").selectOption("provider-deepseek-hosted");
  await page.getByRole("button", { name: "Discover available models" }).click();
  await page.getByLabel("Available model").selectOption("first-model");
  await expect(page.getByLabel("Model name", { exact: true })).toHaveValue("first-model");
  await page.getByRole("button", { name: "Load next page" }).click();
  await page.getByLabel("Available model").selectOption("second-model");
  await expect(page.getByLabel("Model name", { exact: true })).toHaveValue("second-model");
});

test("desktop editing sends the selected provider protocol", async ({ page }) => {
  await page.goto("/models");
  await page.evaluate(() => {
    const original = window.__ARUVI_E2E__.invoke;
    window.__providerUpdate = null;
    window.__ARUVI_E2E__.invoke = async (command, args) => {
      if (command === "update_provider") {
        window.__providerUpdate = args;
        return { id: args.id, name: args.name, provider_type: args.providerType, protocol: args.protocol,
          base_url: args.baseUrl, auth_secret_ref: null, enabled: true, created_at: "", updated_at: "" };
      }
      return original(command, args);
    };
  });
  const card = page.getByText("DeepSeek (Hosted)", { exact: true }).first().locator("..");
  await card.getByRole("button", { name: "Edit", exact: true }).click();
  await page.locator("select").filter({ has: page.locator('option[value="anthropic"]') }).last().selectOption("anthropic");
  await page.getByRole("button", { name: "Save Changes" }).click();
  await expect.poll(() => page.evaluate(() => window.__providerUpdate)).toMatchObject({
    id: "provider-deepseek-hosted", providerType: "openai_compatible", protocol: "anthropic",
  });
});
