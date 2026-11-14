import { test, expect } from "./support/test.js";

test("desktop DeepSeek presets select currently documented model IDs", async ({ page }) => {
  await page.goto("/models");
  await page.getByRole("button", { name: "+ Add Model" }).click();
  await page.getByRole("button", { name: "DeepSeek Flash Preset" }).click();
  await expect(page.getByLabel("Model name")).toHaveValue("deepseek-flash");
  await page.getByRole("button", { name: "DeepSeek V4 Pro Preset" }).click();
  await expect(page.getByLabel("Model name")).toHaveValue("deepseek-v4-pro");
  await page.getByRole("button", { name: "DeepSeek Transcription Preset" }).click();
  await expect(page.getByLabel("Model name")).toHaveValue("deepseek-voice-transcription");
});

test("desktop settings offers an untagged DeepSeek transcription selector", async ({ page }) => {
  await page.addInitScript(() => {
    const original = window.__ARUVI_E2E__.invoke;
    window.__ARUVI_E2E__.invoke = async (command, args) => {
      if (command === "list_model_definitions") {
        const models = await original(command, args);
        return [...models, { id: "deepseek-voice", provider_id: "provider-deepseek-hosted", name: "deepseek-voice-transcription", context_window: null, capability_tags: [], notes: "", enabled: true, created_at: "", updated_at: "" }];
      }
      return original(command, args);
    };
  });
  await page.goto("/settings");
  await page.getByLabel("Speech provider").selectOption("provider-deepseek-hosted");
  const model = page.getByLabel("Speech model");
  await expect(model.locator('option[value="deepseek-voice-transcription"]')).toHaveCount(1);
  await model.selectOption("deepseek-voice-transcription");
  await expect(model).toHaveValue("deepseek-voice-transcription");
});

test("desktop settings preserves an unregistered saved speech selector while models load", async ({ page }) => {
  await page.addInitScript(() => {
    const original = window.__ARUVI_E2E__.invoke;
    window.__ARUVI_E2E__.invoke = async (command, args) => {
      if (command === "get_setting" && args.key === "speech.transcription_provider_id") return "provider-deepseek-hosted";
      if (command === "get_setting" && args.key === "speech.transcription_model_name") return "deepseek-voice-transcription";
      if (command === "list_model_definitions") {
        await new Promise((resolve) => setTimeout(resolve, 500));
        const models = await original(command, args);
        window.__speechModelsLoaded = true;
        return models;
      }
      return original(command, args);
    };
  });
  await page.goto("/settings");
  const model = page.getByLabel("Speech model");
  await expect(model).toHaveValue("deepseek-voice-transcription");
  await expect.poll(() => page.evaluate(() => window.__speechModelsLoaded)).toBe(true);
  await expect(model).toHaveValue("deepseek-voice-transcription");
  await expect(model.locator('option[value="deepseek-voice-transcription"]')).toHaveText("deepseek-voice-transcription (not registered)");
  await expect(page.getByLabel("Speech provider")).toHaveValue("provider-deepseek-hosted");
  await page.getByLabel("Speech provider").selectOption("");
  await expect(model).toHaveValue("");
});

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
          ? { models: [{ id: "second-model", display_name: "Second", metadata: { task: { unexpected: true } } }], next_cursor: null }
          : { models: [{ id: "first-model", display_name: "First", metadata: { task: "transcription" } }], next_cursor: "first-model" };
      }
      return original(command, args);
    };
  });
  await page.getByRole("button", { name: "+ Add Model" }).click();
  await page.getByLabel("Provider for model").selectOption("provider-deepseek-hosted");
  await page.getByRole("button", { name: "Discover available models" }).click();
  await expect(page.getByLabel("Available model").locator('option[value="first-model"]')).toHaveText("First (first-model) — provider task: transcription");
  await page.getByLabel("Available model").selectOption("first-model");
  await expect(page.getByLabel("Model name", { exact: true })).toHaveValue("first-model");
  await page.getByRole("button", { name: "Load next page" }).click();
  await expect(page.getByLabel("Available model").locator('option[value="second-model"]')).toHaveText("Second (second-model)");
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

test("desktop agent model selector marks local Whisper as transcription only", async ({ page }) => {
  await page.addInitScript(() => {
    const original = window.__ARUVI_E2E__.invoke;
    window.__ARUVI_E2E__.invoke = async (command, args) => {
      if (command === "list_providers") return [...await original(command, args), { id: "local-speech", name: "Local speech", provider_type: "local_runtime", enabled: true }];
      if (command === "list_model_definitions") return [...await original(command, args), { id: "whisper-fixture", provider_id: "local-speech", name: "Whisper fixture", enabled: true, capability_tags: ["speech_to_text"] }, { id: "deepseek-voice-fixture", provider_id: "provider-deepseek-hosted", name: "deepseek-voice-transcription", enabled: true, capability_tags: [] }];
      return original(command, args);
    };
  });
  await page.goto("/agents");
  const selector = page.getByLabel("Primary model");
  await expect(selector).toBeEnabled();
  await expect(selector.locator('option[value="whisper-fixture"]')).toBeDisabled();
  await expect(selector.locator('option[value="whisper-fixture"]')).toHaveText("Whisper fixture · transcription only");
  await expect(selector.locator('option[value="deepseek-voice-fixture"]')).toBeDisabled();
  await expect(selector.locator('option[value="model-deepseek-chat"]')).toBeEnabled();
});


test("desktop provider key fields are masked and clear on close", async ({ page }) => {
  await page.goto("/models");
  await page.getByRole("button", { name: "+ Add Provider" }).click();
  const createKey = page.getByLabel("API Key / Secret Ref (optional)");
  await expect(createKey).toHaveAttribute("type", "password");
  await createKey.fill("synthetic-key");
  await page.getByRole("button", { name: "Close", exact: true }).click();
  await page.getByRole("button", { name: "+ Add Provider" }).click();
  await expect(createKey).toHaveValue("");
  await page.getByRole("button", { name: "Close", exact: true }).click();
  const card = page.getByText("DeepSeek (Hosted)", { exact: true }).first().locator("..");
  await card.getByRole("button", { name: "Edit", exact: true }).click();
  const rotateKey = page.getByLabel("Rotate API Key (optional)");
  await expect(rotateKey).toHaveAttribute("type", "password");
  await rotateKey.fill("synthetic-key");
  await page.getByRole("button", { name: "Close", exact: true }).click();
  await card.getByRole("button", { name: "Edit", exact: true }).click();
  await expect(rotateKey).toHaveValue("");
});


test("desktop Model Host preset requests an endpoint without assuming a port", async ({ page }) => {
  await page.goto("/models");
  await page.getByRole("button", { name: "Use Aruvi Model Host" }).click();
  await expect(page.getByLabel("Provider name")).toHaveValue("Aruvi Model Host");
  await expect(page.getByLabel("Provider type")).toHaveValue("openai_compatible");
  await expect(page.getByLabel("Provider base URL")).toHaveValue("");
  await expect(page.getByText(/ports can change/)).toBeVisible();
  await expect(page.getByLabel("API Key / Secret Ref (optional)")).toHaveAttribute("type", "password");
});
