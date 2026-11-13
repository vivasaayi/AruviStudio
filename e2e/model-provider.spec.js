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
