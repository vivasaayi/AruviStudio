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
  await expect(page.getByTestId("draft-node-add-child-type")).toHaveValue("work_item");
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
  const confirmation = await page.evaluate(() => window.__ARUVI_E2E__.getInvokeCalls("confirm_planner_plan_command"));
  expect(confirmation).toHaveLength(1);
  expect(confirmation[0].args.productId).toBe("example-product-calculator");

  await page.getByTestId("nav-products").click();
  await expect(page.getByRole("heading", { name: "Products" })).toBeVisible();
});

test("planner retains its draft when a catalog conflict prevents confirmation", async ({ page }) => {
  await page.goto("/");
  await page.getByLabel("Planner product").selectOption("example-product-calculator");
  await page.getByTestId("planner-input").fill("Design input and history product areas");
  await page.getByTestId("planner-send").click();
  await page.getByTestId("planner-view-draft").click();
  await expect(page.getByTestId("draft-node-draft-product-calculator")).toBeVisible();
  await page.evaluate(() => window.__ARUVI_E2E__.setPlannerConfirmFailure(true));
  await page.getByTestId("draft-commit").click();
  await expect(page.getByText(/Catalog node changed; review the draft/).first()).toBeVisible();
  await page.getByTestId("planner-view-draft").click();
  await expect(page.getByTestId("draft-node-draft-product-calculator")).toBeVisible();
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
    await window.__ARUVI_E2E__?.runPlannerVoiceTranscript?.("view conversation.");
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
      if (command === "planner_relevant_actions_command") {
        if (args.node?.label === "Input Experience") window.fixturePlannerAncestors = args.node.ancestors;
        return args.node?.label === "Input Experience"
          ? [{ type: "update_product_area", target: { productAreaName: "Input Experience" }, fields: { description: "shared action" } }] : [];
      }
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
  await expect.poll(() => page.evaluate(() => window.fixturePlannerAncestors)).toEqual([{ label: "Calculator", node_type: "product" }]);
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

test("product overview stages the selected existing chapter before enabling Planner", async ({ page }) => {
  await page.goto("/products");
  await page.getByRole("button", { name: "Product Overview", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Product Book" })).toBeVisible();
  await page.evaluate(() => {
    const bridge = window.__ARUVI_E2E__, original = bridge.invoke;
    window.__stageCalls = [];
    bridge.invoke = (command, args) => {
      if (command !== "stage_planner_catalog_node_command") return original(command, args);
      window.__stageCalls.push(args);
      return new Promise((resolve) => { window.__finishStage = () => resolve({status:"session_update", assistant_message:"Staged", selected_draft_node_id:args.nodeId, draft_tree_nodes:[{id:args.nodeId,label:"Existing chapter",node_type:"product_area",children:[]}], execution_lines:[],execution_errors:[],trace_events:[]}); });
    };
  });
  await page.getByRole("button", { name: "Improve Chapter", exact: true }).first().click();
  await expect.poll(() => page.evaluate(() => window.__stageCalls.length)).toBe(1);
  await expect(page.getByTestId("planner-send")).toBeDisabled();
  await expect(page.getByLabel("Planner product")).toBeDisabled();
  await expect(page.getByLabel("Planner model")).toBeDisabled();
  const staged = await page.evaluate(() => window.__stageCalls[0]);
  expect(staged).toMatchObject({productId:"example-product-calculator",nodeType:"product_area"});
  expect(staged.nodeId).toBeTruthy();
  await page.evaluate(() => window.__finishStage());
  await expect(page.getByTestId("planner-input")).toHaveValue(/Enhance product area/);
  await expect(page.getByTestId("planner-send")).toBeEnabled();
  await page.getByTestId("planner-send").click();
  await expect.poll(() => page.evaluate(() => window.__ARUVI_E2E__.getInvokeCalls("submit_planner_turn_command").length)).toBe(1);
  const turn = await page.evaluate(() => window.__ARUVI_E2E__.getInvokeCalls("submit_planner_turn_command")[0].args);
  expect(turn.selectedDraftNodeId).toBe(staged.nodeId);
});

test("product overview does not prepare an unsafe prompt when staging fails", async ({ page }) => {
  await page.goto("/products");
  await page.getByRole("button", { name: "Product Overview", exact: true }).click();
  await page.evaluate(() => {
    const bridge = window.__ARUVI_E2E__, original = bridge.invoke;
    bridge.invoke = (command, args) => command === "stage_planner_catalog_node_command" ? Promise.reject(new Error("Catalog changed")) : original(command, args);
  });
  await page.getByRole("button", { name: "Improve Chapter", exact: true }).first().click();
  await expect(page.getByText("Catalog changed").first()).toBeVisible();
  await expect(page.getByTestId("planner-input")).toHaveValue("");
});

test("late desktop catalog staging does not navigate away from a newer page", async ({ page }) => {
  await page.goto("/products");
  await page.getByRole("button", { name: "Product Overview", exact: true }).click();
  await page.evaluate(() => {
    const bridge = window.__ARUVI_E2E__, original = bridge.invoke;
    bridge.invoke = (command, args) => command !== "stage_planner_catalog_node_command" ? original(command, args) : new Promise((resolve) => {
      window.__finishStage = () => resolve({status:"session_update", assistant_message:"Staged", selected_draft_node_id:args.nodeId, draft_tree_nodes:[{id:args.nodeId,label:"Existing chapter",node_type:"product_area",children:[]}], execution_lines:[],execution_errors:[],trace_events:[]});
    });
  });
  await page.getByRole("button", { name: "Improve Chapter", exact: true }).first().click();
  await expect.poll(() => page.evaluate(() => typeof window.__finishStage)).toBe("function");
  await page.getByTestId("nav-products").click();
  await expect(page).toHaveURL(/\/products$/);
  await page.evaluate(() => window.__finishStage());
  await expect(page.getByRole("heading", { name: "Products" })).toBeVisible();
  await expect(page).toHaveURL(/\/products$/);
});

test("desktop catalog staging preserves text typed while the request is pending", async ({ page }) => {
  await page.goto("/products");
  await page.getByRole("button", { name: "Product Overview", exact: true }).click();
  await page.evaluate(() => {
    const bridge = window.__ARUVI_E2E__, original = bridge.invoke;
    bridge.invoke = (command, args) => command !== "stage_planner_catalog_node_command" ? original(command, args) : new Promise((resolve) => {
      window.__finishStage = () => resolve({status:"session_update", assistant_message:"Staged", selected_draft_node_id:args.nodeId, draft_tree_nodes:[{id:args.nodeId,label:"Existing chapter",node_type:"product_area",children:[]}], execution_lines:[],execution_errors:[],trace_events:[]});
    });
  });
  await page.getByRole("button", { name: "Improve Chapter", exact: true }).first().click();
  await expect.poll(() => page.evaluate(() => typeof window.__finishStage)).toBe("function");
  await page.getByTestId("planner-input").fill("Keep my request");
  await page.evaluate(() => window.__finishStage());
  await expect(page.getByTestId("planner-input")).toHaveValue("Keep my request");
  await expect(page.getByText("The selected item is staged. Your unsent request was preserved.").first()).toBeVisible();
  await expect(page.getByTestId("planner-send")).toBeEnabled();
});

test("failed desktop Planner session creation does not leave staging locked", async ({ page }) => {
  await page.goto("/products");
  await page.getByRole("button", { name: "Product Overview", exact: true }).click();
  await page.evaluate(() => {
    const bridge = window.__ARUVI_E2E__, original = bridge.invoke;
    bridge.invoke = (command, args) => command === "create_planner_session_command" ? Promise.reject(new Error("Session unavailable")) : original(command, args);
  });
  await page.getByRole("button", { name: "Improve Chapter", exact: true }).first().click();
  await expect(page.getByText(/Session unavailable/).first()).toBeVisible();
  await expect(page.getByLabel("Planner product")).toBeEnabled();
  await expect(page.getByTestId("planner-input")).toHaveValue("");
});

test("product overview stages the product root before preparing its Planner request", async ({ page }) => {
  await page.goto("/products");
  await page.getByRole("button", { name: "Product Overview", exact: true }).click();
  await page.evaluate(() => {
    const bridge = window.__ARUVI_E2E__, original = bridge.invoke;
    window.__stageCalls = [];
    bridge.invoke = (command, args) => {
      if (command !== "stage_planner_catalog_node_command") return original(command, args);
      window.__stageCalls.push(args);
      return Promise.resolve({status:"session_update", assistant_message:"Staged", selected_draft_node_id:args.nodeId, draft_tree_nodes:[{id:args.nodeId,label:"Calculator",node_type:"product",children:[]}], execution_lines:[],execution_errors:[],trace_events:[]});
    };
  });
  await page.getByRole("button", { name: "Improve Book", exact: true }).click();
  await expect(page.getByTestId("planner-input")).toHaveValue(/Enhance product/);
  const staged=await page.evaluate(() => window.__stageCalls[0]);
  expect(staged).toMatchObject({productId:"example-product-calculator",nodeType:"product",nodeId:"example-product-calculator"});
  await page.getByTestId("planner-send").click();
  await expect.poll(() => page.evaluate(() => window.__ARUVI_E2E__.getInvokeCalls("submit_planner_turn_command").length)).toBe(1);
  const turn=await page.evaluate(() => window.__ARUVI_E2E__.getInvokeCalls("submit_planner_turn_command")[0].args);
  expect(turn.selectedDraftNodeId).toBe(staged.nodeId);
});

test("desktop no-session packet delegates its snapshot to the backend", async ({ page }) => {
  await page.goto("/");
  await page.evaluate(async () => {
    const harness = await import("/e2e/support/planner-snapshot-harness.tsx");
    harness.mountPlannerSnapshotHarness();
    window.__ARUVI_E2E__.clearInvokeCalls();
  });
  await page.getByRole("button", { name: "Export snapshot fixture" }).click();
  await expect(page.getByTestId("snapshot-path")).toContainText("design-review-packet.html");
  const calls = await page.evaluate(() => ({ render: window.__ARUVI_E2E__.getInvokeCalls("render_planner_packet_snapshot_command"), save: window.__ARUVI_E2E__.getInvokeCalls("export_product_overview_html") }));
  expect(calls.render).toHaveLength(1);
  expect(calls.render[0].args.snapshot.title).toBe("<Inventory>");
  expect(calls.save[0].args.html).toContain("caller-supplied snapshot");
  expect(calls.save[0].args.html).toContain("&lt;Inventory&gt;");
});
