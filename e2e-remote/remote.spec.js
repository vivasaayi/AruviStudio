import { test, expect } from "@playwright/test";

const token = process.env.ARUVI_MOBILE_API_TOKEN;
const mcpToken = process.env.ARUVI_MCP_API_TOKEN;

test("remote UI connects and opens each operational section", async ({ page }) => {
  test.skip(!token, "Set ARUVI_MOBILE_API_TOKEN for a running remote server");
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto("/remote");
  await page.locator("#tokenInput").fill(token);
  await page.locator("#saveButton").click();
  await page.locator("#healthButton").click();
  await expect(page.locator("#status")).toHaveText("Connected");

  for (const name of ["Products", "Workspaces", "Work", "Chat", "Voice", "Models", "Agents", "Teams", "Skills", "Routing", "CI", "Activity", "Planner"]) {
    await page.getByRole("button", { name, exact: true }).click();
    await expect(page.locator(`#tab-${name.toLowerCase()}`)).toBeVisible();
  }
  expect(errors).toEqual([]);
});

test("planner sends its explicit product scope", async ({ page }) => {
  await page.addInitScript(() => { localStorage.setItem("aruvi.remote.token", "synthetic-token"); localStorage.setItem("aruvi.remote.selected_product", "product-a"); localStorage.setItem("aruvi.remote.selected_node", "old-node"); });
  await page.route("**/api/mobile/products", (route) => route.fulfill({ json: [{ id: "product-a", name: "Product A" }, { id: "product-b", name: "Product B" }] }));
  await page.route("**/api/mobile/planner/sessions", (route) => route.fulfill({ json: { session_id: "session-1" } }));
  const turns = [];
  await page.route("**/api/mobile/planner/sessions/*/turn", async (route) => { turns.push(route.request().postDataJSON()); await route.fulfill({ json: { session_id: "session-1", assistant_message: "Ready", draft_tree_nodes: [] } }); });
  await page.goto("/remote");
  await expect(page.locator("#plannerProductSelect")).toBeEnabled();
  await expect(page.locator("#plannerProductSelect")).toHaveValue("product-a");
  await page.locator("#plannerProductSelect").selectOption("product-b");
  await page.locator("#composer").fill("Plan this product");
  let turn = page.waitForResponse((response) => response.url().endsWith("/turn"));
  await page.locator("#sendButton").click(); await turn;
  expect(turns[0].product_id).toBe("product-b");
  expect(turns[0].selected_draft_node_id).toBeNull();
  await page.locator("#plannerProductSelect").selectOption("");
  await page.locator("#composer").fill("Plan without a product");
  turn = page.waitForResponse((response) => response.url().endsWith("/turn"));
  await page.locator("#sendButton").click(); await turn;
  expect(turns[1].product_id).toBeNull();
});

test("planner locks product and draft selection while a turn is pending", async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem("aruvi.remote.token", "synthetic-token"));
  await page.route("**/api/mobile/products", (route) => route.fulfill({ json: [{ id: "product-a", name: "Product A" }] }));
  await page.route("**/api/mobile/planner/sessions", (route) => route.fulfill({ json: { session_id: "session-1" } }));
  let releaseTurn;
  let notifyTurn;
  const turnReceived = new Promise((resolve) => { notifyTurn = resolve; });
  let holdTurn = false;
  const turns = [];
  await page.route("**/api/mobile/planner/sessions/*/turn", async (route) => {
    turns.push(route.request().postDataJSON());
    if (holdTurn) await new Promise((resolve) => { releaseTurn = resolve; notifyTurn(); });
    await route.fulfill({ json: { session_id: "session-1", assistant_message: "Ready", draft_tree_nodes: [{ id: "node-1", label: "Draft node" }] } });
  });
  await page.goto("/remote");
  await expect(page.locator("#plannerProductSelect")).toBeEnabled();
  await page.locator("#plannerProductSelect").selectOption("product-a");
  await page.locator("#composer").fill("Create a draft");
  await page.locator("#sendButton").click();
  await page.getByRole("button", { name: "Draft node" }).click();
  holdTurn = true;
  await page.locator("#composer").fill("Refine the draft");
  await page.locator("#sendButton").click();
  try {
    await Promise.race([turnReceived, new Promise((_, reject) => setTimeout(() => reject(new Error("Planner request did not arrive")), 5000))]);
    await expect(page.locator("#plannerProductSelect")).toBeDisabled();
    await expect(page.locator("#refreshPlannerProductsButton")).toBeDisabled();
    await expect(page.getByRole("button", { name: "Draft node" })).toBeDisabled();
    expect(turns[1]).toMatchObject({ product_id: "product-a", selected_draft_node_id: "node-1" });
  } finally {
    if (releaseTurn) releaseTurn();
  }
  await expect(page.locator("#plannerProductSelect")).toBeEnabled();
  await expect(page.getByRole("button", { name: "Draft node" })).toBeEnabled();
});

test("planner offers draft rename, add and delete controls", async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem("aruvi.remote.token", "synthetic-token"));
  await page.route("**/api/mobile/products", (route) => route.fulfill({ json: [] }));
  await page.route("**/api/mobile/planner/sessions", (route) => route.fulfill({ json: { session_id: "draft-session" } }));
  await page.route("**/api/mobile/planner/sessions/*/turn", (route) => route.fulfill({ json: { session_id: "draft-session", assistant_message: "Ready", draft_tree_nodes: [{ id: "root", label: "Product", node_type: "product" }] } }));
  const edits = [];
  await page.route("**/api/mobile/planner/sessions/*/draft/*", (route) => {
    const action = route.request().url().split("/").pop();
    edits.push({ action, body: route.request().postDataJSON() });
    const label = action === "rename" ? "Renamed" : "Product";
    const children = action === "add" ? [{ id: "child", label: "Area", node_type: "product_area" }] : [];
    return route.fulfill({ json: { session_id: "draft-session", assistant_message: "Updated", selected_draft_node_id: action === "add" ? "child" : "root", draft_tree_nodes: [{ id: "root", label, node_type: "product", children }] } });
  });
  await page.goto("/remote");
  await expect(page.locator("#plannerProductSelect")).toBeEnabled();
  await page.locator("#composer").fill("Create draft");
  await page.locator("#sendButton").click();
  await page.locator("#draftTree button").first().click();
  await page.getByLabel("Draft node name").fill("Renamed");
  await page.getByRole("button", { name: "Rename selected" }).click();
  await expect(page.locator("#draftTree")).toContainText("Renamed");
  await page.getByLabel("Draft node name").fill("Area");
  await page.getByRole("button", { name: "Add child" }).click();
  await expect(page.locator("#draftTree")).toContainText("Area");
  page.once("dialog", (dialog) => dialog.accept());
  await page.getByRole("button", { name: "Delete selected" }).click();
  await expect.poll(() => edits.length).toBe(3);
  expect(edits.map((edit) => edit.action)).toEqual(["rename", "add", "delete"]);
  expect(edits[2].body.node_id).toBe("child");
});

test("planner shows selected node evidence and honors server-cleared selection", async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem("aruvi.remote.token", "synthetic-token"));
  await page.route("**/api/mobile/products", (route) => route.fulfill({ json: [] }));
  await page.route("**/api/mobile/planner/sessions", (route) => route.fulfill({ json: { session_id: "details-session" } }));
  await page.route("**/api/mobile/planner/suggestions", async (route) => {
    const node = route.request().postDataJSON().node;
    if (node.label === "Area") await new Promise((resolve) => setTimeout(resolve, 300));
    await route.fulfill({ json: [`Prompt for ${node.label}`] });
  });
  const requests = [];
  await page.route("**/api/mobile/planner/sessions/*/turn", (route) => {
    requests.push(route.request().postDataJSON());
    return route.fulfill({ json: { session_id: "details-session", assistant_message: "Ready", selected_draft_node_id: null,
      draft_tree_nodes: [{ id: "root", label: "Product", node_type: "product", children: [{ id: "child", label: "Area", node_type: "product_area", summary: "Details", source: "repository", confidence: "high", evidence: ["<script>unsafe</script>"] }] }] } });
  });
  await page.goto("/remote");
  await expect(page.locator("#plannerProductSelect")).toBeEnabled();
  async function send(text) { await page.locator("#composer").fill(text); await page.locator("#sendButton").click(); }
  await send("Create draft");
  const stale = page.waitForResponse((response) => response.url().endsWith("/planner/suggestions") && response.request().postDataJSON().node.label === "Area");
  await page.locator("#draftTree button").filter({ hasText: "Area" }).click();
  await page.locator("#draftTree button").first().click();
  await stale;
  await expect(page.locator("#draftNodeDetails")).toContainText("Prompt for Product");
  await expect(page.locator("#draftNodeDetails")).not.toContainText("Prompt for Area");
  await page.locator("#draftTree button").filter({ hasText: "Area" }).click();
  const details = page.locator("#draftNodeDetails");
  await expect(details).toContainText("Path: Product / Area");
  await expect(details).toContainText("Source: repository");
  await expect(details).toContainText("Confidence: high");
  await expect(details).toContainText("Evidence: <script>unsafe</script>");
  await expect(details.locator("script")).toHaveCount(0);
  await send("Clear selection");
  await expect(details).toContainText("Select a draft node");
  expect(requests[1].selected_draft_node_id).toBe("child");
  expect(await page.evaluate(() => localStorage.getItem("aruvi.remote.selected_node"))).toBeNull();
  await send("Continue");
  await expect.poll(() => requests.length).toBe(3);
  expect(requests[2].selected_draft_node_id).toBeNull();
});

test("browser edits a real planner draft through the authenticated API", async ({ page, request }) => {
  test.skip(!token || !mcpToken || !process.env.ARUVI_TEST_REPOSITORY_PATH, "Requires an isolated fixture");
  const headers = { Authorization: `Bearer ${token}` };
  const created = await request.post("/api/mobile/products", { headers, data: { name: `Planner fixture ${Date.now()}` } });
  expect(created.status()).toBe(201);
  const product = await created.json();
  try {
    await page.goto("/remote");
    await page.locator("#tokenInput").fill(token);
    await page.locator("#saveButton").click();
    await expect(page.locator("#plannerProductSelect option").filter({ hasText: product.name })).toHaveCount(1);
    await page.locator("#plannerProductSelect").selectOption(product.id);
    await page.locator("#composer").fill("Show the current plan");
    await page.locator("#sendButton").click();
    await expect(page.locator("#draftTree")).toContainText(product.name);
    await page.locator("#draftTree button").first().click();
    await page.getByLabel("Draft node name").fill("Renamed draft");
    const renameResponse = page.waitForResponse((response) => response.url().endsWith("/draft/rename"));
    await page.getByRole("button", { name: "Rename selected" }).click();
    const renamed = await renameResponse;
    expect(renamed.status(), await renamed.text()).toBe(200);
    await expect(page.locator("#draftTree")).toContainText("Renamed draft");
    await page.getByLabel("Draft node name").fill("Draft area");
    await page.getByLabel("New child summary (optional)").fill("Reviewable design details fixture");
    await page.getByRole("button", { name: "Add child" }).click();
    await expect(page.locator("#draftTree")).toContainText("Draft area");
    await expect(page.locator("#draftNodeDetails")).toContainText("Reviewable design details fixture");
    await expect(page.locator("#draftNodeDetails")).toContainText("Renamed draft / Draft area");
    const suggestionPayload = { node: { label: "Draft area", node_type: "product_area", meta: null } };
    expect((await request.post("/api/mobile/planner/suggestions", { data: suggestionPayload })).status()).toBe(401);
    const suggested = await (await request.post("/api/mobile/planner/suggestions", { headers, data: suggestionPayload })).json();
    const mcpSuggested = await request.post("/api/mcp", { headers: { Authorization: `Bearer ${mcpToken}` }, data: { jsonrpc: "2.0", id: 3, method: "tools/call", params: { name: "aruvi_planner", arguments: { action: "suggest_planner_prompts", arguments: suggestionPayload } } } });
    expect((await mcpSuggested.json()).result.structuredContent.result).toEqual(suggested);
    expect(suggested[0]).toBe("Enhance Draft area with 3 concrete capabilities.");
    await page.getByRole("button", { name: suggested[0], exact: true }).click();
    await expect(page.locator("#composer")).toHaveValue(suggested[0]);
    await expect(page.locator("#draftNodeDetails")).toContainText("create_product_area");
    const matchPayload = { ...suggestionPayload, actions: [{ type: "create_product_area", name: "Draft area" }, { type: "create_product_area", name: "Other" }] };
    expect((await request.post("/api/mobile/planner/match-actions", { data: matchPayload })).status()).toBe(401);
    const matched = await (await request.post("/api/mobile/planner/match-actions", { headers, data: matchPayload })).json();
    expect(matched).toEqual([matchPayload.actions[0]]);
    const mcpMatch = await request.post("/api/mcp", { headers: { Authorization: `Bearer ${mcpToken}` }, data: { jsonrpc: "2.0", id: 4, method: "tools/call", params: { name: "aruvi_planner", arguments: { action: "match_planner_actions", arguments: matchPayload } } } });
    expect((await mcpMatch.json()).result.structuredContent.result).toEqual(matched);
    await page.getByRole("button", { name: "Validate design" }).click();
    await expect(page.locator("#draftValidation")).toContainText("Readiness: 88/100");
    await expect(page.locator("#draftValidation")).toContainText("Draft area is empty");
    const packetDownload = page.waitForEvent("download");
    await page.getByRole("button", { name: "Download review packet" }).click();
    const packet = await packetDownload;
    expect(packet.suggestedFilename()).toContain("design-review-packet.html");
    const packetHtml = await (await packet.createReadStream()).toArray();
    const packetText = Buffer.concat(packetHtml).toString("utf8");
    expect(packetText).toContain("Draft area");
    expect(packetText).toContain("Reviewable design details fixture");
    expect(packetText).toContain("draftNodes");
    expect(packetText).toContain("absence does not imply deletion");
    expect(packetText).toContain("Approval Section");
    const review = await page.context().newPage();
    await review.setContent(packetText);
    await expect(review.locator('svg[aria-label="Staged design hierarchy"]')).toBeVisible();
    await expect(review.getByRole("link", { name: "3. Proposed Architecture" })).toBeVisible();
    await expect(review.locator("#section-3 ul").getByText("Draft area").first()).toBeVisible();
    await review.close();
    page.once("dialog", (dialog) => dialog.accept());
    await page.getByRole("button", { name: "Delete selected" }).click();
    await expect(page.locator("#draftTree")).not.toContainText("Draft area");
    await expect(page.locator("#draftValidation")).toContainText("Validate again");
    const unchanged = await request.get(`/api/mobile/products/${product.id}/tree`, { headers });
    expect(JSON.stringify(await unchanged.json())).toContain(product.name);
  } finally {
    await request.post("/api/mcp", { headers: { Authorization: `Bearer ${mcpToken}` }, data: { jsonrpc: "2.0", id: 1, method: "tools/call", params: { name: "aruvi_catalog", arguments: { action: "archive_product", arguments: { id: product.id } } } } });
  }
});

