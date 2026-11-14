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
    name: "Claude test", providerType: "openai_compatible", protocol: "anthropic", completionProfile: "standard", baseUrl: "https://api.anthropic.com",
  });
});

test("desktop creates and edits the reasoning provider profile", async ({ page }) => {
  await page.addInitScript(() => {
    const original = window.__ARUVI_E2E__.invoke;
    window.__providerCalls = [];
    window.__ARUVI_E2E__.invoke = async (command, args) => {
      if (command === "list_providers") {
        const providers = await original(command, args);
        return providers.map((provider) => provider.id === "provider-deepseek-hosted" ? { ...provider, completion_profile: "openai_reasoning", protocol: "openai_compatible" } : provider);
      }
      if (command === "create_provider" || command === "update_provider") {
        window.__providerCalls.push({ command, args });
        return { id: args.id || "reasoning-fixture", name: args.name, provider_type: args.providerType,
          protocol: args.protocol, completion_profile: args.completionProfile, base_url: args.baseUrl,
          auth_secret_ref: null, enabled: true, created_at: "", updated_at: "" };
      }
      return original(command, args);
    };
  });
  await page.goto("/models");
  await page.getByRole("button", { name: "+ Add Provider" }).click();
  await page.getByLabel("Provider name").fill("Reasoning test");
  await page.getByLabel("Provider type").selectOption("openai_reasoning");
  await expect(page.getByText(/do not support temperature/)).toBeVisible();
  await page.getByLabel("Provider base URL").fill("https://example.invalid/v1");
  await page.getByRole("button", { name: "Add Provider", exact: true }).click();
  await expect.poll(() => page.evaluate(() => window.__providerCalls[0])).toMatchObject({ command: "create_provider", args: {
    providerType: "openai_compatible", protocol: "openai_compatible", completionProfile: "openai_reasoning",
  } });
  const card = page.getByText("DeepSeek (Hosted)", { exact: true }).first().locator("..");
  await card.getByRole("button", { name: "Edit", exact: true }).click();
  const selection = page.locator("select").filter({ has: page.locator('option[value="openai_reasoning"]') }).last();
  await expect(selection).toHaveValue("openai_reasoning");
  await page.getByRole("button", { name: "Save Changes" }).click();
  await expect.poll(() => page.evaluate(() => window.__providerCalls[1])).toMatchObject({ command: "update_provider", args: {
    id: "provider-deepseek-hosted", providerType: "openai_compatible", protocol: "openai_compatible", completionProfile: "openai_reasoning",
  } });
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

test("desktop speech preset sends its pinned source and checksum", async ({ page }) => {
  await page.goto("/models");
  await page.evaluate(() => {
    const original = window.__ARUVI_E2E__.invoke;
    window.__installRequest = null;
    window.__ARUVI_E2E__.invoke = async (command, args) => {
      if (command === "install_managed_local_model_command") {
        window.__installRequest = args.request;
        throw new Error("Synthetic install stopped");
      }
      return original(command, args);
    };
  });
  const card = page.getByText("Whisper Tiny English", { exact: true }).locator("..");
  await card.getByRole("button", { name: "Install & Register" }).click();
  await expect.poll(() => page.evaluate(() => window.__installRequest)).toMatchObject({
    expected_sha256: "921e4cf8686fdd993dcd081a5da5b6c365bfde1162e72b08d75ac75289920b1f",
    download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1/ggml-tiny.en.bin?download=true",
  });
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
