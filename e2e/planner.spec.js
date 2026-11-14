import { test, expect } from "./support/test.js";

test("planner supports deterministic create, refine, packet, and apply flow", async ({ page }) => {
  await page.goto("/");

  await expect(page.getByTestId("planner-input")).toBeVisible();
  await page.getByLabel("Planner product").selectOption("example-product-calculator");

  await page.getByTestId("planner-input").fill("Design input, history, reporting, and expression product areas for this product");
  await page.getByTestId("planner-send").click();

  await page.getByTestId("planner-view-draft").click();
  await expect(page.getByTestId("draft-node-draft-product-calculator")).toBeVisible();
  await expect(page.getByTestId("draft-node-draft-product_area-input-experience")).toBeVisible();
  await expect(page.getByTestId("draft-node-draft-product_area-calculation-history")).toBeVisible();

  await page.getByTestId("draft-node-draft-product_area-input-experience").click();
  await page.getByTestId("draft-node-rename-input").fill("Input Workspace");
  await page.getByTestId("draft-node-rename-save").click();
  await expect(page.getByText("Input Workspace").first()).toBeVisible();

  await page.getByTestId("draft-node-draft-product-calculator").click();
  await page.getByTestId("draft-node-add-child-type").selectOption("product_area");
  await page.getByTestId("draft-node-add-child-name").fill("Concierge Experience");
  await page.getByTestId("draft-node-add-child-summary").fill("Handle concierge requests and premium guest experiences.");
  await page.getByTestId("draft-node-add-child-save").click();
  await expect(page.getByText("Concierge Experience").first()).toBeVisible();

  await page.getByTestId("draft-node-delete").click();
  await expect(page.getByTestId("draft-node-draft-product_area-concierge-experience")).toHaveCount(0);

  await page.getByTestId("draft-node-draft-product-calculator").click();
  await page.getByTestId("planner-input").fill("Add email and WhatsApp notifications to this product");
  await page.getByTestId("planner-send").click();
  await expect(page.getByTestId("draft-node-draft-product_area-notifications-messaging")).toBeVisible();

  await page.getByTestId("draft-node-draft-product_area-notifications-messaging").click();
  await page.getByTestId("planner-input").fill("Enhance this product area with guest notification preferences and outbound delivery tracking");
  await page.getByTestId("planner-send").click();
  await expect(page.getByTestId("draft-node-draft-capability-outbound-delivery-tracking")).toBeVisible();

  await page.getByTestId("draft-node-draft-capability-outbound-delivery-tracking").click();
  await page.getByTestId("planner-input").fill("Add work items to implement this capability");
  await page.getByTestId("planner-send").click();
  await expect(page.getByTestId("draft-node-draft-work_item-implement-delivery-audit-timeline")).toBeVisible();

  await page.getByTestId("draft-node-draft-work_item-implement-delivery-audit-timeline").click();
  await page.getByTestId("planner-input").fill("Revise this work item to include WhatsApp consent capture");
  await page.getByTestId("planner-send").click();
  await expect(page.getByText("Implement Delivery Audit Timeline and Consent Handling").first()).toBeVisible();

  await page.getByRole("button", { name: "Generate Packet" }).click();
  await expect(page.getByText("Packet exported to /tmp/aruvi-e2e/")).toBeVisible();
  const packetCalls = await page.evaluate(() => ({ build: window.__ARUVI_E2E__.getInvokeCalls("build_planner_review_packet_command"), save: window.__ARUVI_E2E__.getInvokeCalls("export_product_overview_html") }));
  expect(packetCalls.build).toHaveLength(1);
  expect(packetCalls.build[0].args.selectedNodeId).toBe("draft-work_item-implement-delivery-audit-timeline");
  expect(packetCalls.save[0].args.html).toContain("Shared packet fixture");

  await page.getByTestId("draft-commit").click();
  await expect(page.getByText("Applied design to catalog.").first()).toBeVisible();

  await page.getByTestId("nav-products").click();
  await expect(page.getByRole("heading", { name: "Products" })).toBeVisible();
});

test("planner can reverse engineer a registered repository into a design tree", async ({ page }) => {
  await page.goto("/");

  await expect(page.getByTestId("planner-input")).toBeVisible();
  await page.getByLabel("Planner product").selectOption("example-product-calculator");
  await page.getByRole("button", { name: "Reverse engineer repository" }).click();
  await expect(page.getByText("Reverse Engineer Repository")).toBeVisible();

  await page.getByPlaceholder("/absolute/path/to/repository").fill("/tmp/aruvi-studio");
  await page.getByRole("button", { name: "Register Repo" }).click();
  await expect(page.getByText('Registered repository "aruvi-studio".')).toBeVisible();

  await page.getByRole("button", { name: "Analyze Repo Into Design" }).click();
  await expect(page.getByText("Staged Design Tree")).toBeVisible();

  await expect(page.getByTestId("draft-node-draft-product-calculator")).toBeVisible();
  await expect(page.getByText("Interactive Planner").first()).toBeVisible();
  await expect(page.getByText("Repository Intelligence").first()).toBeVisible();
});