test("CI view reports an offline service and renders service data safely", async ({ page, request }) => {
  test.skip(!token, "Set ARUVI_MOBILE_API_TOKEN for a running remote server");
  expect((await request.get("/api/mobile/ci/status")).status()).toBe(401);
  expect((await request.get("/api/mobile/ci/enqueue", { headers: { Authorization: `Bearer ${token}` } })).status()).toBe(404);
  await page.goto("/remote");
  await page.locator("#tokenInput").fill(token);
  await page.locator("#saveButton").click();
  await page.getByRole("button", { name: "CI", exact: true }).click();
  await expect(page.locator("#ciStatus")).toContainText("CI unavailable");
  await page.route("**/api/mobile/ci/*", async (route) => {
    const kind = new URL(route.request().url()).pathname.split("/").pop();
    const value = kind === "status" ? { status: "online" } : kind === "targets" ? [{ id: "<img src=x onerror=alert(1)>" }] : [{ id: 7, status: "passed" }];
    await route.fulfill({ json: value });
  });
  await page.locator("#refreshCiButton").click();
  await expect(page.locator("#ciStatus")).toHaveText("Service: online");
  await expect(page.locator("#ciTargets")).toContainText("<img src=x onerror=alert(1)>");
  await expect(page.locator("#ciTargets img")).toHaveCount(0);
  await expect(page.locator("#ciRuns")).toContainText("passed");
});

test("browser registers and attaches a container workspace", async ({ page, request }) => {
  test.skip(!token || !mcpToken || !process.env.ARUVI_TEST_REPOSITORY_PATH, "Set tokens and a disposable Git repository path inside the container");
  async function call(name, action, arguments_) {
    const response = await request.post("/api/mcp", {
      headers: { Authorization: `Bearer ${mcpToken}`, Accept: "application/json, text/event-stream" },
      data: { jsonrpc: "2.0", id: 1, method: "tools/call", params: { name, arguments: { action, arguments: arguments_ } } },
    });
    const body = await response.json();
    expect(body.result.isError, JSON.stringify(body.result)).not.toBeTruthy();
    return body.result.structuredContent.result;
  }
  const name = `Workspace product ${Date.now()}`;
  const product = await call("aruvi_catalog", "create_product", { name });
  const area = await call("aruvi_catalog", "create_product_area", { product_id: product.id, name: "Workspace test area" });
  const otherProduct = await call("aruvi_catalog", "create_product", { name: `Other ${name}` });
  const otherArea = await call("aruvi_catalog", "create_product_area", { product_id: otherProduct.id, name: "Other area" });
  const copilotProvider = process.env.ARUVI_TEST_MODEL_URL ? await call("aruvi_models", "create_provider", { name: `Copilot ${Date.now()}`, base_url: process.env.ARUVI_TEST_MODEL_URL }) : null;
  let repositoryId;
  let secondRepositoryId;
  let staleCopilotPayload;
  try {
    await page.goto("/remote");
    await page.locator("#tokenInput").fill(token);
    if (copilotProvider) { await page.locator("#providerInput").fill(copilotProvider.id); await page.locator("#modelInput").fill("browser-test-model"); }
    await page.locator("#saveButton").click();
    await page.getByRole("button", { name: "Workspaces", exact: true }).click();
    await page.getByLabel("Workspace name").fill("Browser test workspace");
    await page.getByLabel("Container repository path").fill(process.env.ARUVI_TEST_REPOSITORY_PATH);
    const registered = page.waitForResponse((response) => response.url().endsWith("/api/mobile/repositories") && response.request().method() === "POST");
    await page.getByRole("button", { name: "Register workspace", exact: true }).click();
    const response = await registered;
    expect(response.status()).toBe(201);
    repositoryId = (await response.json()).id;
    await expect(page.locator("#repositoryList")).toContainText("Browser test workspace");
    if (process.env.ARUVI_TEST_INSPECT === "1") {
      const row = page.locator("#repositoryList .tool-card").filter({ hasText: "Browser test workspace" });
      await row.getByRole("button", { name: "Browse workspace" }).click();
      await expect(page.locator("#repositoryGitStatus")).toContainText("changed file(s)");
      await page.getByRole("button", { name: "Show Git diff" }).click();
      await expect(page.locator("#repositoryDiff")).toContainText("Staged changes");
      await expect(page.locator("#repositoryDiff")).toContainText("+staged line");
      await expect(page.locator("#repositoryDiff")).toContainText("+working line");
      const limitedDiffs = [];
      for (const maxBytes of [1024, 1025, 1026]) {
        const limitedDiff = await call("aruvi_repositories", "get_git_diff", { repository_id: repositoryId, max_bytes: maxBytes });
        expect(limitedDiff.truncated).toBe(true);
        expect(typeof limitedDiff.diff).toBe("string");
        limitedDiffs.push(limitedDiff.diff);
      }
      expect(limitedDiffs.some((diff) => diff.endsWith("�"))).toBe(true);
      await expect(page.locator("#repositoryTree")).not.toContainText("node_modules");
      await page.locator("#repositoryTree").getByRole("button", { name: "inspection.txt" }).click();
      await expect(page.getByLabel("Selected file contents")).toHaveValue("workspace inspection fixture\n");
      await page.getByLabel("Selected file contents").fill("saved from browser\n");
      await page.getByRole("button", { name: "Save file", exact: true }).click();
      await expect(page.locator("#status")).toHaveText("File saved");
      expect((await call("aruvi_repositories", "read_repository_file", { repository_id: repositoryId, relative_path: "inspection.txt" })).content).toBe("saved from browser\n");
      const patchFile = "patch-safety-fixture.txt";
      await call("aruvi_repositories", "write_repository_file", { repository_id: repositoryId, relative_path: patchFile, content: "alpha\nbeta\n" });
      const patchHash = (await call("aruvi_repositories", "get_repository_file_sha256", { repository_id: repositoryId, relative_path: patchFile })).sha256;
      const previewArgs = { repository_id: repositoryId, relative_path: patchFile, base_sha256: patchHash, patch: "@@ -1,0 +2,1 @@\n+middle" };
      const preview = await call("aruvi_repositories", "preview_repository_patch", previewArgs);
      expect(preview.exists).toBe(true);
      expect(preview.base_sha256).toBe(patchHash);
      expect(preview.result_content).toBe("alpha\nmiddle\nbeta\n");
      const browserPreview = await request.post("/api/mobile/repository-inspect", { headers: { Authorization: `Bearer ${token}` }, data: { action: "preview_repository_patch", ...previewArgs } });
      expect(browserPreview.status()).toBe(200);
      expect(await browserPreview.json()).toEqual(preview);
      expect((await call("aruvi_repositories", "read_repository_file", { repository_id: repositoryId, relative_path: patchFile })).content).toBe("alpha\nbeta\n");
      const newPreview = await call("aruvi_repositories", "preview_repository_patch", { repository_id: repositoryId, relative_path: "preview-only/new.txt", patch: "@@ -0,0 +1 @@\n+new" });
      expect(newPreview.exists).toBe(false);
      expect(newPreview.result_content).toBe("new");
      await call("aruvi_repositories", "write_repository_file", { repository_id: repositoryId, relative_path: "preview-only/new.txt", content: "" });
      const existenceConflict = await request.post("/api/mobile/repository-inspect", { headers: { Authorization: `Bearer ${token}` }, data: { action: "apply_reviewed_repository_patch", repository_id: repositoryId, relative_path: "preview-only/new.txt", patch: "@@ -0,0 +1 @@\n+new", base_sha256: newPreview.base_sha256, expected_exists: false } });
      expect(existenceConflict.status()).toBe(400);
      expect((await call("aruvi_repositories", "read_repository_file", { repository_id: repositoryId, relative_path: "preview-only/new.txt" })).content).toBe("");
      const newArgs = { repository_id: repositoryId, relative_path: "preview-only/applied.txt", patch: "@@ -0,0 +1 @@\n+created" };
      const newFile = await call("aruvi_repositories", "preview_repository_patch", newArgs);
      await call("aruvi_repositories", "apply_reviewed_repository_patch", { ...newArgs, base_sha256: newFile.base_sha256, expected_exists: false });
      expect((await call("aruvi_repositories", "read_repository_file", { repository_id: repositoryId, relative_path: newArgs.relative_path })).content).toBe("created");
      const badPatchResponse = await request.post("/api/mcp", {
        headers: { Authorization: `Bearer ${mcpToken}`, Accept: "application/json, text/event-stream" },
        data: { jsonrpc: "2.0", id: 1, method: "tools/call", params: { name: "aruvi_repositories", arguments: { action: "apply_repository_patch", arguments: { repository_id: repositoryId, relative_path: patchFile, base_sha256: patchHash, patch: "@@ -1,2 +1,1 @@\n-alpha\n+changed" } } } },
      });
      const badPatch = await badPatchResponse.json();
      expect(badPatch.result?.isError || badPatch.error, JSON.stringify(badPatch)).toBeTruthy();
      expect((await call("aruvi_repositories", "read_repository_file", { repository_id: repositoryId, relative_path: patchFile })).content).toBe("alpha\nbeta\n");
      await call("aruvi_repositories", "apply_repository_patch", { repository_id: repositoryId, relative_path: patchFile, base_sha256: patchHash, patch: "@@ -1,0 +2,1 @@\n+middle" });
      expect((await call("aruvi_repositories", "read_repository_file", { repository_id: repositoryId, relative_path: patchFile })).content).toBe("alpha\nmiddle\nbeta\n");
      if (copilotProvider) {
        await page.getByLabel("Copilot question").fill("Explain this file");
        await page.getByRole("button", { name: "Ask about saved file" }).click();
        await expect(page.locator("#repositoryCopilotResult")).toContainText("Container provider reply");
        const captured = await (await request.get(process.env.ARUVI_TEST_MODEL_URL.replace(/\/v1\/?$/, "/copilot/last"))).json();
        expect(JSON.stringify(captured.messages)).toContain("saved from browser");
        expect(JSON.stringify(captured.messages)).toContain("inspection.txt");
        expect(JSON.stringify(captured.messages)).not.toContain(process.env.ARUVI_TEST_REPOSITORY_PATH);
        await page.getByLabel("Copilot question").fill("What did I ask first?");
        await page.getByRole("button", { name: "Ask about saved file" }).click();
        await expect(page.locator("#repositoryCopilotResult")).toContainText("What did I ask first?");
        const followup = await (await request.get(process.env.ARUVI_TEST_MODEL_URL.replace(/\/v1\/?$/, "/copilot/last"))).json();
        expect(followup.messages.map((entry) => entry.role)).toEqual(["system", "user", "assistant", "user"]);
        expect(followup.messages[1].content).toContain("Explain this file");
        expect(followup.messages[2].content).toBe("Container provider reply");
        expect(followup.messages[3].content).toBe("What did I ask first?");
        await page.getByLabel("Copilot question").fill("Synthetic Copilot failure");
        await page.getByRole("button", { name: "Ask about saved file" }).click();
        await expect(page.locator("#repositoryCopilotResult")).toContainText("Copilot error:");
        await page.getByLabel("Copilot question").fill("Continue after failure");
        await page.getByRole("button", { name: "Ask about saved file" }).click();
        await expect(page.locator("#repositoryCopilotResult")).toContainText("Continue after failure");
        const afterFailure = await (await request.get(process.env.ARUVI_TEST_MODEL_URL.replace(/\/v1\/?$/, "/copilot/last"))).json();
        expect(JSON.stringify(afterFailure.messages)).not.toContain("Synthetic Copilot failure");
        await page.getByLabel("Stream Copilot response").check();
        await page.getByLabel("Copilot question").fill("Stream this file");
        await page.getByRole("button", { name: "Ask about saved file" }).click();
        await expect(page.locator("#repositoryCopilotResult")).toContainText("First second");
        await expect(page.locator("#status")).toHaveText("Copilot reply ready");
        await page.getByLabel("Copilot question").fill("stream failure");
        await page.getByRole("button", { name: "Ask about saved file" }).click();
        await expect(page.locator("#repositoryCopilotResult")).toContainText("[Response incomplete]");
        await page.getByLabel("Copilot question").fill("stream cancel");
        await page.getByRole("button", { name: "Ask about saved file" }).click();
        await expect(page.getByRole("button", { name: "Stop Copilot" })).toBeEnabled();
        await page.getByRole("button", { name: "Stop Copilot" }).click();
        await expect(page.locator("#status")).toHaveText("Stream stopped");
        await page.getByLabel("Stream Copilot response").uncheck();
        await page.getByLabel("Copilot question").fill("After stopped stream");
        await page.getByRole("button", { name: "Ask about saved file" }).click();
        await expect(page.locator("#status")).toHaveText("Copilot reply ready");
        const afterStreamFailure = await (await request.get(process.env.ARUVI_TEST_MODEL_URL.replace(/\/v1\/?$/, "/copilot/last"))).json();
        expect(JSON.stringify(afterStreamFailure.messages)).not.toContain("stream failure");
        expect(JSON.stringify(afterStreamFailure.messages)).not.toContain("stream cancel");
        expect(JSON.stringify(afterStreamFailure.messages)).toContain("Stream this file");
        await page.getByRole("button", { name: "New file conversation" }).click();
        await page.getByLabel("Copilot question").fill("Fresh question");
        await page.getByRole("button", { name: "Ask about saved file" }).click();
        await expect(page.locator("#repositoryCopilotResult")).toContainText("Fresh question");
        const fresh = await (await request.get(process.env.ARUVI_TEST_MODEL_URL.replace(/\/v1\/?$/, "/copilot/last"))).json();
        expect(fresh.messages.map((entry) => entry.role)).toEqual(["system", "user"]);
        expect(JSON.stringify(fresh.messages)).not.toContain("What did I ask first?");
        await page.getByRole("button", { name: "Planner", exact: true }).click();
        await page.locator("#modelInput").fill("browser-test-model-2");
        await page.locator("#saveButton").click();
        await page.getByRole("button", { name: "Workspaces", exact: true }).click();
        await page.getByLabel("Copilot question").fill("After model change");
        await page.getByRole("button", { name: "Ask about saved file" }).click();
        await expect(page.locator("#repositoryCopilotResult")).toContainText("After model change");
        const changedModel = await (await request.get(process.env.ARUVI_TEST_MODEL_URL.replace(/\/v1\/?$/, "/copilot/last"))).json();
        expect(changedModel.messages.map((entry) => entry.role)).toEqual(["system", "user"]);
        expect(JSON.stringify(changedModel.messages)).not.toContain("Fresh question");
        await page.getByLabel("Copilot question").fill("Synthetic long Copilot reply");
        await page.getByRole("button", { name: "Ask about saved file" }).click();
        await expect(page.locator("#repositoryCopilotResult")).toContainText("[Earlier conversation omitted]");
        expect((await page.locator("#repositoryCopilotResult").textContent()).length).toBeLessThan(100100);
        await page.getByLabel("Copilot question").fill("After long reply");
        await page.getByRole("button", { name: "Ask about saved file" }).click();
        await expect(page.locator("#repositoryCopilotResult")).toContainText("After long reply");
        const afterLong = await (await request.get(process.env.ARUVI_TEST_MODEL_URL.replace(/\/v1\/?$/, "/copilot/last"))).json();
        expect(afterLong.messages.map((entry) => entry.role)).toEqual(["system", "user"]);
        await page.getByLabel("Stream Copilot response").check();
        await page.getByLabel("Copilot question").fill("Propose fixture changes");
        await page.getByRole("button", { name: "Propose changes" }).click();
        await expect(page.locator("#repositoryPatchReview")).toContainText("Two fixture changes");
        await expect(page.locator("#repositoryPatchReview")).toContainText("proposal-created.txt (new file)");
        expect((await call("aruvi_repositories", "read_repository_file", { repository_id: repositoryId, relative_path: patchFile })).content).toBe("alpha\nmiddle\nbeta\n");
        await call("aruvi_repositories", "write_repository_file", { repository_id: repositoryId, relative_path: patchFile, content: "concurrent change\n" });
        await page.getByRole("button", { name: "Apply reviewed changes" }).click();
        await expect(page.locator("#status")).toContainText("Applied 0 of 2 patches; stopped");
        expect((await call("aruvi_repositories", "read_repository_file", { repository_id: repositoryId, relative_path: patchFile })).content).toBe("concurrent change\n");
        await call("aruvi_repositories", "write_repository_file", { repository_id: repositoryId, relative_path: patchFile, content: "alpha\nmiddle\nbeta\n" });
        await page.getByLabel("Copilot question").fill("Propose fixture changes again");
        await page.getByRole("button", { name: "Propose changes" }).click();
        await expect(page.locator("#repositoryPatchReview")).toContainText("Two fixture changes");
        await call("aruvi_repositories", "write_repository_file", { repository_id: repositoryId, relative_path: "proposal-created.txt", content: "" });
        await page.getByRole("button", { name: "Apply reviewed changes" }).click();
        await expect(page.locator("#status")).toContainText("Applied 1 of 2 patches; stopped");
        expect((await call("aruvi_repositories", "read_repository_file", { repository_id: repositoryId, relative_path: patchFile })).content).toBe("alpha\nreviewed\nbeta\n");
        await call("aruvi_repositories", "write_repository_file", { repository_id: repositoryId, relative_path: patchFile, content: "alpha\nmiddle\nbeta\n" });
        await page.getByLabel("Copilot question").fill("Propose fixture changes after partial application");
        await page.getByRole("button", { name: "Propose changes" }).click();
        await expect(page.locator("#repositoryPatchReview")).toContainText("Two fixture changes");
        await page.getByRole("button", { name: "Apply reviewed changes" }).click();
        await expect(page.locator("#status")).toHaveText("Applied 2 reviewed patch(es)");
        await page.getByLabel("Stream Copilot response").uncheck();
        expect((await call("aruvi_repositories", "read_repository_file", { repository_id: repositoryId, relative_path: patchFile })).content).toBe("alpha\nreviewed\nbeta\n");
        expect((await call("aruvi_repositories", "read_repository_file", { repository_id: repositoryId, relative_path: "proposal-created.txt" })).content).toBe("created");
        const inspect = await (await request.post("/api/mobile/repository-inspect", { headers: { Authorization: `Bearer ${token}` }, data: { action: "read_repository_file", repository_id: repositoryId, relative_path: "inspection.txt" } })).json();
        const payload = { repository_id: repositoryId, relative_path: "inspection.txt", expected_sha256: inspect.sha256, provider_id: copilotProvider.id, model_name: "browser-test-model", question: "Explain" };
        expect((await call("aruvi_repositories", "get_repository_file_sha256", { repository_id: repositoryId, relative_path: "inspection.txt" })).sha256).toBe(inspect.sha256);
        expect((await call("aruvi_models", "run_file_copilot", payload)).content).toBe("Container provider reply");
        const mcpProposal = JSON.parse((await call("aruvi_models", "run_file_copilot", { ...payload, patch_mode: true })).content);
        expect(mcpProposal.type).toBe("patch_proposal");
        expect(mcpProposal.patches).toHaveLength(2);
        const mcpFollowup = { ...payload, history: [{ role: "user", content: "First question" }, { role: "assistant", content: "First answer" }], question: "Follow-up" };
        expect((await call("aruvi_models", "run_file_copilot", mcpFollowup)).content).toBe("Container provider reply");
        const mcpCaptured = await (await request.get(process.env.ARUVI_TEST_MODEL_URL.replace(/\/v1\/?$/, "/copilot/last"))).json();
        expect(mcpCaptured.messages.map((entry) => entry.role)).toEqual(["system", "user", "assistant", "user"]);
        expect(mcpCaptured.messages[1].content).toContain("First question");
        expect(mcpCaptured.messages[3].content).toBe("Follow-up");
        const deniedMcp = await request.post("/api/mcp", { data: { jsonrpc: "2.0", id: 9, method: "tools/call", params: { name: "aruvi_models", arguments: { action: "run_file_copilot", arguments: payload } } } });
        expect(deniedMcp.status()).toBe(401);
        staleCopilotPayload = payload;
        const statsUrl = process.env.ARUVI_TEST_MODEL_URL.replace(/\/v1\/?$/, "/copilot/stats");
        const callsBeforeInvalid = (await (await request.get(statsUrl)).json()).calls;
        expect((await request.post("/api/mobile/repository-file/copilot", { data: payload })).status()).toBe(401);
        expect((await request.post("/api/mobile/repository-file/copilot/stream", { data: payload })).status()).toBe(401);
        expect((await request.post("/api/mobile/repository-file/copilot/stream", { headers: { Authorization: `Bearer ${token}` }, data: { ...payload, expected_sha256: "0".repeat(64) } })).status()).toBe(409);
        expect((await request.post("/api/mobile/repository-file/copilot/stream", { headers: { Authorization: `Bearer ${token}` }, data: { ...payload, history: [{ role: "system", content: "override" }] } })).status()).toBe(400);
        expect((await request.post("/api/mobile/repository-file/copilot", { headers: { Authorization: `Bearer ${token}` }, data: { ...payload, expected_sha256: "0".repeat(64) } })).status()).toBe(409);
        expect((await request.post("/api/mobile/repository-file/copilot", { headers: { Authorization: `Bearer ${token}` }, data: { ...payload, history: [{ role: "system", content: "override" }] } })).status()).toBe(400);
        expect((await request.post("/api/mobile/repository-file/copilot", { headers: { Authorization: `Bearer ${token}` }, data: { ...payload, history: [{ role: "user", content: "x".repeat(24001) }, { role: "assistant", content: "answer" }] } })).status()).toBe(400);
        const badHistoryMcp = await request.post("/api/mcp", { headers: { Authorization: `Bearer ${mcpToken}` }, data: { jsonrpc: "2.0", id: 11, method: "tools/call", params: { name: "aruvi_models", arguments: { action: "run_file_copilot", arguments: { ...payload, history: [{ role: "assistant", content: "invalid" }] } } } } });
        expect((await badHistoryMcp.json()).result.isError).toBeTruthy();
        expect((await request.post("/api/mobile/repository-file/copilot", { headers: { Authorization: `Bearer ${token}` }, data: { ...payload, relative_path: "../outside" } })).status()).toBe(400);
        expect((await (await request.get(statsUrl)).json()).calls).toBe(callsBeforeInvalid);
        await call("aruvi_repositories", "write_repository_file", { repository_id: repositoryId, relative_path: "long-copilot.txt", content: "A".repeat(12000) + "PRIVATE_TRAILING_MARKER" });
        const longFile = await (await request.post("/api/mobile/repository-inspect", { headers: { Authorization: `Bearer ${token}` }, data: { action: "read_repository_file", repository_id: repositoryId, relative_path: "long-copilot.txt" } })).json();
        const longReply = await request.post("/api/mobile/repository-file/copilot", { headers: { Authorization: `Bearer ${token}` }, data: { ...payload, relative_path: "long-copilot.txt", expected_sha256: longFile.sha256 } });
        expect(longReply.ok()).toBeTruthy();
        const bounded = JSON.stringify((await (await request.get(process.env.ARUVI_TEST_MODEL_URL.replace(/\/v1\/?$/, "/copilot/last"))).json()).messages);
        expect(bounded).toContain("truncated for model context");
        expect(bounded).not.toContain("PRIVATE_TRAILING_MARKER");
        const calls = await (await request.get("/api/mobile/model-calls?limit=100", { headers: { Authorization: `Bearer ${token}` } })).json();
        expect(calls.some((item) => item.provider_id === copilotProvider.id && item.source_kind === "mobile_ide" && item.status === "completed")).toBeTruthy();
        expect(calls.some((item) => item.provider_id === copilotProvider.id && item.source_kind === "mcp_ide" && item.status === "completed")).toBeTruthy();
      }
      await page.getByLabel("Selected file contents").fill("unsaved browser edit\n");
      await expect(page.getByRole("button", { name: "Ask about saved file" })).toBeDisabled();
      await page.locator("#repositoryTree").getByRole("button", { name: "windows.txt" }).click();
      await expect(page.getByLabel("Selected file contents")).toHaveValue("unsaved browser edit\n");
      await call("aruvi_repositories", "write_repository_file", { repository_id: repositoryId, relative_path: "inspection.txt", content: "concurrent edit\n" });
      if (staleCopilotPayload) {
        const statsUrl = process.env.ARUVI_TEST_MODEL_URL.replace(/\/v1\/?$/, "/copilot/stats");
        const before = (await (await request.get(statsUrl)).json()).calls;
        expect((await request.post("/api/mobile/repository-file/copilot", { headers: { Authorization: `Bearer ${token}` }, data: staleCopilotPayload })).status()).toBe(409);
        const staleMcp = await request.post("/api/mcp", { headers: { Authorization: `Bearer ${mcpToken}` }, data: { jsonrpc: "2.0", id: 10, method: "tools/call", params: { name: "aruvi_models", arguments: { action: "run_file_copilot", arguments: staleCopilotPayload } } } });
        expect((await staleMcp.json()).result.isError).toBeTruthy();
        expect((await (await request.get(statsUrl)).json()).calls).toBe(before);
      }
      const conflict = page.waitForResponse((response) => response.url().endsWith("/api/mobile/repository-file/save"));
      await page.getByRole("button", { name: "Save file", exact: true }).click();
      expect((await conflict).status()).toBe(409);
      await expect(page.getByLabel("Selected file contents")).toHaveValue("unsaved browser edit\n");
      expect((await call("aruvi_repositories", "read_repository_file", { repository_id: repositoryId, relative_path: "inspection.txt" })).content).toBe("concurrent edit\n");
      await page.getByRole("button", { name: "Discard file edits" }).click();
      await page.locator("#repositoryTree").getByRole("button", { name: "windows.txt" }).click();
      await expect(page.getByLabel("Selected file contents")).toHaveValue("windows line\n");
      await page.getByLabel("Selected file contents").fill("updated windows\n");
      await page.getByRole("button", { name: "Save file", exact: true }).click();
      await expect(page.locator("#status")).toHaveText("File saved");
      expect((await call("aruvi_repositories", "read_repository_file", { repository_id: repositoryId, relative_path: "windows.txt" })).content).toBe("updated windows\r\n");
      await page.locator("#repositoryTree").getByRole("button", { name: "mixed.txt" }).click();
      await expect(page.getByLabel("Selected file contents")).toHaveAttribute("readonly", "");
      await expect(page.locator("#status")).toContainText("mixed line endings");
      const tooLarge = { repository_id: repositoryId, relative_path: "inspection.txt", expected_content: "concurrent edit\n", content: "x".repeat(2097153) };
      expect((await request.post("/api/mobile/repository-file/save", { headers: { Authorization: `Bearer ${token}` }, data: tooLarge })).status()).toBe(400);
      const saveData = { repository_id: repositoryId, relative_path: "../outside", expected_content: "", content: "x" };
      expect((await request.post("/api/mobile/repository-file/save", { data: saveData })).status()).toBe(401);
      expect((await request.post("/api/mobile/repository-file/save", { headers: { Authorization: `Bearer ${token}` }, data: saveData })).status()).toBe(400);
      const inspectHeaders = { Authorization: `Bearer ${token}` };
      expect((await request.post("/api/mobile/repository-inspect", { data: { action: "get_git_status", repository_id: repositoryId } })).status()).toBe(401);
      expect((await request.post("/api/mobile/repository-inspect", { headers: inspectHeaders, data: { action: "write_repository_file", repository_id: repositoryId } })).status()).toBe(400);
      expect((await request.post("/api/mobile/repository-inspect", { headers: inspectHeaders, data: { action: "read_repository_file", repository_id: repositoryId, relative_path: "../outside" } })).status()).toBe(400);
      expect((await request.post("/api/mobile/repository-inspect", { headers: inspectHeaders, data: { action: "read_repository_file", repository_id: repositoryId, relative_path: "outside-link" } })).status()).toBe(400);
      expect((await request.post("/api/mobile/repository-inspect", { headers: inspectHeaders, data: { action: "read_repository_file", repository_id: repositoryId, relative_path: "oversized.txt" } })).status()).toBe(400);
    }
    await page.getByLabel("Product for workspace").selectOption(product.id);
    await page.getByLabel("Registered workspace").selectOption(repositoryId);
    await page.getByRole("button", { name: "Attach workspace", exact: true }).click();
    await expect(page.locator("#repositoryAttachmentList")).toContainText(`${name} · Browser test workspace · default`);
    const resolved = await call("aruvi_repositories", "resolve_repository_for_scope", { product_id: product.id });
    expect(resolved.id).toBe(repositoryId);
    if (process.env.ARUVI_TEST_SECOND_REPOSITORY_PATH) {
      expect((await request.get(`/api/mobile/products/${product.id}/areas`)).status()).toBe(401);
      const denied = await request.post("/api/mobile/repository-attachments/default", { data: { product_id: product.id, repository_id: repositoryId } });
      expect(denied.status()).toBe(401);
      const detachDenied = await request.post("/api/mobile/repository-attachments/detach", { data: { product_id: product.id, repository_id: repositoryId } });
      expect(detachDenied.status()).toBe(401);
      const missing = await request.post("/api/mobile/repository-attachments/default", { headers: { Authorization: `Bearer ${token}` }, data: { product_id: product.id, repository_id: "missing" } });
      expect(missing.status()).toBe(400);
      expect((await call("aruvi_repositories", "resolve_repository_for_scope", { product_id: product.id })).id).toBe(repositoryId);
      await page.getByLabel("Workspace name").fill("Second test workspace");
      await page.getByLabel("Container repository path").fill(process.env.ARUVI_TEST_SECOND_REPOSITORY_PATH);
      const secondRegistered = page.waitForResponse((response) => response.url().endsWith("/api/mobile/repositories") && response.request().method() === "POST");
      await page.getByRole("button", { name: "Register workspace", exact: true }).click();
      secondRepositoryId = (await (await secondRegistered).json()).id;
      await page.getByLabel("Product for workspace").selectOption(product.id);
      await page.locator("#repositoryArea").selectOption(area.id);
      await page.getByLabel("Registered workspace").selectOption(secondRepositoryId);
      await page.getByRole("button", { name: "Attach workspace", exact: true }).click();
      const areaRow = page.locator("#repositoryAttachmentList .tool-card").filter({ hasText: `${name} / Workspace test area · Second test workspace` });
      await expect(areaRow).toContainText("default");
      expect((await call("aruvi_repositories", "resolve_repository_for_scope", { product_id: product.id, product_area_id: area.id })).id).toBe(secondRepositoryId);
      expect((await call("aruvi_repositories", "resolve_repository_for_scope", { product_id: product.id })).id).toBe(repositoryId);
      const invalid = await request.post("/api/mobile/repository-attachments", { headers: { Authorization: `Bearer ${token}` }, data: { product_id: product.id, product_area_id: otherArea.id, repository_id: secondRepositoryId } });
      expect(invalid.status()).toBe(400);
      await page.getByLabel("Registered workspace").selectOption(repositoryId);
      await page.getByRole("button", { name: "Attach workspace", exact: true }).click();
      const firstAreaRow = page.locator("#repositoryAttachmentList .tool-card").filter({ hasText: `${name} / Workspace test area · Browser test workspace` });
      await firstAreaRow.getByRole("button", { name: "Make default" }).click();
      await expect(firstAreaRow).toContainText("default");
      expect((await call("aruvi_repositories", "resolve_repository_for_scope", { product_id: product.id, product_area_id: area.id })).id).toBe(repositoryId);
      await firstAreaRow.getByRole("button", { name: "Detach workspace" }).click();
      await expect(firstAreaRow).toHaveCount(0);
      expect((await call("aruvi_repositories", "resolve_repository_for_scope", { product_id: product.id, product_area_id: area.id })).id).toBe(secondRepositoryId);
      await areaRow.getByRole("button", { name: "Detach workspace" }).click();
      await expect(areaRow).toHaveCount(0);
      expect((await call("aruvi_repositories", "resolve_repository_for_scope", { product_id: product.id, product_area_id: area.id })).id).toBe(repositoryId);
      await page.locator("#repositoryArea").selectOption("");
      await page.getByLabel("Registered workspace").selectOption(secondRepositoryId);
      await page.getByRole("button", { name: "Attach workspace", exact: true }).click();
      const secondRow = page.locator("#repositoryAttachmentList .tool-card").filter({ hasText: `${name} · Second test workspace` });
      await expect(secondRow).toBeVisible();
      await secondRow.getByRole("button", { name: "Make default" }).click();
      await expect(secondRow).toContainText("default");
      expect((await call("aruvi_repositories", "resolve_repository_for_scope", { product_id: product.id })).id).toBe(secondRepositoryId);
      const attachments = await call("aruvi_repositories", "list_repository_attachments", { scope_type: "product", scope_id: product.id });
      expect(attachments.find((item) => item.repository_id === secondRepositoryId).is_default).toBe(true);
      await call("aruvi_repositories", "set_default_repository", { scope_type: "product", scope_id: product.id, repository_id: repositoryId });
      expect((await call("aruvi_repositories", "resolve_repository_for_scope", { product_id: product.id })).id).toBe(repositoryId);
      await page.locator("#refreshWorkspacesButton").click();
      await secondRow.getByRole("button", { name: "Make default" }).click();
      await expect(secondRow).toContainText("default");
      await secondRow.getByRole("button", { name: "Detach workspace" }).click();
      await expect(secondRow).toHaveCount(0);
      expect((await call("aruvi_repositories", "resolve_repository_for_scope", { product_id: product.id })).id).toBe(repositoryId);
      await call("aruvi_repositories", "set_default_repository", { scope_type: "product", scope_id: product.id, repository_id: repositoryId });
      await call("aruvi_repositories", "detach_repository", { scope_type: "product", scope_id: product.id, repository_id: repositoryId });
      expect(await call("aruvi_repositories", "list_repository_attachments", { scope_type: "product", scope_id: product.id })).toEqual([]);
    }
  } finally {
    await call("aruvi_catalog", "archive_product", { id: product.id });
    await call("aruvi_catalog", "archive_product", { id: otherProduct.id });
    if (secondRepositoryId) await call("aruvi_repositories", "delete_repository", { id: secondRepositoryId });
    if (repositoryId) await call("aruvi_repositories", "delete_repository", { id: repositoryId });
  }
});