test("planner voice commands can select, switch views, and apply the staged design", async ({ page }) => {
  await page.goto("/");
  await page.getByLabel("Planner product").selectOption("example-product-calculator");

  await page.getByTestId("planner-input").fill("Design input, history, reporting, and expression product areas for this product");
  await page.getByTestId("planner-send").click();

  await expect(page.getByTestId("draft-node-draft-product-calculator")).toBeVisible();

  await page.evaluate(async () => {
    await window.__ARUVI_E2E__?.runPlannerVoiceTranscript?.("select product area calculation history");
  });
  await expect(page.getByTestId("draft-node-rename-input")).toHaveValue("Calculation History");

  await page.evaluate(async () => {
    await window.__ARUVI_E2E__?.runPlannerVoiceTranscript?.("view conversation");
  });
  await expect(page.getByText("Switched back to the planner conversation.")).toBeVisible();

  await page.evaluate(async () => {
    await window.__ARUVI_E2E__?.runPlannerVoiceTranscript?.("apply design");
  });
  await expect(page.getByText("Applied design to catalog.").first()).toBeVisible();
});

test("planner displays validation returned by the shared native command", async ({ page }) => {
  await page.goto("/");
  await page.evaluate(() => {
    const bridge = window.__ARUVI_E2E__;
    const original = bridge.invoke;
    bridge.invoke = (command, args) => command === "validate_planner_tree_command"
      ? { score: 47, counts: { product: 1, "product area": 0, capability: 0, "work item": 0 }, issues: [{ tone: "warn", title: "Shared service validation", detail: `Validated ${args.nodes.length} roots` }] }
      : original(command, args);
  });
  await page.getByLabel("Planner product").selectOption("example-product-calculator");
  await page.getByTestId("planner-input").fill("Design input, history, reporting, and expression product areas for this product");
  await page.getByTestId("planner-send").click();
  await page.getByTestId("planner-view-draft").click();
  await expect(page.getByText("Shared service validation")).toBeVisible();
  await expect(page.getByText("Validated 1 roots")).toBeVisible();
});

test("planner uses suggested prompts from the shared native command", async ({ page }) => {
  await page.goto("/");
  await page.evaluate(() => {
    const bridge = window.__ARUVI_E2E__;
    const original = bridge.invoke;
    bridge.invoke = (command, args) => {
      if (command === "planner_suggested_prompts_command") return [`Shared suggestion for ${args.node?.label}`];
      if (command === "planner_relevant_actions_command") return args.node?.label === "Input Experience"
        ? [{ type: "update_product_area", target: { productAreaName: "Input Experience" }, fields: { description: "shared action" } }] : [];
      return original(command, args);
    };
  });
  await page.getByLabel("Planner product").selectOption("example-product-calculator");
  await page.getByTestId("planner-input").fill("Design input, history, reporting, and expression product areas for this product");
  await page.getByTestId("planner-send").click();
  await page.getByTestId("planner-view-draft").click();
  await page.getByTestId("draft-node-draft-product_area-input-experience").click();
  await page.getByRole("button", { name: "Shared suggestion for Input Experience" }).click();
  await expect(page.getByTestId("planner-input")).toHaveValue("Shared suggestion for Input Experience");
  await expect(page.getByText("Update product area Input Experience")).toBeVisible();
});

test("planner reports microphone setup errors and releases the audio track", async ({ page }) => {
  await page.addInitScript(() => {
    window.fixtureMicrophoneStops = 0;
    window.fixtureContextCloses = 0;
    Object.defineProperty(navigator, "mediaDevices", { configurable: true, value: {
      getUserMedia: async () => ({ getTracks: () => [{ stop() { window.fixtureMicrophoneStops += 1; } }] }),
    } });
    window.AudioContext = class {
      async resume() { throw new Error("Microphone setup failed"); }
      async close() { window.fixtureContextCloses += 1; }
    };
  });
  await page.goto("/");
  await page.getByLabel("Planner product").selectOption("example-product-calculator");
  await page.getByRole("button", { name: "Start Voice Input" }).click();
  await expect(page.getByRole("alert")).toHaveText("Microphone setup failed");
  expect(await page.evaluate(() => window.fixtureMicrophoneStops)).toBe(1);
  expect(await page.evaluate(() => window.fixtureContextCloses)).toBe(1);
  await expect(page.getByRole("button", { name: "Start Voice Input" })).toBeEnabled();
});