test("browser edits workflow automation with conflict protection", async ({ page, request }) => {
  test.skip(!token || !mcpToken || process.env.ARUVI_TEST_INSPECT !== "1", "Requires disposable remote fixture");
  const url = "/api/mobile/workflow-automation", headers = { Authorization: `Bearer ${token}` };
  expect((await request.get(url)).status()).toBe(401);
  expect((await request.post(url, { data: { key: "workflow.auto_approve_plan", expected_raw: null, enabled: false } })).status()).toBe(401);
  expect((await request.post(url, { headers, data: { key: "mobile.api_token", expected_raw: null, enabled: false } })).status()).toBe(400);
  const initial = await (await request.get(url, { headers })).json();
  expect(initial).toHaveLength(3);
  await page.goto("/remote"); await page.locator("#tokenInput").fill(token); await page.locator("#saveButton").click();
  await page.getByRole("button", { name: "Routing", exact: true }).click();
  const toggle = page.getByLabel("Auto-approve planning", { exact: true });
  await expect(toggle).toBeChecked();
  await toggle.uncheck();
  await expect(page.locator("#status")).toHaveText("Workflow automation saved");
  const changed = await (await request.get(url, { headers })).json();
  expect(changed.find((entry) => entry.key === "workflow.auto_approve_plan").enabled).toBe(false);
  expect(changed.filter((entry) => entry.key !== "workflow.auto_approve_plan")).toEqual(initial.filter((entry) => entry.key !== "workflow.auto_approve_plan"));
  const mcp = await request.post("/api/mcp", { headers: { Authorization: `Bearer ${mcpToken}`, Accept: "application/json, text/event-stream" }, data: { jsonrpc: "2.0", id: 1, method: "tools/call", params: { name: "aruvi_settings", arguments: { action: "set_setting", arguments: { key: "workflow.auto_approve_plan", value: "true" } } } } });
  expect((await mcp.json()).result.isError).not.toBeTruthy();
  const conflict = page.waitForResponse((response) => response.url().endsWith(url) && response.request().method() === "POST");
  await toggle.check();
  expect((await conflict).status()).toBe(409);
  await expect(toggle).not.toBeChecked();
  await page.getByRole("button", { name: "Refresh automation" }).click();
  await expect(toggle).toBeChecked();
});

test("browser configures and selects a model", async ({ page, request }) => {
  test.skip(!token || !mcpToken, "Set mobile and MCP tokens for a running remote server");
  const name = `Provider test ${Date.now()}`;
  let providerId, modelId, agentId;
  async function call(namespace, action, arguments_) {
    const response = await request.post("/api/mcp", {
      headers: { Authorization: `Bearer ${mcpToken}`, Accept: "application/json, text/event-stream" },
      data: { jsonrpc: "2.0", id: 1, method: "tools/call", params: { name: namespace, arguments: { action, arguments: arguments_ } } },
    });
    const result = (await response.json()).result;
    expect(result.isError).not.toBeTruthy();
    return result.structuredContent.result;
  }
  try {
    const agent = await call("aruvi_agents", "create_agent_definition", { name: `Test agent ${Date.now()}`, role: "test" });
    agentId = agent.id;
    await page.goto("/remote");
    await page.locator("#tokenInput").fill(token);
    await page.locator("#saveButton").click();
    await page.getByRole("button", { name: "Models", exact: true }).click();
    await page.getByLabel("Provider name", { exact: true }).fill(name);
    await page.getByLabel("API base URL").fill(process.env.ARUVI_TEST_MODEL_URL || "https://example.com/v1");
    const providerResponse = page.waitForResponse((r) => r.url().endsWith("/api/mobile/providers") && r.request().method() === "POST");
    await page.getByRole("button", { name: "Add provider", exact: true }).click();
    const provider = await providerResponse;
    expect(provider.ok()).toBeTruthy();
    const providerBody = await provider.json();
    expect(providerBody).not.toHaveProperty("auth_secret_ref");
    providerId = providerBody.id;
    const expectedProvider = { name: providerBody.name, base_url: providerBody.base_url, enabled: providerBody.enabled };
    const capabilityUrl = `/api/mobile/providers/${providerId}/transport-capabilities`;
    expect((await request.get(capabilityUrl)).status()).toBe(401);
    const transport = await call("aruvi_models", "get_provider_transport_capabilities", { id: providerId });
    expect(transport).toEqual({ text_completion: true, text_streaming: true, model_discovery: true, transcription: true, speech: true, json_object_completion: true });
    expect(await (await request.get(capabilityUrl, { headers: { Authorization: `Bearer ${token}` } })).json()).toEqual(transport);
    expect((await request.post("/api/mobile/providers/update", { data: { id: providerId, expected: expectedProvider, fields: expectedProvider } })).status()).toBe(401);
    expect((await request.post("/api/mobile/providers/update", { headers: { Authorization: `Bearer ${token}` }, data: { id: providerId, expected: expectedProvider, fields: { ...expectedProvider, base_url: "http://user:pass@example.com" } } })).status()).toBe(400);
    await expect(page.locator("#providerList")).toContainText(name);
    const capabilityCard = page.locator("#providerList .tool-card").filter({ hasText: name });
    await capabilityCard.getByRole("button", { name: "Show transport capabilities" }).click();
    await expect(capabilityCard).toContainText("Transport supports: text, streaming, model discovery, transcription, speech, JSON object mode.");
    await expect(capabilityCard).toContainText("Actual provider and model support can differ.");
    if (process.env.ARUVI_TEST_MODEL_URL) {
      const providerCard = page.locator("#providerList .tool-card").filter({ hasText: name });
      await providerCard.getByRole("button", { name: "Test connection" }).click();
      await expect(page.locator("#status")).toHaveText("Connection successful");
    }
    await page.getByLabel("Provider for model").selectOption(providerId);
    if (process.env.ARUVI_TEST_MODEL_URL) {
      let releaseOld, startedOld;
      const oldStarted = new Promise((resolve) => { startedOld = resolve; });
      const oldGate = new Promise((resolve) => { releaseOld = resolve; });
      let discoveryRequests = 0;
      await page.route("**/api/mobile/providers/discover", async (route) => {
        discoveryRequests++;
        const old = discoveryRequests === 1;
        if (old) { startedOld(); await oldGate; }
        await route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify({ models: [{ id: old ? "stale-model" : "fresh-model" }], next_cursor: null }) });
      });
      await page.getByRole("button", { name: "Discover available models" }).click();
      await oldStarted;
      await page.locator("#modelProvider").evaluate((select) => select.add(new Option("Other fixture", "other-fixture")));
      await page.getByLabel("Provider for model").selectOption("other-fixture");
      await page.getByLabel("Provider for model").selectOption(providerId);
      await page.getByRole("button", { name: "Discover available models" }).click();
      await expect(page.locator("#discoveredModels")).toContainText("fresh-model");
      const oldResponse = page.waitForResponse((response) => response.url().endsWith("/api/mobile/providers/discover") && response.request().postDataJSON()?.id === providerId);
      releaseOld();
      await (await oldResponse).finished();
      await page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => resolve())));
      await expect(page.locator("#discoveredModels")).not.toContainText("stale-model");
      await page.unroute("**/api/mobile/providers/discover");
      expect((await request.post("/api/mobile/providers/discover", { data: { id: providerId } })).status()).toBe(401);
      const discovered = await call("aruvi_models", "discover_provider_models", { id: providerId });
      expect(discovered.models.map((model) => model.id)).toContain("browser-test-model");
      await page.getByRole("button", { name: "Discover available models" }).click();
      await page.locator("#discoveredModels").selectOption("browser-test-model");
      await expect(page.getByLabel("Model name", { exact: true })).toHaveValue("browser-test-model");
    } else {
      await page.getByLabel("Model name", { exact: true }).fill("browser-test-model");
    }
    const modelResponse = page.waitForResponse((r) => r.url().endsWith("/api/mobile/models") && r.request().method() === "POST");
    await page.getByRole("button", { name: "Add model", exact: true }).click();
    const model = await modelResponse;
    expect(model.ok()).toBeTruthy();
    modelId = (await model.json()).id;
    const card = page.locator("#modelList .tool-card").filter({ hasText: "browser-test-model" }).filter({ hasText: name });
    await expect(card).toBeVisible();
    await page.locator("#bindingAgent").selectOption(agentId, { timeout: 5000 });
    await page.locator("#bindingModel").selectOption(modelId, { timeout: 5000 });
    await page.getByRole("button", { name: "Save agent model" }).click();
    await expect(page.locator("#agentBindingList")).toContainText(`${agent.name} · browser-test-model`);
    await card.getByRole("button", { name: "Use for this browser" }).click();
    await page.getByRole("button", { name: "Planner", exact: true }).click();
    await expect(page.locator("#providerInput")).toHaveValue(providerId);
    await expect(page.locator("#modelInput")).toHaveValue("browser-test-model");
    if (process.env.ARUVI_TEST_MODEL_URL) {
      await page.getByRole("button", { name: "Chat", exact: true }).click();
      await page.locator("#chatComposer").fill("Hello from browser");
      await page.locator("#chatSendButton").click();
      await expect(page.locator("#chatConversation")).toContainText("Container provider reply");
    }
    await page.getByRole("button", { name: "Models", exact: true }).click();
    let modelCard = page.locator("#modelList .tool-card").filter({ hasText: "browser-test-model" }).filter({ hasText: name });
    await modelCard.getByText("Manage model", { exact: true }).click();
    await modelCard.getByLabel("Edit model name browser-test-model").fill("renamed-browser-model");
    await modelCard.getByLabel("Context window browser-test-model").fill("8192");
    await modelCard.getByLabel("Model notes browser-test-model").fill("Updated notes");
    await modelCard.getByRole("button", { name: "Save model" }).click();
    await expect(page.locator("#status")).toHaveText("Model saved");
    let models = await (await request.get("/api/mobile/models", { headers: { Authorization: `Bearer ${token}` } })).json();
    expect(models.find((item) => item.id === modelId)).toMatchObject({ name: "renamed-browser-model", context_window: 8192, notes: "Updated notes" });
    modelCard = page.locator("#modelList .tool-card").filter({ hasText: "renamed-browser-model" }).filter({ hasText: name });
    await modelCard.getByText("Manage model", { exact: true }).click();
    await modelCard.getByLabel("Edit model name renamed-browser-model").fill("unsaved-model-name");
    await call("aruvi_models", "update_model_definition", { id: modelId, notes: "Concurrent model edit" });
    const conflict = page.waitForResponse((r) => r.url().endsWith("/api/mobile/models/update"));
    await modelCard.getByRole("button", { name: "Save model" }).click();
    expect((await conflict).status()).toBe(409);
    await expect(modelCard.getByLabel("Edit model name renamed-browser-model")).toHaveValue("unsaved-model-name");
    await page.locator("#refreshModelsButton").click();
    await expect(page.getByLabel("Edit model name renamed-browser-model")).toBeHidden();
    const providerCard = page.locator("#providerList .tool-card").filter({ hasText: name });
    await providerCard.getByText("Manage provider", { exact: true }).click();
    await providerCard.getByLabel(`Edit provider name ${name}`).fill(`${name} updated`);
    await providerCard.getByRole("button", { name: "Save provider" }).click();
    await expect(page.locator("#status")).toHaveText("Provider saved");
    let updatedProviderCard = page.locator("#providerList .tool-card").filter({ hasText: `${name} updated` });
    await updatedProviderCard.getByText("Manage provider", { exact: true }).click();
    await updatedProviderCard.getByLabel("Provider enabled").uncheck();
    await updatedProviderCard.getByRole("button", { name: "Save provider" }).click();
    await expect(page.locator("#status")).toHaveText("Provider saved");
    updatedProviderCard = page.locator("#providerList .tool-card").filter({ hasText: `${name} updated` });
    await expect(updatedProviderCard).toContainText("disabled");
    modelCard = page.locator("#modelList .tool-card").filter({ hasText: "renamed-browser-model" }).filter({ hasText: name });
    await expect(modelCard.getByRole("button", { name: "Use for this browser" })).toBeDisabled();
    await updatedProviderCard.getByText("Manage provider", { exact: true }).click();
    await updatedProviderCard.getByLabel("Provider enabled").check();
    await updatedProviderCard.getByRole("button", { name: "Save provider" }).click();
    await expect(page.locator("#status")).toHaveText("Provider saved");
    updatedProviderCard = page.locator("#providerList .tool-card").filter({ hasText: `${name} updated` });
    await updatedProviderCard.getByText("Manage provider", { exact: true }).click();
    await updatedProviderCard.getByLabel(`Edit provider name ${name} updated`).fill("unsaved-provider-name");
    await call("aruvi_models", "update_provider", { id: providerId, name: `${name} external` });
    const providerConflict = page.waitForResponse((r) => r.url().endsWith("/api/mobile/providers/update"));
    await updatedProviderCard.getByRole("button", { name: "Save provider" }).click();
    expect((await providerConflict).status()).toBe(409);
    await expect(updatedProviderCard.getByLabel(`Edit provider name ${name} updated`)).toHaveValue("unsaved-provider-name");
    await page.locator("#refreshModelsButton").click();
    updatedProviderCard = page.locator("#providerList .tool-card").filter({ hasText: `${name} external` });
    await updatedProviderCard.getByText("Manage provider", { exact: true }).click();
    page.once("dialog", (dialog) => dialog.accept());
    await updatedProviderCard.getByRole("button", { name: "Delete provider" }).click();
    await expect(page.locator("#status")).toContainText("Remove its models first");
    modelCard = page.locator("#modelList .tool-card").filter({ hasText: "renamed-browser-model" }).filter({ hasText: name });
    await modelCard.getByText("Manage model", { exact: true }).click();
    page.once("dialog", (dialog) => dialog.accept());
    await modelCard.getByRole("button", { name: "Delete model" }).click();
    await expect(page.locator("#status")).toHaveText("Model deleted");
    modelId = null;
    await expect(page.locator("#agentBindingList")).not.toContainText(`${agent.name} · renamed-browser-model`);
    const updatedProviderAgain = page.locator("#providerList .tool-card").filter({ hasText: `${name} external` });
    await updatedProviderAgain.getByText("Manage provider", { exact: true }).click();
    page.once("dialog", (dialog) => dialog.accept());
    await updatedProviderAgain.getByRole("button", { name: "Delete provider" }).click();
    await expect(page.locator("#status")).toHaveText("Provider deleted");
    providerId = null;
  } finally {
    if (agentId) await call("aruvi_agents", "delete_agent_definition", { id: agentId });
    if (modelId) await call("aruvi_models", "delete_model_definition", { id: modelId });
    if (providerId) await call("aruvi_models", "delete_provider", { id: providerId });
  }
});

test("mobile and MCP APIs require their configured tokens", async ({ request }) => {
  const mobile = await request.get("/api/mobile/products");
  expect(mobile.status()).toBe(401);
  const mcp = await request.post("/api/mcp", {
    headers: { Accept: "application/json, text/event-stream" },
    data: { jsonrpc: "2.0", id: 1, method: "tools/list" },
  });
  expect(mcp.status()).toBe(401);
});

test("activity displays model calls and their details", async ({ page }) => {
  test.skip(!token, "Set ARUVI_MOBILE_API_TOKEN for a running remote server");
  const call = { id: "test-call", source_kind: "mobile_planner", source_id: "planner-1", session_id: "shared", source_label: "Planner test", call_index: 1, model_name: "example-model", status: "failed", created_at: "2026-01-01", provider_name: "Example", token_count_input: 42, error_message: "<script>bad</script>", request_snapshot_path: "/fixture/request.json" };
  const second = { ...call, id: "test-call-2", call_index: 2, status: "completed", token_count_input: 8 };
  const workflow = { ...call, id: "test-call-3", source_kind: "workflow_agent", session_id: null, workflow_run_id: "workflow-1", source_label: "Workflow test", status: "completed" };
  let callList = [call, second, workflow];
  await page.route("**/api/mobile/model-calls?*", (route) => route.fulfill({ json: callList }));
  await page.route("**/api/mobile/model-calls/test-call", (route) => route.fulfill({ json: call }));
  await page.route("**/api/mobile/model-calls/test-call/snapshots/request", (route) => route.fulfill({ json: { content: "Snapshot remains visible" } }));
  await page.goto("/remote");
  await page.locator("#tokenInput").fill(token);
  await page.locator("#saveButton").click();
  await page.getByRole("button", { name: "Activity", exact: true }).click();
  await expect(page.locator("#modelCallSummary")).toHaveText("2 sessions · 3 calls · 3 loaded (latest 300 maximum)");
  await expect(page.locator("#modelCallList")).toContainText("2 calls · failed · input 50");
  await page.getByLabel("Call source").selectOption("workflow");
  await expect(page.locator("#modelCallSummary")).toContainText("1 sessions · 1 calls");
  await expect(page.locator("#modelCallList")).not.toContainText("Planner test");
  await page.getByLabel("Call source").selectOption("failed");
  await expect(page.locator("#modelCallSummary")).toContainText("1 sessions · 1 calls");
  await page.getByLabel("Call source").selectOption("all");
  await page.getByRole("button", { name: /Planner test.*example-model.*Call #1/ }).click();
  await expect(page.locator("#modelCallDetails")).toContainText("Input tokens: 42");
  await expect(page.locator("#modelCallDetails")).toContainText("<script>bad</script>");
  await expect(page.locator("#modelCallDetails script")).toHaveCount(0);
  await page.getByRole("button", { name: "Show request snapshot" }).click();
  await expect(page.locator("#modelCallDetails")).toContainText("Snapshot remains visible");
  await page.getByLabel("Call source").selectOption("planner");
  callList = [call, second, workflow, { ...call, id: "test-call-4", call_index: 3, token_count_input: 3 }];
  await expect(page.locator("#modelCallSummary")).toContainText("1 sessions · 3 calls · 4 loaded", { timeout: 10000 });
  await expect(page.getByLabel("Call source")).toHaveValue("planner");
  await expect(page.locator("#modelCallDetails")).toContainText("Snapshot remains visible");
});

test("authenticated model-call ledger is available", async ({ request }) => {
  test.skip(!token, "Set ARUVI_MOBILE_API_TOKEN for a running remote server");
  const response = await request.get("/api/mobile/model-calls?limit=100", { headers: { Authorization: `Bearer ${token}` } });
  expect(response.status()).toBe(200);
  expect(Array.isArray(await response.json())).toBeTruthy();
});

test("a product can be created from the browser", async ({ page, request }) => {
  test.skip(!token || !mcpToken, "Set mobile and MCP tokens for a running remote server");
  const name = `Browser product ${Date.now()}`;
  let productId;
  try {
    await page.goto("/remote");
    await page.locator("#tokenInput").fill(token);
    await page.locator("#saveButton").click();
    await page.getByRole("button", { name: "Products", exact: true }).click();
    await page.getByLabel("Product name", { exact: true }).fill(name);
    await page.getByLabel("Description", { exact: true }).fill("Created from the remote browser");
    const created = page.waitForResponse((response) => response.url().endsWith("/api/mobile/products") && response.request().method() === "POST");
    await page.getByRole("button", { name: "Create product", exact: true }).click();
    const response = await created;
    expect(response.status()).toBe(201);
    productId = (await response.json()).id;
    await expect(page.getByRole("button", { name: new RegExp(name) })).toBeVisible();
    await expect(page.locator("#productStatus")).toHaveText(name);
    await expect(page.getByLabel("Product name", { exact: true })).toHaveValue("");
  } finally {
    if (productId) {
      const response = await request.post("/api/mcp", {
        headers: { Authorization: `Bearer ${mcpToken}`, Accept: "application/json, text/event-stream" },
        data: { jsonrpc: "2.0", id: 1, method: "tools/call", params: { name: "aruvi_catalog", arguments: { action: "archive_product", arguments: { id: productId } } } },
      });
      expect((await response.json()).result.isError).not.toBeTruthy();
    }
  }
});

test("product tree loads on demand and shows nested catalog data", async ({ page, request }) => {
  test.skip(!token || !mcpToken, "Set mobile and MCP tokens for a running remote server");
  async function catalog(action, arguments_) {
    const response = await request.post("/api/mcp", {
      headers: { Authorization: `Bearer ${mcpToken}`, Accept: "application/json, text/event-stream" },
      data: { jsonrpc: "2.0", id: 1, method: "tools/call", params: { name: "aruvi_catalog", arguments: { action, arguments: arguments_ } } },
    });
    expect(response.ok()).toBeTruthy();
    const body = await response.json();
    expect(body.result.isError, JSON.stringify(body.result)).not.toBeTruthy();
    return body.result.structuredContent.result;
  }
  const name = `Tree test ${Date.now()}`;
  const product = await catalog("create_product", { name });
  try {
    const area = await catalog("create_product_area", { product_id: product.id, name: "Tree test area" });
    const capability = await catalog("create_capability", { product_area_id: area.id, name: "Tree test capability" });
    await catalog("create_capability", { product_area_id: area.id, parent_capability_id: capability.id, name: "Tree test child" });
    const treeRequests = [];
    page.on("request", (req) => { if (req.url().endsWith(`/products/${product.id}/tree`)) treeRequests.push(req); });
    await page.goto("/remote");
    await page.locator("#tokenInput").fill(token);
    await page.locator("#saveButton").click();
    await page.getByRole("button", { name: "Products", exact: true }).click();
    await page.getByRole("button", { name: new RegExp(name) }).click();
    await expect(page.getByRole("button", { name: "Load product tree" })).toBeVisible();
    expect(treeRequests).toHaveLength(0);
    await page.route(`**/products/${product.id}/tree`, (route) => route.fulfill({ status: 503, body: "Tree unavailable" }), { times: 1 });
    await page.getByRole("button", { name: "Load product tree" }).click();
    await expect(page.getByRole("button", { name: "Retry loading product tree" })).toBeEnabled();
    await page.getByRole("button", { name: "Retry loading product tree" }).click();
    await expect(page.locator("#productTree summary").first()).toContainText("Tree test area");
    await page.locator("#productTree summary").first().click();
    await page.locator("#productTree summary", { hasText: "Tree test capability" }).click();
    await expect(page.locator("#productTree summary", { hasText: "Tree test child" })).toBeVisible();
    expect(treeRequests).toHaveLength(2);
  } finally {
    await catalog("archive_product", { id: product.id });
  }
});

test("browser creates work with its full draft details", async ({ page, request }) => {
  test.skip(!token || !mcpToken, "Set mobile and MCP tokens for a running remote server");
  const headers = { Authorization: `Bearer ${token}` };
  async function mcp(name, action, arguments_) {
    const response = await request.post("/api/mcp", { headers: { Authorization: `Bearer ${mcpToken}`, Accept: "application/json, text/event-stream" }, data: { jsonrpc: "2.0", id: 1, method: "tools/call", params: { name, arguments: { action, arguments: arguments_ } } } });
    const result = (await response.json()).result;
    expect(result.isError, JSON.stringify(result)).not.toBeTruthy();
    return result.structuredContent.result;
  }
  const product = await mcp("aruvi_catalog", "create_product", { name: `Draft fields ${Date.now()}` });
  let item;
  try {
    await page.goto("/remote");
    await page.locator("#tokenInput").fill(token);
    await page.locator("#saveButton").click();
    await page.getByRole("button", { name: "Work", exact: true }).click();
    await page.locator("#workProductSelect").selectOption(product.id);
    await page.getByLabel("Work item title").fill("Fix an issue");
    await page.locator("#workProblemInput").fill("The result is wrong");
    await page.getByLabel("Outcome and context", { exact: true }).fill("Produce the expected result");
    await page.getByLabel("Acceptance criteria", { exact: true }).fill("The check passes");
    await page.locator("#workConstraintsInput").fill("Keep the API stable");
    await page.getByLabel("Work type").selectOption("bug");
    await page.locator("#workPriorityInput").selectOption("high");
    await page.locator("#workComplexityInput").selectOption("very_high");
    const created = page.waitForResponse((response) => response.url().endsWith("/api/mobile/work-items") && response.request().method() === "POST");
    await page.getByRole("button", { name: "Create work item", exact: true }).click();
    const response = await created; expect(response.status()).toBe(201);
    item = await response.json();
    const saved = await (await request.get(`/api/mobile/work-items/${item.id}`, { headers })).json();
    expect(saved).toMatchObject({ title: "Fix an issue", problem_statement: "The result is wrong", description: "Produce the expected result", acceptance_criteria: "The check passes", constraints: "Keep the API stable", work_item_type: "bug", priority: "high", complexity: "very_high", status: "draft" });
    await expect(page.getByLabel("Work type")).toHaveValue("task");
  } finally {
    if (item) await mcp("aruvi_work_items", "delete_work_item", { id: item.id });
    await mcp("aruvi_catalog", "archive_product", { id: product.id });
  }
});

test("browser places work in hierarchy and under a parent", async ({ page, request }) => {
  test.skip(!token || !mcpToken, "Set mobile and MCP tokens for a running remote server");
  const headers = { Authorization: `Bearer ${token}` };
  async function mcp(name, action, arguments_) {
    const response = await request.post("/api/mcp", { headers: { Authorization: `Bearer ${mcpToken}`, Accept: "application/json, text/event-stream" }, data: { jsonrpc: "2.0", id: 1, method: "tools/call", params: { name, arguments: { action, arguments: arguments_ } } } });
    const result = (await response.json()).result;
    expect(result.isError, JSON.stringify(result)).not.toBeTruthy();
    return result.structuredContent.result;
  }
  const product = await mcp("aruvi_catalog", "create_product", { name: `Placement ${Date.now()}` });
  const other = await mcp("aruvi_catalog", "create_product", { name: `Other placement ${Date.now()}` });
  let parent, child, scoped;
  try {
    const area = await mcp("aruvi_catalog", "create_product_area", { product_id: product.id, name: "Placement area" });
    const feature = await mcp("aruvi_catalog", "create_capability", { product_area_id: area.id, name: "Placement capability" });
    parent = await mcp("aruvi_work_items", "create_story", { product_id: product.id, source_node_id: feature.id, source_node_type: "capability", title: "Parent story" });
    const crossProduct = await request.post("/api/mobile/work-items", { headers, data: { product_id: other.id, title: "Wrong parent", parent_work_item_id: parent.id } });
    expect(crossProduct.status()).toBe(400);
    const wrongNode = await request.post("/api/mobile/work-items", { headers, data: { product_id: other.id, title: "Wrong node", source_node_id: feature.id, source_node_type: "capability" } });
    expect(wrongNode.status()).toBe(400);
    await page.goto("/remote");
    await page.locator("#tokenInput").fill(token); await page.locator("#saveButton").click();
    await page.getByRole("button", { name: "Work", exact: true }).click();
    await page.locator("#workProductSelect").selectOption(product.id);
    await page.getByRole("button", { name: "Load hierarchy and parent items" }).click();
    await expect(page.locator("#workScopeStatus")).toContainText("Child work inherits");
    await page.locator("#workScopeInput").selectOption(JSON.stringify({ id: feature.id, type: "capability" }));
    await page.locator("#workTitleInput").fill("Scoped item");
    let created = page.waitForResponse((r) => r.url().endsWith("/api/mobile/work-items") && r.request().method() === "POST");
    await page.locator("#createWorkButton").click(); scoped = await (await created).json();
    expect(scoped.source_node_id).toBe(feature.id);
    await page.getByRole("button", { name: "Load hierarchy and parent items" }).click();
    await expect(page.locator("#workScopeStatus")).toContainText("Child work inherits");
    await page.locator("#workParentInput").selectOption(parent.id);
    await expect(page.locator("#workScopeInput")).toBeDisabled();
    await page.locator("#workTitleInput").fill("Child item");
    created = page.waitForResponse((r) => r.url().endsWith("/api/mobile/work-items") && r.request().method() === "POST");
    await page.locator("#createWorkButton").click(); child = await (await created).json();
    expect(child).toMatchObject({ parent_work_item_id: parent.id, source_node_id: feature.id, source_node_type: "capability" });
  } finally {
    for (const item of [child, scoped, parent]) if (item?.id) await mcp("aruvi_work_items", "delete_work_item", { id: item.id });
    await mcp("aruvi_catalog", "archive_product", { id: other.id });
    await mcp("aruvi_catalog", "archive_product", { id: product.id });
  }
});

test("browser edits task details and preserves conflicting edits", async ({ page, request }) => {
  test.skip(!token || !mcpToken, "Set mobile and MCP tokens for a running remote server");
  const headers = { Authorization: `Bearer ${token}` };
  async function mcp(name, action, arguments_) {
    const response = await request.post("/api/mcp", {
      headers: { Authorization: `Bearer ${mcpToken}`, Accept: "application/json, text/event-stream" },
      data: { jsonrpc: "2.0", id: 1, method: "tools/call", params: { name, arguments: { action, arguments: arguments_ } } },
    });
    const result = (await response.json()).result;
    expect(result.isError, JSON.stringify(result)).not.toBeTruthy();
    return result.structuredContent.result;
  }
  const product = await mcp("aruvi_catalog", "create_product", { name: `Edit task ${Date.now()}` });
  let item;
  try {
    const created = await request.post("/api/mobile/work-items", { headers, data: { product_id: product.id, title: "Original task", description: "Original description", acceptance_criteria: "Original criteria" } });
    expect(created.status()).toBe(201);
    item = await created.json();
    const fields = { title: item.title, description: item.description, problem_statement: item.problem_statement, acceptance_criteria: item.acceptance_criteria, constraints: item.constraints };
    expect((await request.post(`/api/mobile/work-items/${item.id}`, { data: { expected: fields, fields } })).status()).toBe(401);
    expect((await request.post(`/api/mobile/work-items/${item.id}`, { headers, data: { expected: fields, fields: { ...fields, title: "  " } } })).status()).toBe(400);
    expect((await request.post(`/api/mobile/work-items/${item.id}`, { headers, data: { expected: fields, fields: { ...fields, status: "done" } } })).status()).toBe(400);
    expect((await request.post(`/api/mobile/work-items/${item.id}`, { headers, data: { expected: { ...fields, status: "draft" }, fields: { ...fields, status: "invented" } } })).status()).toBe(422);
    await mcp("aruvi_work_items", "update_work_item", { id: item.id, status: "blocked" });
    expect((await request.post(`/api/mobile/work-items/${item.id}`, { headers, data: { expected: { ...fields, status: "draft" }, fields: { ...fields, status: "done", title: "Must not save" } } })).status()).toBe(409);
    expect((await (await request.get(`/api/mobile/work-items/${item.id}`, { headers })).json()).title).toBe("Original task");
    const legacy = await request.post(`/api/mobile/work-items/${item.id}`, { headers, data: { expected: fields, fields: { ...fields, title: "Legacy save" } } });
    expect(legacy.status()).toBe(200);
    expect((await legacy.json()).status).toBe("blocked");
    await mcp("aruvi_work_items", "update_work_item", { id: item.id, title: "Original task", status: "draft" });
    await page.goto("/remote");
    await page.locator("#tokenInput").fill(token);
    await page.locator("#saveButton").click();
    await page.getByRole("button", { name: "Work", exact: true }).click();
    await page.locator("#workProductSelect").selectOption(product.id);
    await expect(page.locator("#deliveryDetails")).toContainText("Original task");
    await page.getByRole("button", { name: "Edit task", exact: true }).click();
    await page.getByLabel("Edit title").fill("Updated task");
    await page.getByLabel("Edit outcome and context").fill("Updated description");
    await page.getByLabel("Edit acceptance criteria").fill("Updated criteria");
    await page.getByLabel("Constraints", { exact: true }).fill("Keep the API stable");
    await page.getByLabel("Edit status").selectOption("blocked");
    await page.getByRole("button", { name: "Save task", exact: true }).click();
    await expect(page.locator("#status")).toHaveText("Task saved");
    await expect(page.locator("#deliveryDetails")).toContainText("Updated criteria");
    const saved = await (await request.get(`/api/mobile/work-items/${item.id}`, { headers })).json();
    expect(saved.title).toBe("Updated task");
    expect(saved.constraints).toBe("Keep the API stable");
    expect(saved.status).toBe("blocked");
    await page.getByRole("button", { name: "Edit task", exact: true }).click();
    await page.getByLabel("Edit outcome and context").fill("Unsaved browser edit");
    await mcp("aruvi_work_items", "update_work_item", { id: item.id, description: "Concurrent MCP edit" });
    const conflict = page.waitForResponse((response) => response.url().endsWith(`/api/mobile/work-items/${item.id}`) && response.request().method() === "POST");
    await page.getByRole("button", { name: "Save task", exact: true }).click();
    expect((await conflict).status()).toBe(409);
    await expect(page.getByLabel("Edit outcome and context")).toHaveValue("Unsaved browser edit");
    expect((await (await request.get(`/api/mobile/work-items/${item.id}`, { headers })).json()).description).toBe("Concurrent MCP edit");
  } finally {
    if (item) await mcp("aruvi_work_items", "delete_work_item", { id: item.id });
    await mcp("aruvi_catalog", "archive_product", { id: product.id });
  }
});

test("browser builds a product hierarchy", async ({ page, request }) => {
  test.skip(!token || !mcpToken, "Set mobile and MCP tokens for a running remote server");
  async function catalog(action, arguments_) {
    const response = await request.post("/api/mcp", {
      headers: { Authorization: `Bearer ${mcpToken}`, Accept: "application/json, text/event-stream" },
      data: { jsonrpc: "2.0", id: 1, method: "tools/call", params: { name: "aruvi_catalog", arguments: { action, arguments: arguments_ } } },
    });
    const result = (await response.json()).result;
    expect(result.isError, JSON.stringify(result)).not.toBeTruthy();
    return result.structuredContent.result;
  }
  const product = await catalog("create_product", { name: `Hierarchy editor ${Date.now()}` });
  const other = await catalog("create_product", { name: `Other hierarchy ${Date.now()}` });
  try {
    const otherArea = await catalog("create_product_area", { product_id: other.id, name: "Other area" });
    const url = `/api/mobile/products/${product.id}/hierarchy`;
    expect((await request.post(url, { data: { name: "Unauthorized" } })).status()).toBe(401);
    expect((await request.post(url, { headers: { Authorization: `Bearer ${token}` }, data: { name: "Wrong scope", product_area_id: otherArea.id } })).status()).toBe(400);
    await page.goto("/remote");
    await page.locator("#tokenInput").fill(token);
    await page.locator("#saveButton").click();
    await page.getByRole("button", { name: "Products", exact: true }).click();
    await page.locator("#productList").getByRole("button", { name: new RegExp(product.name) }).click();
    await expect(page.locator("#productStatus")).toHaveText(product.name);
    await page.getByLabel("New product area").fill("Browser area");
    await page.getByRole("button", { name: "Add product area" }).click();
    await expect(page.locator("#productTree")).toContainText("product area: Browser area");
    await page.locator("#productTree summary").filter({ hasText: "Browser area" }).click();
    await page.getByLabel("New capability in Browser area").fill("Browser capability");
    await page.getByRole("button", { name: "Add capability" }).click();
    await expect(page.locator("#productTree")).toContainText("Browser capability");
    await page.locator("#productTree summary").filter({ hasText: "Browser area" }).click();
    await page.locator("#productTree summary").filter({ hasText: "Browser capability" }).click();
    await page.getByLabel("New feature in Browser capability").fill("Browser feature");
    await page.getByRole("button", { name: "Add feature" }).click();
    await expect(page.locator("#productTree")).toContainText("Browser feature");
    const treeResponse = await request.get(`/api/mobile/products/${product.id}/tree`, { headers: { Authorization: `Bearer ${token}` } });
    const tree = await treeResponse.json();
    const area = tree.product_areas.find((entry) => entry.product_area.name === "Browser area");
    expect(area.features[0].capability.name).toBe("Browser capability");
    expect(area.features[0].children[0].capability.name).toBe("Browser feature");
    const update = { node_type: "product_area", id: area.product_area.id, expected_name: "Browser area", expected_description: "", name: "Wrong scope", description: "" };
    expect((await request.post(`/api/mobile/products/${product.id}/hierarchy/update`, { data: update })).status()).toBe(401);
    expect((await request.post(`/api/mobile/products/${other.id}/hierarchy/update`, { headers: { Authorization: `Bearer ${token}` }, data: update })).status()).toBe(409);
    await page.locator("#productTree summary").filter({ hasText: "Browser area" }).click();
    await page.locator("#productTree summary").filter({ hasText: "Browser capability" }).click();
    const feature = page.locator("#productTree summary").filter({ hasText: "Browser feature" }).locator("..");
    await feature.locator(":scope > summary").click();
    await feature.getByText("Edit details", { exact: true }).click();
    await feature.getByLabel("Edit name for Browser feature").fill("Renamed feature");
    await feature.getByLabel("Edit description for Browser feature").fill("Browser-edited description");
    await feature.getByRole("button", { name: "Save details" }).click();
    await expect(page.locator("#productTree")).toContainText("Renamed feature");
    const current = await (await request.get(`/api/mobile/products/${product.id}/tree`, { headers: { Authorization: `Bearer ${token}` } })).json();
    expect(current.product_areas[0].features[0].children[0].capability.description).toBe("Browser-edited description");
    await page.locator("#productTree summary").filter({ hasText: "Browser area" }).click();
    const areaElement = page.locator("#productTree > details").first();
    const areaEdit = areaElement.locator(":scope > details > summary").filter({ hasText: "Edit details" }).locator("..");
    await areaEdit.locator(":scope > summary").click();
    await areaElement.getByLabel("Edit name for Browser area").fill("Unsaved area edit");
    await catalog("update_product_area", { id: area.product_area.id, description: "Concurrent description" });
    const conflict = page.waitForResponse((response) => response.url().endsWith("/hierarchy/update") && response.request().method() === "POST");
    await areaEdit.getByRole("button", { name: "Save details" }).click();
    expect((await conflict).status()).toBe(409);
    await expect(areaElement.getByLabel("Edit name for Browser area")).toHaveValue("Unsaved area edit");
    const latest = await (await request.get(`/api/mobile/products/${product.id}/tree`, { headers: { Authorization: `Bearer ${token}` } })).json();
    expect(latest.product_areas[0].product_area.description).toBe("Concurrent description");
    expect(latest.product_areas[0].product_area.name).toBe("Browser area");
  } finally {
    await catalog("archive_product", { id: product.id });
    await catalog("archive_product", { id: other.id });
  }
});

test("hierarchy deletion reconciles related data", async ({ page, request }) => {
  test.skip(!token || !mcpToken || !process.env.ARUVI_TEST_REPOSITORY_PATH, "Requires tokens and a disposable repository");
  async function call(name, action, arguments_) {
    const response = await request.post("/api/mcp", {
      headers: { Authorization: `Bearer ${mcpToken}`, Accept: "application/json, text/event-stream" },
      data: { jsonrpc: "2.0", id: 1, method: "tools/call", params: { name, arguments: { action, arguments: arguments_ } } },
    });
    const result = (await response.json()).result;
    expect(result.isError, JSON.stringify(result)).not.toBeTruthy();
    return result.structuredContent.result;
  }
  const product = await call("aruvi_catalog", "create_product", { name: `Delete hierarchy ${Date.now()}` });
  let repo, team, featureWork, areaWork, reference, areaReference;
  try {
    const area = await call("aruvi_catalog", "create_product_area", { product_id: product.id, name: "Delete area" });
    const other = await call("aruvi_catalog", "create_product_area", { product_id: product.id, name: "Keep area" });
    const capability = await call("aruvi_catalog", "create_capability", { product_area_id: area.id, name: "Delete capability" });
    const feature = await call("aruvi_catalog", "create_capability", { product_area_id: area.id, parent_capability_id: capability.id, name: "Delete feature" });
    featureWork = await call("aruvi_work_items", "create_work_item", { product_id: product.id, product_area_id: area.id, capability_id: feature.id, source_node_id: feature.id, source_node_type: "capability", title: "Keep feature work" });
    areaWork = await call("aruvi_work_items", "create_work_item", { product_id: product.id, product_area_id: area.id, source_node_id: area.id, source_node_type: "product_area", title: "Keep area work" });
    reference = await call("aruvi_catalog", "create_reference", { scope_type: "feature", scope_id: feature.id, title: "Keep feature reference" });
    areaReference = await call("aruvi_catalog", "create_reference", { scope_type: "product_area", scope_id: area.id, title: "Keep area reference" });
    team = await call("aruvi_agents", "create_agent_team", { name: `Delete test team ${Date.now()}` });
    await call("aruvi_agents", "assign_team_scope", { team_id: team.id, scope_type: "capability", scope_id: feature.id });
    await call("aruvi_agents", "assign_team_scope", { team_id: team.id, scope_type: "product_area", scope_id: area.id });
    await call("aruvi_agents", "assign_team_scope", { team_id: team.id, scope_type: "product_area", scope_id: other.id });
    repo = await call("aruvi_repositories", "register_repository", { name: "Deletion test repository", local_path: process.env.ARUVI_TEST_REPOSITORY_PATH });
    await call("aruvi_repositories", "attach_repository", { scope_type: "product_area", scope_id: area.id, repository_id: repo.id });
    const url = `/api/mobile/products/${product.id}/hierarchy/delete`;
    const body = { node_type: "capability", id: capability.id, expected_name: capability.name };
    expect((await request.post(url, { data: body })).status()).toBe(401);
    expect((await request.post(url, { headers: { Authorization: `Bearer ${token}` }, data: { ...body, expected_name: "stale" } })).status()).toBe(409);
    await page.goto("/remote");
    await page.locator("#tokenInput").fill(token);
    await page.locator("#saveButton").click();
    await page.getByRole("button", { name: "Products", exact: true }).click();
    await page.locator("#productList").getByRole("button", { name: new RegExp(product.name) }).click();
    await expect(page.locator("#productStatus")).toHaveText(product.name);
    await page.getByRole("button", { name: "Load product tree" }).click();
    await page.locator("#productTree summary").filter({ hasText: "Delete area" }).click();
    const cap = page.locator("#productTree summary").filter({ hasText: "Delete capability" }).locator("..");
    await cap.locator(":scope > summary").click();
    const capDelete = cap.locator(":scope > details > summary").filter({ hasText: "Delete node" }).locator("..");
    await capDelete.locator(":scope > summary").click();
    await capDelete.getByLabel("Type name to delete Delete capability").fill("wrong name");
    await capDelete.getByRole("checkbox").check();
    await capDelete.getByRole("button", { name: "Delete node", exact: true }).click();
    await expect(page.locator("#status")).toContainText("Type the exact node name");
    await capDelete.getByLabel("Type name to delete Delete capability").fill("Delete capability");
    await capDelete.getByRole("button", { name: "Delete node", exact: true }).click();
    await expect(page.locator("#status")).toHaveText("Hierarchy node deleted");
    let tree = await call("aruvi_catalog", "get_product_tree", { product_id: product.id });
    expect(tree.product_areas.find((entry) => entry.product_area.id === area.id).features).toEqual([]);
    expect(tree.product_areas.some((entry) => entry.product_area.id === other.id)).toBe(true);
    let work = await call("aruvi_work_items", "get_work_item", { id: featureWork.id });
    expect(work.capability_id).toBeNull(); expect(work.source_node_id).toBeNull(); expect(work.product_area_id).toBe(area.id);
    let refs = await call("aruvi_catalog", "list_references", { scope_type: "product", scope_id: product.id });
    expect(refs.some((entry) => entry.id === reference.id)).toBe(true);
    let assignments = await call("aruvi_agents", "list_team_assignments", {});
    expect(assignments.some((entry) => entry.team_id === team.id && entry.scope_id === feature.id)).toBe(false);
    await page.locator("#productTree summary").filter({ hasText: "Delete area" }).click();
    const areaElement = page.locator("#productTree summary").filter({ hasText: "Delete area" }).locator("..");
    const areaDelete = areaElement.locator(":scope > details > summary").filter({ hasText: "Delete node" }).locator("..");
    await areaDelete.locator(":scope > summary").click();
    await areaDelete.getByLabel("Type name to delete Delete area").fill("Delete area");
    await areaDelete.getByRole("checkbox").check();
    await areaDelete.getByRole("button", { name: "Delete node", exact: true }).click();
    await expect(page.locator("#productTree")).not.toContainText("Delete area");
    tree = await call("aruvi_catalog", "get_product_tree", { product_id: product.id });
    expect(tree.product_areas.map((entry) => entry.product_area.id)).toEqual([other.id]);
    work = await call("aruvi_work_items", "get_work_item", { id: areaWork.id });
    expect(work.product_area_id).toBeNull(); expect(work.source_node_id).toBeNull();
    refs = await call("aruvi_catalog", "list_references", { scope_type: "product", scope_id: product.id });
    expect(refs.some((entry) => entry.id === areaReference.id)).toBe(true);
    assignments = await call("aruvi_agents", "list_team_assignments", {});
    expect(assignments.some((entry) => entry.team_id === team.id && entry.scope_id === area.id)).toBe(false);
    expect(assignments.some((entry) => entry.team_id === team.id && entry.scope_id === other.id)).toBe(true);
    expect(await call("aruvi_repositories", "list_repository_attachments", { scope_type: "product_area", scope_id: area.id })).toEqual([]);
    const mcpParent = await call("aruvi_catalog", "create_capability", { product_area_id: other.id, name: "MCP parent" });
    await call("aruvi_catalog", "create_capability", { product_area_id: other.id, parent_capability_id: mcpParent.id, name: "MCP child" });
    await call("aruvi_catalog", "delete_capability", { id: mcpParent.id });
    tree = await call("aruvi_catalog", "get_product_tree", { product_id: product.id });
    expect(tree.product_areas[0].features).toEqual([]);
  } finally {
    if (featureWork) await call("aruvi_work_items", "delete_work_item", { id: featureWork.id });
    if (areaWork) await call("aruvi_work_items", "delete_work_item", { id: areaWork.id });
    if (reference) await call("aruvi_catalog", "delete_reference", { id: reference.id });
    if (areaReference) await call("aruvi_catalog", "delete_reference", { id: areaReference.id });
    if (team) await call("aruvi_agents", "delete_agent_team", { id: team.id });
    if (repo) await call("aruvi_repositories", "delete_repository", { id: repo.id });
    await call("aruvi_catalog", "archive_product", { id: product.id });
  }
});

test("browser resets a product plan with and without delivery", async ({ page, request }) => {
  test.skip(!token || !mcpToken, "Set mobile and MCP tokens for a running remote server");
  async function call(name, action, arguments_) {
    const response = await request.post("/api/mcp", {
      headers: { Authorization: `Bearer ${mcpToken}`, Accept: "application/json, text/event-stream" },
      data: { jsonrpc: "2.0", id: 1, method: "tools/call", params: { name, arguments: { action, arguments: arguments_ } } },
    });
    const result = (await response.json()).result;
    expect(result.isError, JSON.stringify(result)).not.toBeTruthy();
    return result.structuredContent.result;
  }
  const product = await call("aruvi_catalog", "create_product", { name: `Plan reset ${Date.now()}` });
  let work, reference, secondWork;
  try {
    const area = await call("aruvi_catalog", "create_product_area", { product_id: product.id, name: "Reset area" });
    const cap = await call("aruvi_catalog", "create_capability", { product_area_id: area.id, name: "Reset capability" });
    work = await call("aruvi_work_items", "create_work_item", { product_id: product.id, product_area_id: area.id, capability_id: cap.id, source_node_id: cap.id, source_node_type: "capability", title: "Preserved work" });
    reference = await call("aruvi_catalog", "create_reference", { scope_type: "capability", scope_id: cap.id, title: "Preserved reset reference" });
    const url = `/api/mobile/products/${product.id}/reset-plan`;
    expect((await request.post(url, { data: { expected_name: product.name, delete_delivery: false } })).status()).toBe(401);
    expect((await request.post(url, { headers: { Authorization: `Bearer ${token}` }, data: { expected_name: "stale", delete_delivery: false } })).status()).toBe(400);
    await page.goto("/remote");
    await page.locator("#tokenInput").fill(token);
    await page.locator("#saveButton").click();
    await page.getByRole("button", { name: "Products", exact: true }).click();
    await page.locator("#productList").getByRole("button", { name: new RegExp(product.name) }).click();
    await expect(page.locator("#productStatus")).toHaveText(product.name);
    await page.getByText("Reset product plan", { exact: true }).click();
    await page.getByLabel("Type product name to reset").fill("wrong");
    await page.getByLabel("I understand the hierarchy will be deleted.").check();
    await page.getByRole("button", { name: "Reset plan", exact: true }).click();
    await expect(page.locator("#status")).toContainText("Type the exact product name");
    await page.getByLabel("Type product name to reset").fill(product.name);
    await page.getByRole("button", { name: "Reset plan", exact: true }).click();
    await expect(page.locator("#status")).toContainText("Plan reset: 1 areas and 1 nodes removed");
    let tree = await call("aruvi_catalog", "get_product_tree", { product_id: product.id });
    expect(tree.product_areas).toEqual([]);
    let retained = await call("aruvi_work_items", "get_work_item", { id: work.id });
    expect(retained.capability_id).toBeNull(); expect(retained.source_node_id).toBeNull();
    let refs = await call("aruvi_catalog", "list_references", { scope_type: "product", scope_id: product.id });
    expect(refs.some((item) => item.id === reference.id)).toBe(true);
    const nextArea = await call("aruvi_catalog", "create_product_area", { product_id: product.id, name: "Second area" });
    secondWork = await call("aruvi_work_items", "create_work_item", { product_id: product.id, product_area_id: nextArea.id, title: "Delete this work" });
    await page.getByText("Reset product plan", { exact: true }).click();
    await page.getByLabel("Type product name to reset").fill(product.name);
    await page.getByLabel("I understand the hierarchy will be deleted.").check();
    await page.getByLabel("Also delete delivery work and agent-work records.").check();
    await page.getByRole("button", { name: "Reset plan", exact: true }).click();
    await expect(page.locator("#status")).toContainText("Plan reset: 1 areas and 0 nodes removed");
    const list = await (await request.get(`/api/mobile/work-items?product_id=${product.id}`, { headers: { Authorization: `Bearer ${token}` } })).json();
    expect(list.items).toEqual([]);
    work = null; secondWork = null;
    await call("aruvi_catalog", "create_product_area", { product_id: product.id, name: "MCP reset area" });
    const result = await call("aruvi_catalog", "reset_product_plan", { product_id: product.id, delete_delivery: false });
    expect(result.productAreasDeleted).toBe(1);
  } finally {
    if (work) await call("aruvi_work_items", "delete_work_item", { id: work.id });
    if (secondWork) await call("aruvi_work_items", "delete_work_item", { id: secondWork.id });
    if (reference) await call("aruvi_catalog", "delete_reference", { id: reference.id });
    await call("aruvi_catalog", "archive_product", { id: product.id });
  }
});

test("browser and MCP edit an Anthropic provider protocol", async ({ page, request }) => {
  test.skip(!token || !mcpToken, "Set mobile and MCP tokens for a running remote server");
  const name = `Claude fixture ${Date.now()}`;
  let provider;
  try {
    await page.goto("/remote");
    await page.locator("#tokenInput").fill(token);
    await page.locator("#saveButton").click();
    await page.getByRole("button", { name: "Models", exact: true }).click();
    await page.getByLabel("Provider name", { exact: true }).fill(name);
    await page.locator("#providerProtocol").selectOption("anthropic");
    await page.getByLabel("API base URL").fill("https://api.anthropic.com");
    await page.getByLabel("API key (optional)").fill("synthetic-anthropic-capability-key");
    const created = page.waitForResponse((response) => response.url().endsWith("/api/mobile/providers") && response.request().method() === "POST");
    await page.getByRole("button", { name: "Add provider", exact: true }).click();
    const response = await created;
    expect(response.ok()).toBeTruthy();
    provider = await response.json();
    expect(provider.protocol).toBe("anthropic");
    await expect(page.locator("#providerList")).toContainText(`${name} · anthropic`);
    const card = page.locator("#providerList .tool-card").filter({ hasText: name });
    const anthropicTransport = await (await request.get(`/api/mobile/providers/${provider.id}/transport-capabilities`, { headers: { Authorization: `Bearer ${token}` } })).json();
    expect(anthropicTransport).toEqual({ text_completion: true, text_streaming: true, model_discovery: true, transcription: false, speech: false, json_object_completion: false });
    await card.getByRole("button", { name: "Show transport capabilities" }).click();
    await expect(card).toContainText("Transport supports: text, streaming, model discovery.");
    await card.getByText("Manage provider", { exact: true }).click();
    const protocol = card.getByLabel(`Edit provider protocol ${name}`);
    await expect(protocol).toHaveValue("anthropic");
    await protocol.selectOption("openai_compatible");
    const saved = page.waitForResponse((r) => r.url().endsWith("/api/mobile/providers/update"));
    await card.getByRole("button", { name: "Save provider" }).click();
    expect((await saved).ok()).toBeTruthy();
    await expect(page.locator("#providerList")).toContainText(`${name} · openai_compatible`);
    const expected = { name, base_url: "https://api.anthropic.com", enabled: true, protocol: "anthropic" };
    const stale = await request.post("/api/mobile/providers/update", { headers: { Authorization: `Bearer ${token}` }, data: { id: provider.id, expected, fields: expected } });
    expect(stale.status()).toBe(409);
    const mcp = await request.post("/api/mcp", { headers: { Authorization: `Bearer ${mcpToken}` }, data: { jsonrpc: "2.0", id: 1, method: "tools/call", params: { name: "aruvi_models", arguments: { action: "update_provider", arguments: { id: provider.id, protocol: "anthropic" } } } } });
    const result = (await mcp.json()).result;
    expect(result.isError).not.toBeTruthy();
    expect(result.structuredContent.result.protocol).toBe("anthropic");
    const invalid = await request.post("/api/mobile/providers/update", { headers: { Authorization: `Bearer ${token}` }, data: { id: provider.id, expected, fields: { ...expected, protocol: "unknown" } } });
    expect(invalid.status()).toBe(400);
  } finally {
    if (provider) {
      const response = await request.post("/api/mobile/providers/delete", { headers: { Authorization: `Bearer ${token}` }, data: { id: provider.id, expected_name: name } });
      expect(response.ok()).toBeTruthy();
    }
  }
});

test("planner analyzes a selected workspace into the selected product", async ({ page }) => {
  test.skip(!token, "Requires remote fixture token");
  await page.addInitScript((value) => localStorage.setItem("aruvi.remote.token", value), token);
  await page.route("**/api/mobile/products", (route) => route.fulfill({ json: [{ id: "product-a", name: "Product A" }] }));
  await page.route("**/api/mobile/repositories", (route) => route.fulfill({ json: [{ id: "repo-a", name: "Workspace A" }] }));
  await page.route("**/api/mobile/planner/sessions", (route) => route.fulfill({ json: { session_id: "analysis-session" } }));
  let release;
  let received;
  const pending = new Promise((resolve) => { received = resolve; });
  await page.route("**/api/mobile/planner/sessions/*/analyze-repository", async (route) => {
    received(route.request().postDataJSON());
    await new Promise((resolve) => { release = resolve; });
    await route.fulfill({ json: { session_id: "analysis-session", status: "proposal", assistant_message: "Workspace analyzed", draft_tree_nodes: [{ id: "root", label: "Product A", children: [{ id: "area", label: "Inventory", node_type: "product_area", children: [] }] }] } });
  });
  await page.goto("/remote");
  await expect(page.locator("#plannerProductSelect")).toBeEnabled();
  await page.locator("#refreshPlannerRepositoriesButton").click();
  await page.locator("#plannerRepositorySelect").selectOption("repo-a");
  await page.locator("#analyzePlannerRepositoryButton").click();
  await expect(page.locator("#status")).toContainText("Select a planner product");
  await page.locator("#plannerProductSelect").selectOption("product-a");
  await page.locator("#analyzePlannerRepositoryButton").click();
  try {
    expect(await pending).toEqual({ repository_id: "repo-a", product_id: "product-a", selected_draft_node_id: null });
    await expect(page.locator("#plannerProductSelect")).toBeDisabled();
    await expect(page.locator("#plannerRepositorySelect")).toBeDisabled();
  } finally { if (release) release(); }
  await expect(page.getByRole("button", { name: /Inventory/ })).toBeVisible();
  await expect(page.locator("#status")).toContainText("Workspace analyzed");
});
