import { test, expect } from "@playwright/test";

for (const scenario of ["passed", "failed", "skipped"]) test(`isolated workflow records ${scenario} test execution`, async ({ page, request }) => {
  test.skip(process.env.ARUVI_ISOLATED_WORKFLOW !== "1", "Requires a disposable isolated server and simulated provider");
  test.setTimeout(180000);
  const mobileToken = process.env.ARUVI_MOBILE_API_TOKEN;
  const mcpToken = process.env.ARUVI_MCP_API_TOKEN;
  if (mobileToken !== "workflow-mobile-test" || mcpToken !== "workflow-mcp-test" || process.env.ARUVI_TEST_REPOSITORY_PATH !== "/repositories/workflow-test") {
    throw new Error("The workflow test must use its disposable server, tokens, and repository.");
  }
  const headers = { Authorization: `Bearer ${mobileToken}` };
  async function api(path, data) {
    const response = await request.post(path, { headers, data });
    expect(response.ok(), await response.text()).toBeTruthy();
    return response.json();
  }
  async function mcp(name, action, arguments_) {
    const response = await request.post("/api/mcp", {
      headers: { Authorization: `Bearer ${mcpToken}`, Accept: "application/json, text/event-stream" },
      data: { jsonrpc: "2.0", id: 1, method: "tools/call", params: { name, arguments: { action, arguments: arguments_ } } },
    });
    const result = (await response.json()).result;
    expect(result.isError, JSON.stringify(result)).not.toBeTruthy();
    return result.structuredContent.result;
  }
  await request.post("http://127.0.0.1:19876/workflow/reset");
  const provider = await api("/api/mobile/providers", { name: "Workflow mock", base_url: process.env.ARUVI_TEST_MODEL_URL.replace(/\/v1\/?$/, "/workflow/v1") });
  const model = await api("/api/mobile/models", { provider_id: provider.id, name: "browser-test-model" });
  const agents = await (await request.get("/api/mobile/agents", { headers })).json();
  for (const agent of agents.filter((item) => item.enabled)) {
    await api("/api/mobile/agent-model-bindings", { agent_id: agent.id, model_id: model.id });
  }
  await mcp("aruvi_settings", "set_setting", { key: "workflow.auto_approve_plan", value: "false" });
  await mcp("aruvi_settings", "set_setting", { key: "workflow.auto_approve_test_review", value: scenario === "skipped" ? "true" : "false" });
  await api("/api/mobile/workflow-test-config", { argv: ["/bin/false"] });
  const product = await api("/api/mobile/products", { name: `Isolated workflow ${scenario}` });
  let assignedTeam, assignedAgent, codingPolicy;
  if (scenario === "passed") {
    assignedTeam = await api("/api/mobile/teams", { name: "Workflow owner", department: "Engineering", description: "", enabled: true, max_concurrent_workflows: 2 });
    await api("/api/mobile/team-assignments", { team_id: assignedTeam.id, product_id: product.id, scope_type: "product", scope_id: product.id });
    assignedAgent = await api("/api/mobile/agents", { name: "Workflow coding specialist", role: "specialist_coder", description: "", prompt_template_ref: "", allowed_tools: ["repo.read_file", "repo.write_file"], skill_tags: [], boundaries: { json_object_output: true }, enabled: true, employment_status: "active" });
    await api("/api/mobile/team-members", { team_id: assignedTeam.id, agent_id: assignedAgent.id, title: "Developer", is_lead: false });
    await api("/api/mobile/agent-model-bindings", { agent_id: assignedAgent.id, model_id: model.id });
    codingPolicy = await api("/api/mobile/routing-policies", { stage_name: "coding", expected: null, fields: { primary_roles: ["specialist_coder"], fallback_roles: ["developer"], coordinator_required: false } });
    const activeSkill = await api("/api/mobile/skills", { name: "Workflow active skill", category: "test", description: "", instructions: "Follow the synthetic skill instruction.", enabled: true });
    const disabledSkill = await api("/api/mobile/skills", { name: "Workflow disabled skill", category: "test", description: "", instructions: "Do not include this synthetic instruction.", enabled: false });
    for (const skill of [activeSkill, disabledSkill]) await api("/api/mobile/skill-links", { skill_id: skill.id, target_type: "team", target_id: assignedTeam.id });
  }
  const repository = await api("/api/mobile/repositories", { name: "Isolated workflow repo", local_path: `${process.env.ARUVI_TEST_REPOSITORY_PATH}-${scenario}` });
  await api("/api/mobile/repository-attachments", { product_id: product.id, repository_id: repository.id });

  await page.goto("/remote");
  await page.locator("#tokenInput").fill(mobileToken);
  await page.locator("#saveButton").click();
  await page.getByRole("button", { name: "Work", exact: true }).click();
  await page.locator("#workProductSelect").selectOption(product.id);
  await expect(page.locator("#workflowTestScopeStatus")).toContainText(`Settings for Isolated workflow ${scenario}`);
  await page.locator("#workflowTestMode").selectOption(scenario === "skipped" ? "disabled" : "custom");
  const command = scenario === "passed" ? ["/bin/sh", "-c", "test -z \"${ARUVI_MOBILE_API_TOKEN-}\" && test \"$(cat src/generated.txt)\" = \"generated by workflow test\""] : scenario === "failed" ? ["/bin/sh", "-c", "printf 'test failure'; head -c 50000 /dev/zero; exit 9"] : [];
  if (scenario !== "skipped") await page.locator("#workflowTestArgv").fill(JSON.stringify(command));
  await page.getByRole("button", { name: "Save test command" }).click();
  await expect(page.locator("#status")).toHaveText("Workflow test command saved");
  await page.locator("#workTitleInput").fill("Generate a test file");
  await page.locator("#workDescriptionInput").fill("Create a small generated file in the isolated repository.");
  await page.locator("#workAcceptanceInput").fill("The file contains the expected test text.");
  const created = page.waitForResponse((response) => response.url().endsWith("/api/mobile/work-items") && response.request().method() === "POST");
  await page.locator("#createWorkButton").click();
  const workItem = await (await created).json();
  await expect(page.locator("#deliveryStatus")).toHaveText("draft");
  const started = page.waitForResponse((response) => response.url().endsWith(`/work-items/${workItem.id}/approve`), { timeout: 60000 });
  await page.locator("#approveWorkButton").click();
  const startResponse = await started;
  expect(startResponse.ok(), await startResponse.text()).toBeTruthy();
  if (assignedTeam) expect((await startResponse.json()).workflow_run.assigned_team_id).toBe(assignedTeam.id);
  await expect(page.locator("#deliveryStatus")).toHaveText("pending plan approval", { timeout: 60000 });
  const advanced = page.waitForResponse((response) => response.url().includes("/api/mobile/workflows/") && response.url().endsWith("/action"), { timeout: 90000 });
  await page.getByRole("button", { name: "Approve Gate" }).click();
  const advanceResponse = await advanced;
  if (scenario === "failed") {
    expect(advanceResponse.ok()).toBeFalsy();
    await expect(page.locator("#deliveryStatus")).toHaveText("failed", { timeout: 90000 });
  } else {
    expect(advanceResponse.ok(), await advanceResponse.text()).toBeTruthy();
    await expect(page.locator("#deliveryStatus")).toHaveText("pending test review", { timeout: 90000 });
  }
  const delivery = await (await request.get(`/api/mobile/work-items/${workItem.id}/delivery`, { headers })).json();
  expect(delivery.artifacts.some((artifact) => artifact.artifact_type === "coding_applied_files")).toBeTruthy();
  const testResult = delivery.artifacts.find((artifact) => artifact.artifact_type === "workflow_test_result");
  expect(testResult.summary).toBe(scenario === "passed" ? "Configured test command passed" : scenario === "failed" ? "Configured test command failed" : "Tests were not run; manual test review is required");
  await page.getByRole("button", { name: "Show test result" }).click();
  await expect(page.locator("#deliveryDetails")).toContainText(`"status": "${scenario}"`);
  if (scenario === "failed") {
    await expect(page.locator("#deliveryDetails")).toContainText("test failure");
    await expect(page.locator("#deliveryDetails")).toContainText("[output truncated]");
  }
  if (assignedTeam) {
    const calls = await (await request.get("/api/mobile/model-calls?limit=100", { headers })).json();
    const snapshotCall = calls.find((item) => item.workflow_run_id === delivery.workflow_run.id && item.agent_id === assignedAgent.id && item.stage === "coding" && item.request_snapshot_path && item.response_snapshot_path);
    expect(snapshotCall).toBeTruthy();
    const snapshotPath = `/api/mobile/model-calls/${snapshotCall.id}/snapshots/request`;
    expect((await request.get(snapshotPath)).status()).toBe(401);
    expect((await request.get(`/api/mobile/model-calls/${snapshotCall.id}/snapshots/invalid`, { headers })).status()).toBe(400);
    const snapshot = await (await request.get(snapshotPath, { headers })).json();
    expect(snapshot.content).toContain("synthetic skill instruction");
    expect(await mcp("aruvi_models", "read_model_call_snapshot", { id: snapshotCall.id, kind: "request" })).toBe(snapshot.content);
    const responseSnapshot = await (await request.get(`/api/mobile/model-calls/${snapshotCall.id}/snapshots/response`, { headers })).json();
    expect(typeof responseSnapshot.content).toBe("string");
    expect(await mcp("aruvi_models", "read_model_call_snapshot", { id: snapshotCall.id, kind: "response" })).toBe(responseSnapshot.content);
    await page.getByRole("button", { name: "Activity", exact: true }).click();
    const callButton = page.locator("#modelCallList button").filter({ hasText: snapshotCall.source_label || snapshotCall.source_kind }).first();
    await callButton.click();
    await page.getByRole("button", { name: "Show request snapshot" }).click();
    await expect(page.locator("#modelCallDetails")).toContainText("synthetic skill instruction");
    const prompts = await (await request.get("http://127.0.0.1:19876/workflow/prompts")).json();
    expect(prompts.length).toBeGreaterThanOrEqual(3);
    expect(JSON.stringify(prompts[2])).toContain("Follow the synthetic skill instruction.");
    const providerRequests = await (await request.get("http://127.0.0.1:19876/workflow/requests")).json();
    expect(providerRequests[0].response_format).toBeNull();
    expect(providerRequests[1].response_format).toBeNull();
    expect(providerRequests[2].response_format).toEqual({ type: "json_object" });
    expect(providerRequests[3].response_format).toEqual({ type: "json_object" });
    expect(providerRequests.slice(4).every((call) => call.response_format === null)).toBe(true);
    expect(JSON.stringify(prompts)).not.toContain("Do not include this synthetic instruction.");
    expect(JSON.stringify(prompts)).not.toContain("Workflow disabled skill");
    const runs = await mcp("aruvi_workflows", "list_agent_runs_for_workflow", { workflow_run_id: delivery.workflow_run.id });
    expect(runs.some((run) => run.stage === "coding" && run.agent_id === assignedAgent.id)).toBe(true);
    const { id, name, role, description, prompt_template_ref, allowed_tools, skill_tags, boundaries, enabled, employment_status } = assignedAgent;
    const denied = await request.post("/api/mobile/agents/delete", { headers, data: { id, expected: { name, role, description, prompt_template_ref, allowed_tools, skill_tags, boundaries, enabled, employment_status } } });
    expect(denied.status()).toBe(409);
    expect(await denied.text()).toContain("execution history");
    const { id: policyId, primary_roles, fallback_roles, coordinator_required } = codingPolicy;
    await api("/api/mobile/routing-policies/delete", { stage_name: "coding", expected: { id: policyId, primary_roles, fallback_roles, coordinator_required } });
  }
  const file = await mcp("aruvi_repositories", "read_repository_file", { repository_id: repository.id, relative_path: "src/generated.txt" });
  expect(file.content).toBe("generated by workflow test\n");
});

test("MCP creates a product and drives a workflow through delivery", async ({ page, request }) => {
  test.skip(process.env.ARUVI_ISOLATED_WORKFLOW !== "1", "Requires an isolated server and simulated model");
  test.setTimeout(180000);
  const auth = { Authorization: `Bearer ${process.env.ARUVI_MCP_API_TOKEN}` };
  async function mcp(name, action, arguments_) {
    const response = await request.post("/api/mcp", { headers: auth, data: { jsonrpc: "2.0", id: 1, method: "tools/call", params: { name, arguments: { action, arguments: arguments_ } } } });
    expect(response.ok(), await response.text()).toBeTruthy();
    const result = (await response.json()).result;
    expect(result.isError, JSON.stringify(result)).not.toBeTruthy();
    return result.structuredContent.result;
  }
  await request.post("http://127.0.0.1:19876/workflow/reset");
  const provider = await mcp("aruvi_models", "create_provider", { name: "MCP fixture provider", base_url: process.env.ARUVI_TEST_MODEL_URL.replace(/\/v1\/?$/, "/workflow/v1") });
  const model = await mcp("aruvi_models", "create_model_definition", { provider_id: provider.id, name: "mcp-fixture-model" });
  for (const agent of (await mcp("aruvi_agents", "list_agent_definitions", {})).filter((item) => item.enabled)) {
    await mcp("aruvi_agents", "set_primary_agent_model_binding", { agent_id: agent.id, model_id: model.id });
  }
  for (const [key, value] of [["workflow.auto_start_after_work_item_approval", "false"], ["workflow.auto_approve_plan", "false"], ["workflow.auto_approve_test_review", "false"], ["workflow.test_argv", '["/bin/true"]']]) {
    await mcp("aruvi_settings", "set_setting", { key, value });
  }
  const product = await mcp("aruvi_catalog", "create_product", { name: "MCP lifecycle fixture" });
  const updated = await mcp("aruvi_catalog", "update_product", { id: product.id, description: "Managed through MCP" });
  expect(updated.description).toBe("Managed through MCP");
  const area = await mcp("aruvi_catalog", "create_product_area", { product_id: product.id, name: "MCP area" });
  const capability = await mcp("aruvi_catalog", "create_capability", { product_area_id: area.id, name: "MCP capability" });
  const repo = await mcp("aruvi_repositories", "register_repository", { name: "MCP fixture", local_path: "/repositories/mcp-fixture", default_branch: "main" });
  await mcp("aruvi_repositories", "attach_repository", { scope_type: "product", scope_id: product.id, repository_id: repo.id, is_default: true });
  const item = await mcp("aruvi_work_items", "create_work_item", { product_id: product.id, product_area_id: area.id, capability_id: capability.id, title: "Generate a test file", description: "Create the fixture file.", work_item_type: "task" });
  await mcp("aruvi_checkpoints", "approve_work_item", { work_item_id: item.id });
  const started = await mcp("aruvi_workflows", "start_work_item_workflow", { work_item_id: item.id });
  const runId = started.workflow_run_id;
  let run = await mcp("aruvi_workflows", "get_workflow_run", { workflow_run_id: runId });
  expect(run.current_stage).toBe("pending_plan_approval");
  await mcp("aruvi_workflows", "handle_workflow_user_action", { workflow_run_id: runId, action: "approve" });
  run = await mcp("aruvi_workflows", "get_workflow_run", { workflow_run_id: runId });
  expect(run.current_stage).toBe("pending_test_review");
  const file = await mcp("aruvi_repositories", "read_repository_file", { repository_id: repo.id, relative_path: "src/generated.txt" });
  expect(file.content).toBe("generated by workflow test\n");
  await mcp("aruvi_workflows", "handle_workflow_user_action", { workflow_run_id: runId, action: "approve" });
  run = await mcp("aruvi_workflows", "get_workflow_run", { workflow_run_id: runId });
  expect(run.current_stage).toBe("done");
  await page.goto("/remote");
  await page.locator("#tokenInput").fill(process.env.ARUVI_MOBILE_API_TOKEN);
  await page.locator("#saveButton").click();
  await page.getByRole("button", { name: "Products", exact: true }).click();
  await expect(page.getByRole("button", { name: /MCP lifecycle fixture/ })).toBeVisible();
});

test("remote speech generation works through UI, API and MCP", async ({ page, request }) => {
  test.skip(process.env.ARUVI_ISOLATED_WORKFLOW !== "1", "Requires an isolated server and simulated provider");
  const headers = { Authorization: `Bearer ${process.env.ARUVI_MOBILE_API_TOKEN}` };
  const created = await request.post("/api/mobile/providers", { headers, data: { name: "Speech fixture", base_url: process.env.ARUVI_TEST_MODEL_URL } });
  expect(created.ok()).toBeTruthy();
  const provider = await created.json();
  const payload = { provider_id: provider.id, model: "synthetic-tts", text: "Hello" };
  expect((await request.post("/api/mobile/speech/generate", { data: payload })).status()).toBe(401);
  const generated = await request.post("/api/mobile/speech/generate", { headers, data: payload });
  expect(generated.status()).toBe(200);
  expect(generated.headers()["content-type"]).toContain("audio/wav");
  expect((await generated.body()).subarray(0, 4).toString()).toBe("RIFF");
  expect((await request.post("/api/mobile/speech/generate", { headers, data: { ...payload, model: "missing" } })).status()).toBe(400);
  const mcpResponse = await request.post("/api/mcp", { headers: { Authorization: `Bearer ${process.env.ARUVI_MCP_API_TOKEN}` }, data: { jsonrpc: "2.0", id: 1, method: "tools/call", params: { name: "aruvi_speech", arguments: { action: "generate_speech", arguments: payload } } } });
  const mcp = (await mcpResponse.json()).result;
  expect(mcp.isError, JSON.stringify(mcp)).not.toBeTruthy();
  expect(Buffer.from(mcp.structuredContent.result.audio_bytes_base64, "base64").subarray(0, 4).toString()).toBe("RIFF");
  await page.goto("/remote");
  await page.locator("#tokenInput").fill(process.env.ARUVI_MOBILE_API_TOKEN);
  await page.locator("#saveButton").click();
  await page.getByRole("button", { name: "Voice", exact: true }).click();
  await expect(page.locator("#speechProvider option").filter({ hasText: "Speech fixture" })).toHaveCount(1);
  await page.locator("#speechProvider").selectOption(provider.id);
  await page.locator("#speechModel").fill("synthetic-tts");
  await page.locator("#speechText").fill("Hello from the browser");
  await page.getByRole("button", { name: "Generate speech" }).click();
  await expect(page.locator("#voiceMeta")).toHaveText("Speech ready to play.");
  await expect(page.locator("#generatedSpeech")).toBeVisible();
});

test("product test policies inherit, override, and disable the default", async ({ request }) => {
  test.skip(process.env.ARUVI_ISOLATED_WORKFLOW !== "1", "Requires a disposable isolated server");
  const headers = { Authorization: `Bearer ${process.env.ARUVI_MOBILE_API_TOKEN}` };
  async function post(path, data) {
    const response = await request.post(path, { headers, data });
    expect(response.ok(), await response.text()).toBeTruthy();
    return response.json();
  }
  const product = await post("/api/mobile/products", { name: "Test policy isolation" });
  const url = `/api/mobile/workflow-test-config?product_id=${encodeURIComponent(product.id)}`;
  await post("/api/mobile/workflow-test-config", { argv: ["/bin/true"] });
  expect(await (await request.get(url, { headers })).json()).toEqual({ mode: "inherit", argv: ["/bin/true"] });
  expect(await post("/api/mobile/workflow-test-config", { product_id: product.id, mode: "custom", argv: ["/bin/false"] })).toEqual({ mode: "custom", argv: ["/bin/false"] });
  expect(await post("/api/mobile/workflow-test-config", { product_id: product.id, mode: "disabled" })).toEqual({ mode: "disabled", argv: [] });
  expect(await post("/api/mobile/workflow-test-config", { product_id: product.id, mode: "inherit" })).toEqual({ mode: "inherit", argv: ["/bin/true"] });
  expect((await request.post("/api/mobile/workflow-test-config", { headers, data: { product_id: product.id, mode: "custom", argv: [] } })).status()).toBe(400);
  expect((await request.post("/api/mobile/workflow-test-config", { headers, data: { product_id: "missing", mode: "disabled" } })).status()).toBe(400);
});

test("isolated provider key rotation uses the new credential", async ({ page, request }) => {
  test.skip(process.env.ARUVI_ISOLATED_WORKFLOW !== "1", "Requires a disposable server and simulated provider");
  const token = process.env.ARUVI_MOBILE_API_TOKEN;
  const headers = { Authorization: `Bearer ${token}` };
  const key = "synthetic-rotation-key-12345";
  const legacy = await request.post("/api/mobile/providers", { headers, data: { name: "Managed key provider", base_url: process.env.ARUVI_TEST_MODEL_URL.replace(/\/v1\/?$/, "/legacy/v1"), api_key: "synthetic-legacy-key-12345" } });
  expect(legacy.ok()).toBeTruthy();
  const legacyProvider = await legacy.json();
  const legacyTest = await request.post(`/api/mobile/providers/${legacyProvider.id}/test`, { headers });
  expect((await legacyTest.json()).message).toBe("Connection successful");
  const external = await request.post("/api/mobile/providers", { headers, data: { name: "Explicit reference provider", base_url: process.env.ARUVI_TEST_MODEL_URL.replace(/\/v1\/?$/, "/legacy/v1"), api_key: "ref:synthetic-external-reference-with-hyphens" } });
  expect(external.ok()).toBeTruthy();
  const externalProvider = await external.json();
  const externalTest = await request.post(`/api/mobile/providers/${externalProvider.id}/test`, { headers });
  expect((await externalTest.json()).message).toBe("Connection successful");
  const create = await request.post("/api/mobile/providers", { headers, data: { name: "Rotation provider", base_url: process.env.ARUVI_TEST_MODEL_URL.replace(/\/v1\/?$/, "/credential/v1") } });
  expect(create.ok()).toBeTruthy();
  const provider = await create.json();
  const expected = { name: provider.name, base_url: provider.base_url, enabled: provider.enabled };
  const payload = { id: provider.id, expected, api_key: key };
  expect((await request.post("/api/mobile/providers/credential", { data: payload })).status()).toBe(401);
  expect((await request.post("/api/mobile/providers/credential", { headers, data: { ...payload, api_key: "" } })).status()).toBe(400);
  expect((await request.post("/api/mobile/providers/credential", { headers, data: { ...payload, expected: { ...expected, name: "stale" } } })).status()).toBe(409);
  await page.goto("/remote");
  await page.locator("#tokenInput").fill(token);
  await page.locator("#saveButton").click();
  await page.getByRole("button", { name: "Models", exact: true }).click();
  const card = page.locator("#providerList .tool-card").filter({ hasText: "Rotation provider" });
  await card.getByText("Manage provider", { exact: true }).click();
  await card.getByLabel("New API key for Rotation provider").fill(key);
  const rotated = page.waitForResponse((response) => response.url().endsWith("/api/mobile/providers/credential"));
  await card.getByRole("button", { name: "Replace API key" }).click();
  const response = await rotated;
  expect(response.ok()).toBeTruthy();
  expect(await response.text()).not.toContain(key);
  await expect(page.locator("#status")).toHaveText("Provider key replaced");
  const reloadedCard = page.locator("#providerList .tool-card").filter({ hasText: "Rotation provider" });
  await reloadedCard.getByRole("button", { name: "Test connection" }).click();
  await expect(page.locator("#status")).toHaveText("Connection successful");
  await reloadedCard.getByText("Manage provider", { exact: true }).click();
  await expect(reloadedCard.getByLabel("New API key for Rotation provider")).toHaveValue("");
  async function mcp(action, arguments_) {
    const response = await request.post("/api/mcp", { headers: { Authorization: `Bearer ${process.env.ARUVI_MCP_API_TOKEN}`, Accept: "application/json, text/event-stream" },
      data: { jsonrpc: "2.0", id: 1, method: "tools/call", params: { name: "aruvi_models", arguments: { action, arguments: arguments_ } } } });
    const result = (await response.json()).result;
    expect(result.isError).not.toBeTruthy();
    return result.structuredContent.result;
  }
  const providers = await mcp("list_providers", {});
  const firstRef = providers.find((item) => item.id === provider.id).auth_secret_ref;
  const shared = await mcp("create_provider", { name: "Shared rotation reference", base_url: provider.base_url, auth_secret_ref: `ref:${firstRef}` });
  expect((await request.post("/api/mobile/providers/credential", { headers, data: payload })).ok()).toBeTruthy();
  const cleanupPayload = { id: provider.id, expected };
  expect((await request.post("/api/mobile/providers/credential/cleanup", { data: cleanupPayload })).status()).toBe(401);
  expect((await request.post("/api/mobile/providers/credential/cleanup", { headers, data: { ...cleanupPayload, expected: { ...expected, name: "stale" } } })).status()).toBe(409);
  async function clean(count) {
    page.once("dialog", (dialog) => dialog.accept());
    await reloadedCard.getByRole("button", { name: "Clean unused fallback keys" }).click();
    await expect(page.locator("#status")).toHaveText(`Removed ${count} unused fallback key(s)`);
  }
  await clean(0);
  expect((await (await request.post(`/api/mobile/providers/${shared.id}/test`, { headers })).json()).message).toBe("Connection successful");
  await mcp("delete_provider", { id: shared.id });
  await clean(1);
  expect((await (await request.post(`/api/mobile/providers/${provider.id}/test`, { headers })).json()).message).toBe("Connection successful");
  expect((await (await request.post(`/api/mobile/providers/${externalProvider.id}/test`, { headers })).json()).message).toBe("Connection successful");
  await clean(0);
  const listing = await (await request.get("/api/mobile/providers", { headers })).text();
  expect(listing).not.toContain(key);
  expect(listing).not.toContain("auth_secret_ref");
  const before = JSON.parse(listing).find((item) => item.id === provider.id);
  expect(before.credentials_disabled).toBeFalsy();
  const activeRef = (await mcp("list_providers", {})).find((item) => item.id === provider.id).auth_secret_ref;
  const sharedAgain = await mcp("create_provider", { name: "Shared active credential", base_url: provider.base_url, auth_secret_ref: `ref:${activeRef}` });
  expect((await request.post("/api/mobile/providers/credential", { headers, data: payload })).ok()).toBeTruthy();
  const staleClear = { id: provider.id, expected, credential_revision: before.credential_revision };
  expect((await request.post("/api/mobile/providers/credential/clear", { data: staleClear })).status()).toBe(401);
  expect((await request.post("/api/mobile/providers/credential/clear", { headers, data: staleClear })).status()).toBe(409);
  await page.reload();
  await page.getByRole("button", { name: "Models", exact: true }).click();
  const currentCard = page.locator("#providerList .tool-card").filter({ hasText: "Rotation provider" });
  await currentCard.getByText("Manage provider", { exact: true }).click();
  page.once("dialog", (dialog) => dialog.accept());
  await currentCard.getByRole("button", { name: "Detach API key" }).click();
  await expect(page.locator("#status")).toHaveText("Provider key detached");
  const after = (await (await request.get("/api/mobile/providers", { headers })).json()).find((item) => item.id === provider.id);
  expect(after.credentials_disabled).toBeTruthy();
  expect(after.credential_revision).not.toBe(before.credential_revision);
  expect((await (await request.post(`/api/mobile/providers/${provider.id}/test`, { headers })).json()).message).not.toBe("Connection successful");
  expect((await (await request.post(`/api/mobile/providers/${sharedAgain.id}/test`, { headers })).json()).message).toBe("Connection successful");
  const oldState = await mcp("get_provider_credential_state", { id: sharedAgain.id });
  expect(oldState.credentials_disabled).toBeFalsy();
  const sharedView = (await (await request.get("/api/mobile/providers", { headers })).json()).find((item) => item.id === sharedAgain.id);
  const sharedExpected = { name: sharedView.name, base_url: sharedView.base_url, enabled: sharedView.enabled, protocol: sharedView.protocol };
  expect((await request.post("/api/mobile/providers/credential", { headers, data: { id: sharedAgain.id, expected: sharedExpected, api_key: key } })).ok()).toBeTruthy();
  const staleMcp = await request.post("/api/mcp", { headers: { Authorization: `Bearer ${process.env.ARUVI_MCP_API_TOKEN}`, Accept: "application/json, text/event-stream" },
    data: { jsonrpc: "2.0", id: 2, method: "tools/call", params: { name: "aruvi_models", arguments: { action: "detach_provider_credential", arguments: { id: sharedAgain.id, credential_revision: oldState.credential_revision } } } } });
  expect((await staleMcp.json()).result.isError).toBeTruthy();
  expect((await (await request.post(`/api/mobile/providers/${sharedAgain.id}/test`, { headers })).json()).message).toBe("Connection successful");
  const currentState = await mcp("get_provider_credential_state", { id: sharedAgain.id });
  await mcp("detach_provider_credential", { id: sharedAgain.id, credential_revision: currentState.credential_revision });
  expect((await mcp("get_provider_credential_state", { id: sharedAgain.id })).credentials_disabled).toBeTruthy();
  expect((await (await request.post(`/api/mobile/providers/${sharedAgain.id}/test`, { headers })).json()).message).not.toBe("Connection successful");
});


test("disabled providers cannot transcribe through web or MCP", async ({ request }) => {
  test.skip(process.env.ARUVI_ISOLATED_WORKFLOW !== "1", "Requires an isolated server");
  const headers = { Authorization: `Bearer ${process.env.ARUVI_MOBILE_API_TOKEN}` };
  const created = await request.post("/api/mobile/providers", { headers, data: { name: "Disabled speech fixture", base_url: process.env.ARUVI_TEST_MODEL_URL } });
  expect(created.ok()).toBeTruthy();
  const provider = await created.json();
  const expected = { name: provider.name, base_url: provider.base_url, enabled: true };
  const updated = await request.post("/api/mobile/providers/update", { headers, data: { id: provider.id, expected, fields: { ...expected, enabled: false } } });
  expect(updated.ok()).toBeTruthy();
  const payload = { provider_id: provider.id, model_name: "synthetic-stt-model", audio_bytes_base64: "AA==", mime_type: "audio/wav" };
  const web = await request.post("/api/mobile/speech/transcribe", { headers, data: payload });
  expect(web.status()).toBe(400);
  expect(await web.text()).toContain("Selected provider is disabled");
  const response = await request.post("/api/mcp", { headers: { Authorization: `Bearer ${process.env.ARUVI_MCP_API_TOKEN}` }, data: { jsonrpc: "2.0", id: 1, method: "tools/call", params: { name: "aruvi_speech", arguments: { action: "transcribe_audio", arguments: payload } } } });
  const mcp = (await response.json()).result;
  expect(mcp.isError).toBeTruthy();
  expect(JSON.stringify(mcp.content)).toContain("Selected provider is disabled");
});

test("browser chat streams, reports errors, and can stop", async ({ page, request }) => {
  test.skip(process.env.ARUVI_ISOLATED_WORKFLOW !== "1", "Requires a disposable server and simulated provider");
  const headers = { Authorization: `Bearer ${process.env.ARUVI_MOBILE_API_TOKEN}` };
  const created = await request.post("/api/mobile/providers", { headers, data: { name: "Streaming fixture", base_url: process.env.ARUVI_TEST_MODEL_URL } });
  const provider = await created.json();
  const body = { provider_id: provider.id, model_name: "browser-test-model", messages: [{ role: "user", content: "hello" }] };
  expect((await request.post("/api/mobile/chat/stream", { data: body })).status()).toBe(401);
  await page.goto("/remote");
  await page.locator("#tokenInput").fill(process.env.ARUVI_MOBILE_API_TOKEN);
  await page.locator("#providerInput").fill(provider.id);
  await page.locator("#modelInput").fill("browser-test-model");
  await page.locator("#saveButton").click();
  await page.getByRole("button", { name: "Chat", exact: true }).click();
  await page.getByLabel("Stream response").check();
  async function send(text) { await page.locator("#chatComposer").fill(text); await page.locator("#chatSendButton").click(); }
  await send("stream success");
  await expect(page.locator("#chatConversation .assistant").last()).toHaveText("First ");
  await expect(page.locator("#chatConversation .assistant").last()).toHaveText("First second");
  await expect(page.locator("#status")).toHaveText("Connected");
  await send("stream failure");
  await expect(page.locator("#chatConversation")).toContainText("Response incomplete");
  await expect(page.locator("#status")).toContainText("truncated");
  await send("stream malformed");
  await expect(page.locator("#chatConversation .assistant").last()).toContainText("Invalid model content");
  await expect(page.locator("#status")).toContainText("Invalid model content");
  const statsUrl = process.env.ARUVI_TEST_MODEL_URL.replace(/\/v1\/?$/, "/stream/stats");
  const beforeStop = (await (await request.get(statsUrl)).json()).disconnects;
  await send("stream cancel");
  await expect(page.locator("#chatConversation .assistant").last()).toHaveText("First ");
  await page.getByRole("button", { name: "Stop", exact: true }).click();
  await expect(page.locator("#status")).toHaveText("Stream stopped");
  await expect.poll(async () => (await (await request.get(statsUrl)).json()).disconnects).toBeGreaterThan(beforeStop);
  await expect.poll(async () => {
    const calls = await (await request.get("/api/mobile/model-calls?limit=100", { headers })).json();
    return calls.filter((call) => call.provider_id === provider.id && call.source_kind === "mobile_chat").map((call) => call.status).sort();
  }).toEqual(["completed", "failed", "failed", "failed"]);
});

test("browser streams Anthropic text and preserves errors and usage", async ({ page, request }) => {
  test.skip(process.env.ARUVI_ISOLATED_WORKFLOW !== "1", "Requires a disposable server and simulated provider");
  const headers = { Authorization: `Bearer ${process.env.ARUVI_MOBILE_API_TOKEN}` };
  const created = await request.post("/api/mobile/providers", { headers, data: { name: "Anthropic streaming fixture", protocol: "anthropic", base_url: process.env.ARUVI_TEST_MODEL_URL.replace(/\/v1\/?$/, "/anthropic/v1"), api_key: "synthetic-anthropic-key" } });
  expect(created.ok()).toBeTruthy();
  const provider = await created.json();
  await page.goto("/remote");
  await page.locator("#tokenInput").fill(process.env.ARUVI_MOBILE_API_TOKEN);
  await page.locator("#providerInput").fill(provider.id);
  await page.locator("#modelInput").fill("claude-fixture");
  await page.locator("#saveButton").click();
  await page.getByRole("button", { name: "Chat", exact: true }).click();
  await page.getByLabel("Stream response").check();
  await page.locator("#chatComposer").fill("stream success");
  await page.locator("#chatSendButton").click();
  await expect(page.locator("#chatConversation .assistant").last()).toHaveText("Claude ");
  await expect(page.locator("#chatConversation .assistant").last()).toHaveText("Claude fixture");
  await expect(page.locator("#status")).toHaveText("Connected");
  await page.locator("#chatComposer").fill("stream failure");
  await page.locator("#chatSendButton").click();
  await expect(page.locator("#chatConversation")).toContainText("Response incomplete");
  await expect(page.locator("#status")).toContainText("token limit");
  const calls = (await (await request.get("/api/mobile/model-calls?limit=100", { headers })).json()).filter((call) => call.provider_id === provider.id);
  expect(calls.map((call) => call.status).sort()).toEqual(["completed", "failed"]);
  expect(calls.find((call) => call.status === "completed")).toMatchObject({ token_count_input: 17, token_count_output: 3 });
});

test("web and MCP reject partial mixed provider content", async ({ request }) => {
  test.skip(process.env.ARUVI_ISOLATED_WORKFLOW !== "1", "Requires a disposable server and simulated provider");
  const headers = { Authorization: `Bearer ${process.env.ARUVI_MOBILE_API_TOKEN}` };
  for (const [protocol, prefix] of [["openai_compatible", "malformed"], ["anthropic", "anthropic-malformed"]]) {
    const created = await request.post("/api/mobile/providers", { headers, data: { name: `Malformed ${protocol}`, protocol, base_url: process.env.ARUVI_TEST_MODEL_URL.replace(/\/v1\/?$/, `/${prefix}/v1`), api_key: "synthetic-key" } });
    expect(created.ok()).toBeTruthy();
    const provider = await created.json();
    const messages = [{ role: "user", content: "hello" }];
    const web = await request.post("/api/mobile/chat/completions", { headers, data: { provider_id: provider.id, model_name: "fixture", messages } });
    expect(web.status()).toBe(400);
    expect(await web.text()).toContain("unsupported or invalid");
    const mcp = await request.post("/api/mcp", { headers: { Authorization: `Bearer ${process.env.ARUVI_MCP_API_TOKEN}` }, data: { jsonrpc: "2.0", id: 1, method: "tools/call", params: { name: "aruvi_models", arguments: { action: "run_model_chat_completion", arguments: { provider_id: provider.id, model: "fixture", messages } } } } });
    const result = (await mcp.json()).result;
    expect(result.isError).toBeTruthy();
    expect(JSON.stringify(result.content)).toContain("unsupported or invalid");
  }
});

test("voice records, reviews, and sends a transcribed message", async ({ page, request }) => {
  test.skip(process.env.ARUVI_ISOLATED_WORKFLOW !== "1", "Requires a disposable server and simulated provider");
  const headers = { Authorization: `Bearer ${process.env.ARUVI_MOBILE_API_TOKEN}` };
  const created = await request.post("/api/mobile/providers", { headers, data: { name: "Voice fixture", base_url: process.env.ARUVI_TEST_MODEL_URL } });
  expect(created.ok()).toBeTruthy();
  const provider = await created.json();
  expect((await request.post("/api/mobile/speech/transcribe", { data: { audio_bytes_base64: "", mime_type: "audio/wav" } })).status()).toBe(401);
  const emptyAudio = { provider_id: provider.id, model_name: "synthetic-stt-model", audio_bytes_base64: "", mime_type: "audio/wav" };
  const invalid = await request.post("/api/mobile/speech/transcribe", { headers, data: emptyAudio });
  expect(invalid.status()).toBe(400);
  expect(await invalid.text()).toContain("nonempty audio");
  const invalidMcp = await request.post("/api/mcp", { headers: { Authorization: `Bearer ${process.env.ARUVI_MCP_API_TOKEN}` }, data: { jsonrpc: "2.0", id: 1, method: "tools/call", params: { name: "aruvi_speech", arguments: { action: "transcribe_audio", arguments: emptyAudio } } } });
  const invalidMcpResult = (await invalidMcp.json()).result;
  expect(invalidMcpResult.isError).toBeTruthy();
  expect(JSON.stringify(invalidMcpResult.content)).toContain("nonempty audio");
  await page.addInitScript(() => {
    window.fixtureTracksStopped = 0;
    Object.defineProperty(navigator, "mediaDevices", { value: { getUserMedia: async () => ({ getTracks: () => [{ stop() { window.fixtureTracksStopped++; } }] }) } });
    window.AudioContext = class {
      constructor() { this.sampleRate = 16000; this.destination = {}; }
      createMediaStreamSource() { return { connect() {}, disconnect() {} }; }
      createScriptProcessor() {
        return { connect() { setTimeout(() => this.onaudioprocess({ inputBuffer: { getChannelData: () => new Float32Array([0, 0.5, -0.5]) }, outputBuffer: { getChannelData: () => new Float32Array(3) } }), 10); }, disconnect() {} };
      }
      async close() {}
    };
  });
  await page.goto("/remote");
  await page.locator("#tokenInput").fill(process.env.ARUVI_MOBILE_API_TOKEN);
  await page.locator("#providerInput").fill(provider.id);
  await page.locator("#modelInput").fill("browser-test-model");
  await page.locator("#saveButton").click();
  await page.getByRole("button", { name: "Voice", exact: true }).click();
  await expect(page.getByLabel("Transcription provider").locator(`option[value="${provider.id}"]`)).toHaveCount(1);
  await page.getByLabel("Transcription provider").selectOption(provider.id);
  await page.getByLabel("Transcription model").fill("synthetic-stt-model");
  await page.getByLabel("Speech locale").fill("en-US");
  await page.reload();
  await expect(page.getByLabel("Transcription provider")).toHaveValue(provider.id);
  await expect(page.getByLabel("Transcription model")).toHaveValue("synthetic-stt-model");
  await page.getByRole("button", { name: "Record", exact: true }).click();
  await expect(page.getByRole("button", { name: "Stop", exact: true })).toBeVisible();
  await page.waitForTimeout(50);
  await page.getByRole("button", { name: "Stop", exact: true }).click();
  await expect(page.locator("#voiceComposer")).toHaveValue("Synthetic microphone transcript");
  expect(await page.evaluate(() => window.fixtureTracksStopped)).toBe(1);
  await expect(page.locator("#voiceConversation")).not.toContainText("Synthetic microphone transcript");
  await page.getByRole("button", { name: "Speak On" }).click();
  await page.getByRole("button", { name: "Submit", exact: true }).click();
  await expect(page.locator("#voiceConversation")).toContainText("Synthetic microphone transcript");
  await expect(page.locator("#voiceConversation")).toContainText("Container provider reply");
  await page.evaluate(() => { window.AudioContext = class { constructor() { throw new Error("Audio setup failed"); } }; });
  await page.getByRole("button", { name: "Record", exact: true }).click();
  await expect(page.locator("#voiceMeta")).toHaveText("Audio setup failed");
  expect(await page.evaluate(() => window.fixtureTracksStopped)).toBe(2);
  await expect(page.getByRole("button", { name: "Record", exact: true })).toBeEnabled();
  await page.evaluate(() => {
    window.AudioContext = class {
      constructor() { this.sampleRate = 16000; this.destination = {}; }
      createMediaStreamSource() { return { connect() {}, disconnect() {} }; }
      createScriptProcessor() { return { connect() { setTimeout(() => this.onaudioprocess({ inputBuffer: { getChannelData: () => new Float32Array(650000) }, outputBuffer: { getChannelData: () => new Float32Array(1) } }), 10); }, disconnect() {} }; }
      async close() {}
    };
  });
  await page.getByLabel("Review transcript before sending").uncheck();
  const transcription = page.waitForResponse((response) => response.url().endsWith("/api/mobile/speech/transcribe"));
  await page.getByRole("button", { name: "Record", exact: true }).click();
  const cappedResponse = await transcription;
  expect(cappedResponse.ok()).toBeTruthy();
  const cappedRequest = cappedResponse.request().postDataJSON();
  expect(cappedRequest.audio_bytes_base64.length).toBeLessThan(1280100);
  await expect(page.getByRole("button", { name: "Record", exact: true })).toBeEnabled();
  expect(await page.evaluate(() => window.fixtureTracksStopped)).toBe(3);
  await expect(page.locator("#voiceConversation").getByText("Container provider reply", { exact: true })).toHaveCount(2);
});

test("browser manages teams and preserves concurrent edits", async ({ page, request }) => {
  test.skip(process.env.ARUVI_ISOLATED_WORKFLOW !== "1", "Requires a disposable server");
  const headers = { Authorization: `Bearer ${process.env.ARUVI_MOBILE_API_TOKEN}` };
  async function mcp(action, args) {
    const response = await request.post("/api/mcp", { headers: { Authorization: `Bearer ${process.env.ARUVI_MCP_API_TOKEN}`, Accept: "application/json, text/event-stream" },
      data: { jsonrpc: "2.0", id: 1, method: "tools/call", params: { name: "aruvi_agents", arguments: { action, arguments: args } } } });
    const result = (await response.json()).result;
    expect(result.isError).not.toBeTruthy();
    return result.structuredContent.result;
  }
  const agent = await mcp("create_agent_definition", { name: "Team fixture agent", role: "test" });
  const fields = { name: "Browser team", department: "Engineering", description: "Team fixture", enabled: true, max_concurrent_workflows: 103 };
  expect((await request.get("/api/mobile/teams")).status()).toBe(401);
  expect((await request.post("/api/mobile/teams", { data: fields })).status()).toBe(401);
  expect((await request.post("/api/mobile/teams", { headers, data: { ...fields, max_concurrent_workflows: 0 } })).status()).toBe(400);
  await page.goto("/remote");
  await page.locator("#tokenInput").fill(process.env.ARUVI_MOBILE_API_TOKEN);
  await page.locator("#saveButton").click();
  await page.getByRole("button", { name: "Teams", exact: true }).click();
  await page.getByLabel("Team name", { exact: true }).fill(fields.name);
  await page.getByLabel("Team department", { exact: true }).fill(fields.department);
  await page.getByLabel("Team description", { exact: true }).fill(fields.description);
  await page.getByLabel("Team capacity", { exact: true }).fill("103");
  const created = page.waitForResponse((response) => response.url().endsWith("/api/mobile/teams") && response.request().method() === "POST");
  await page.getByRole("button", { name: "Create team", exact: true }).click();
  const team = await (await created).json();
  await expect(page.locator("#status")).toHaveText("Team created");
  const memberData = { team_id: team.id, agent_id: agent.id, title: "Reviewer", is_lead: true };
  expect((await request.post("/api/mobile/team-members", { data: memberData })).status()).toBe(401);
  expect((await request.post("/api/mobile/team-members", { headers, data: { ...memberData, agent_id: "missing" } })).status()).toBe(400);
  let card = page.locator("#teamList .tool-card").filter({ hasText: "Browser team" }).first();
  await card.getByText("Manage team", { exact: true }).click();
  await card.getByLabel("Add agent to Browser team").selectOption(agent.id);
  await card.getByLabel("Member title for Browser team").fill("Reviewer");
  await card.getByLabel("Team lead").check();
  await card.getByRole("button", { name: "Add member" }).click();
  await expect(page.locator("#status")).toHaveText("Member added");
  card = page.locator("#teamList .tool-card").filter({ hasText: "Browser team" }).first();
  await card.getByText("Manage team", { exact: true }).click();
  await expect(card).toContainText("Team fixture agent · Reviewer · lead");
  await card.getByLabel("Edit team name Browser team").fill("Unsaved team name");
  await mcp("update_agent_team", { id: team.id, description: "Concurrent team edit" });
  const conflict = page.waitForResponse((response) => response.url().endsWith("/api/mobile/teams/update"));
  await card.getByRole("button", { name: "Save team" }).click();
  expect((await conflict).status()).toBe(409);
  await expect(card.getByLabel("Edit team name Browser team")).toHaveValue("Unsaved team name");
  expect((await request.post("/api/mobile/teams/delete", { headers, data: { id: team.id, expected: fields } })).status()).toBe(409);
  await page.getByRole("button", { name: "Refresh", exact: true }).click();
  card = page.locator("#teamList .tool-card").filter({ hasText: "Browser team" }).first();
  await card.getByText("Manage team", { exact: true }).click();
  await card.getByLabel("Edit team name Browser team").fill("Renamed team");
  await card.getByRole("button", { name: "Save team" }).click();
  await expect(page.locator("#status")).toHaveText("Team saved");
  card = page.locator("#teamList .tool-card").filter({ hasText: "Renamed team" }).first();
  await card.getByText("Manage team", { exact: true }).click();
  page.once("dialog", (dialog) => dialog.accept());
  await card.getByRole("button", { name: "Remove Team fixture agent from Renamed team" }).click();
  await expect(page.locator("#status")).toHaveText("Member removed");
  card = page.locator("#teamList .tool-card").filter({ hasText: "Renamed team" }).first();
  await card.getByText("Manage team", { exact: true }).click();
  await card.getByLabel("Add agent to Renamed team").selectOption(agent.id);
  await card.getByRole("button", { name: "Add member" }).click();
  await expect(page.locator("#status")).toHaveText("Member added");
  card = page.locator("#teamList .tool-card").filter({ hasText: "Renamed team" }).first();
  await card.getByText("Manage team", { exact: true }).click();
  page.once("dialog", (dialog) => dialog.accept());
  await card.getByRole("button", { name: "Delete team" }).click();
  await expect(page.locator("#status")).toHaveText("Team deleted");
  expect((await mcp("list_agent_teams", {})).some((item) => item.id === team.id)).toBe(false);
  expect((await mcp("list_team_memberships", {})).some((item) => item.team_id === team.id)).toBe(false);
  await mcp("delete_agent_definition", { id: agent.id });
});

test("browser assigns and removes product hierarchy scopes", async ({ page, request }) => {
  test.skip(process.env.ARUVI_ISOLATED_WORKFLOW !== "1", "Requires a disposable server");
  const headers = { Authorization: `Bearer ${process.env.ARUVI_MOBILE_API_TOKEN}` };
  async function mcp(name, action, args) {
    const response = await request.post("/api/mcp", { headers: { Authorization: `Bearer ${process.env.ARUVI_MCP_API_TOKEN}`, Accept: "application/json, text/event-stream" },
      data: { jsonrpc: "2.0", id: 1, method: "tools/call", params: { name, arguments: { action, arguments: args } } } });
    const result = (await response.json()).result;
    expect(result.isError).not.toBeTruthy();
    return result.structuredContent.result;
  }
  async function post(path, data) {
    const response = await request.post(path, { headers, data });
    expect(response.ok(), await response.text()).toBeTruthy();
    return response.json();
  }
  const product = await post("/api/mobile/products", { name: "Scope fixture product" });
  const other = await post("/api/mobile/products", { name: "Other scope product" });
  const area = await mcp("aruvi_catalog", "create_product_area", { product_id: product.id, name: "Scope fixture area" });
  const capability = await mcp("aruvi_catalog", "create_capability", { product_area_id: area.id, name: "Scope fixture capability" });
  const feature = await mcp("aruvi_catalog", "create_capability", { product_area_id: area.id, parent_capability_id: capability.id, name: "Scope fixture feature" });
  const team = await post("/api/mobile/teams", { name: "Scope fixture team", department: "", description: "", enabled: true, max_concurrent_workflows: 2 });
  const assignment = { team_id: team.id, product_id: product.id, scope_type: "capability", scope_id: capability.id };
  const invalidMcp = await request.post("/api/mcp", { headers: { Authorization: `Bearer ${process.env.ARUVI_MCP_API_TOKEN}`, Accept: "application/json, text/event-stream" },
    data: { jsonrpc: "2.0", id: 1, method: "tools/call", params: { name: "aruvi_agents", arguments: { action: "assign_team_scope", arguments: { team_id: team.id, scope_type: "capability", scope_id: "missing" } } } } });
  expect((await invalidMcp.json()).result.isError).toBe(true);
  expect((await request.get("/api/mobile/team-assignments")).status()).toBe(401);
  expect((await request.post("/api/mobile/team-assignments", { data: assignment })).status()).toBe(401);
  expect((await request.post("/api/mobile/team-assignments", { headers, data: { ...assignment, product_id: other.id } })).status()).toBe(409);
  expect((await request.post("/api/mobile/team-assignments", { headers, data: { ...assignment, scope_id: "missing" } })).status()).toBe(409);
  expect((await request.post("/api/mobile/team-assignments", { headers, data: { ...assignment, scope_type: "invalid" } })).status()).toBe(400);
  await page.goto("/remote");
  await page.locator("#tokenInput").fill(process.env.ARUVI_MOBILE_API_TOKEN);
  await page.locator("#saveButton").click();
  await page.getByRole("button", { name: "Teams", exact: true }).click();
  const card = page.locator("#teamList .tool-card").filter({ hasText: "Scope fixture team" }).first();
  async function assign(label) {
    await card.getByText("Manage team", { exact: true }).click();
    await card.getByLabel("Assignment product for Scope fixture team").selectOption(product.id);
    const scope = card.getByLabel("Assignment scope for Scope fixture team");
    await expect(scope).toBeEnabled();
    await scope.selectOption({ label });
    await card.getByRole("button", { name: "Assign scope" }).click();
    await expect(page.locator("#status")).toHaveText("Scope assigned");
  }
  await assign("Product: Scope fixture product");
  await assign("Area: Scope fixture area");
  await assign("Area: Scope fixture area / Scope fixture capability");
  await assign("Area: Scope fixture area / Scope fixture capability / Scope fixture feature");
  await card.getByText("Manage team", { exact: true }).click();
  await expect(card).toContainText("Scope fixture product · product: Scope fixture product");
  await expect(card).toContainText("Scope fixture product · product area: Scope fixture area");
  await expect(card).toContainText("Scope fixture product · capability: Scope fixture feature");
  expect((await request.post("/api/mobile/team-assignments", { headers, data: assignment })).status()).toBe(400);
  const assignments = (await (await request.get("/api/mobile/team-assignments", { headers })).json()).filter((entry) => entry.team_id === team.id);
  expect(assignments).toHaveLength(4);
  const featureAssignment = assignments.find((entry) => entry.scope_id === feature.id);
  expect((await request.post("/api/mobile/team-assignments/remove", { headers, data: { ...featureAssignment, scope_id: "wrong" } })).status()).toBe(409);
  page.once("dialog", (dialog) => dialog.accept());
  await card.getByRole("button", { name: "Remove scope Scope fixture feature from Scope fixture team" }).click();
  await expect(page.locator("#status")).toHaveText("Scope removed");
  expect((await mcp("aruvi_agents", "list_team_assignments", {})).some((entry) => entry.id === featureAssignment.id)).toBe(false);
});

test("browser manages skills and links without overwriting concurrent edits", async ({ page, request }) => {
  test.skip(process.env.ARUVI_ISOLATED_WORKFLOW !== "1", "Requires a disposable server");
  const headers = { Authorization: `Bearer ${process.env.ARUVI_MOBILE_API_TOKEN}` };
  async function mcp(action, args) {
    const response = await request.post("/api/mcp", { headers: { Authorization: `Bearer ${process.env.ARUVI_MCP_API_TOKEN}`, Accept: "application/json, text/event-stream" },
      data: { jsonrpc: "2.0", id: 1, method: "tools/call", params: { name: "aruvi_agents", arguments: { action, arguments: args } } } });
    const result = (await response.json()).result;
    expect(result.isError).not.toBeTruthy();
    return result.structuredContent.result;
  }
  const agent = await mcp("create_agent_definition", { name: "Skill fixture agent", role: "test" });
  const team = await mcp("create_agent_team", { name: "Skill fixture team" });
  const fields = { name: "Browser skill", category: "testing", description: "Fixture description", instructions: "Fixture instructions", enabled: true };
  expect((await request.get("/api/mobile/skills")).status()).toBe(401);
  expect((await request.post("/api/mobile/skills", { data: fields })).status()).toBe(401);
  expect((await request.post("/api/mobile/skills", { headers, data: { ...fields, name: "  " } })).status()).toBe(400);
  await page.goto("/remote");
  await page.locator("#tokenInput").fill(process.env.ARUVI_MOBILE_API_TOKEN);
  await page.locator("#saveButton").click();
  await page.getByRole("button", { name: "Skills", exact: true }).click();
  await expect(page.getByText("Agent runs include enabled linked skill instructions", { exact: false })).toBeVisible();
  await page.getByLabel("Skill name", { exact: true }).fill(fields.name);
  await page.getByLabel("Skill category", { exact: true }).fill(fields.category);
  await page.getByLabel("Skill description", { exact: true }).fill(fields.description);
  await page.getByLabel("Skill instructions", { exact: true }).fill(fields.instructions);
  const created = page.waitForResponse((response) => response.url().endsWith("/api/mobile/skills") && response.request().method() === "POST");
  await page.getByRole("button", { name: "Create skill", exact: true }).click();
  const skill = await (await created).json();
  await expect(page.locator("#status")).toHaveText("Skill created");
  const linkData = { skill_id: skill.id, target_type: "agent", target_id: agent.id, proficiency: "expert" };
  expect((await request.post("/api/mobile/skill-links", { data: linkData })).status()).toBe(401);
  expect((await request.post("/api/mobile/skill-links", { headers, data: { ...linkData, target_id: "missing" } })).status()).toBe(400);
  expect((await request.post("/api/mobile/skill-links", { headers, data: { ...linkData, proficiency: "invalid" } })).status()).toBe(400);
  const card = page.locator("#skillList .tool-card").filter({ hasText: "Browser skill" }).first();
  await card.getByText("Manage skill", { exact: true }).click();
  await card.getByLabel("Link agent to Browser skill").selectOption(agent.id);
  await card.getByLabel("Proficiency for Browser skill").selectOption("expert");
  await card.getByRole("button", { name: "Link agent", exact: true }).click();
  await expect(page.locator("#status")).toHaveText("Skill linked");
  expect((await request.post("/api/mobile/skill-links", { headers, data: linkData })).status()).toBe(400);
  await card.getByText("Manage skill", { exact: true }).click();
  await card.getByLabel("Link team to Browser skill").selectOption(team.id);
  await card.getByRole("button", { name: "Link team", exact: true }).click();
  await expect(page.locator("#status")).toHaveText("Skill linked");
  await card.getByText("Manage skill", { exact: true }).click();
  await expect(card).toContainText("Agent: Skill fixture agent · expert");
  await expect(card).toContainText("Team: Skill fixture team");
  await card.getByLabel("Edit skill name Browser skill").fill("Unsaved skill");
  await mcp("update_skill", { id: skill.id, description: "Concurrent edit" });
  const conflict = page.waitForResponse((response) => response.url().endsWith("/api/mobile/skills/update"));
  await card.getByRole("button", { name: "Save skill" }).click();
  expect((await conflict).status()).toBe(409);
  await expect(card.getByLabel("Edit skill name Browser skill")).toHaveValue("Unsaved skill");
  expect((await request.post("/api/mobile/skills/delete", { headers, data: { id: skill.id, expected: fields } })).status()).toBe(409);
  await page.getByRole("button", { name: "Refresh", exact: true }).click();
  await card.getByText("Manage skill", { exact: true }).click();
  await card.getByLabel("Edit skill name Browser skill").fill("Renamed skill");
  await card.getByLabel("Skill enabled").uncheck();
  await card.getByRole("button", { name: "Save skill" }).click();
  await expect(page.locator("#status")).toHaveText("Skill saved");
  const renamed = page.locator("#skillList .tool-card").filter({ hasText: "Renamed skill" }).first();
  await expect(renamed).toContainText("disabled");
  await renamed.getByText("Manage skill", { exact: true }).click();
  page.once("dialog", (dialog) => dialog.accept());
  await renamed.getByRole("button", { name: "Remove Agent: Skill fixture agent · expert from Renamed skill" }).click();
  await expect(page.locator("#status")).toHaveText("Skill link removed");
  await renamed.getByText("Manage skill", { exact: true }).click();
  page.once("dialog", (dialog) => dialog.accept());
  await renamed.getByRole("button", { name: "Delete skill" }).click();
  await expect(page.locator("#status")).toHaveText("Skill deleted");
  expect((await mcp("list_team_skill_links", {})).some((item) => item.skill_id === skill.id)).toBe(false);
  expect((await mcp("list_agent_skill_links", {})).some((item) => item.skill_id === skill.id)).toBe(false);
});

test("browser manages agent definitions and detects conflicts", async ({ page, request }) => {
  test.skip(process.env.ARUVI_ISOLATED_WORKFLOW !== "1", "Requires a disposable server");
  const headers = { Authorization: `Bearer ${process.env.ARUVI_MOBILE_API_TOKEN}` };
  async function mcp(action, args) {
    const response = await request.post("/api/mcp", { headers: { Authorization: `Bearer ${process.env.ARUVI_MCP_API_TOKEN}`, Accept: "application/json, text/event-stream" },
      data: { jsonrpc: "2.0", id: 1, method: "tools/call", params: { name: "aruvi_agents", arguments: { action, arguments: args } } } });
    const result = (await response.json()).result;
    expect(result.isError).not.toBeTruthy();
    return result.structuredContent.result;
  }
  const fields = { name: "Browser fixture agent", role: "developer", description: "Fixture agent", prompt_template_ref: "fixture-template", allowed_tools: ["repo.read_file"], skill_tags: ["test"], boundaries: { max_tokens: 1000, allowed_paths: ["src"] }, enabled: true, employment_status: "active" };
  expect((await request.post("/api/mobile/agents", { data: fields })).status()).toBe(401);
  expect((await request.post("/api/mobile/agents", { headers, data: { ...fields, name: " " } })).status()).toBe(400);
  expect((await request.post("/api/mobile/agents", { headers, data: { ...fields, boundaries: { max_tokens: "invalid" } } })).status()).toBe(400);
  expect((await request.post("/api/mobile/agents", { headers, data: { ...fields, boundaries: { blocked_path: ["secret"] } } })).status()).toBe(400);
  expect((await request.post("/api/mobile/agents", { headers, data: { ...fields, employment_status: "unknown" } })).status()).toBe(400);
  await page.goto("/remote");
  await page.locator("#tokenInput").fill(process.env.ARUVI_MOBILE_API_TOKEN);
  await page.locator("#saveButton").click();
  await page.getByRole("button", { name: "Agents", exact: true }).click();
  await expect(page.getByText("An empty allowed-tools list uses the default repository tools", { exact: false })).toBeVisible();
  await expect(page.getByText("Prompt template references are stored but are not currently applied", { exact: false })).toBeVisible();
  const createForm = page.locator("#createAgentEditor");
  await createForm.getByLabel("Agent name", { exact: true }).fill(fields.name);
  await createForm.getByLabel("Agent role", { exact: true }).fill(fields.role);
  await createForm.getByLabel("Agent description", { exact: true }).fill(fields.description);
  await createForm.getByLabel("Prompt template reference", { exact: true }).fill(fields.prompt_template_ref);
  await createForm.getByLabel("Allowed tools (JSON array)", { exact: true }).fill(JSON.stringify(fields.allowed_tools));
  await createForm.getByLabel("Skill tags (JSON array)", { exact: true }).fill(JSON.stringify(fields.skill_tags));
  await createForm.getByLabel("Execution boundaries (JSON object)", { exact: true }).fill("not json");
  await createForm.getByRole("button", { name: "Create agent" }).click();
  await expect(page.locator("#status")).toContainText("JSON arrays");
  await expect(createForm.getByLabel("Agent name", { exact: true })).toHaveValue(fields.name);
  await createForm.getByLabel("Execution boundaries (JSON object)", { exact: true }).fill(JSON.stringify(fields.boundaries));
  const created = page.waitForResponse((response) => response.url().endsWith("/api/mobile/agents") && response.request().method() === "POST");
  await createForm.getByRole("button", { name: "Create agent" }).click();
  const agent = await (await created).json();
  expect(agent).toMatchObject(fields);
  await expect(page.locator("#status")).toHaveText("Agent created");
  const team = await mcp("create_agent_team", { name: "Agent fixture team" });
  const skill = await mcp("create_skill", { name: "Agent fixture skill" });
  await mcp("add_team_member", { team_id: team.id, agent_id: agent.id });
  await mcp("link_skill_to_agent", { agent_id: agent.id, skill_id: skill.id });
  let card = page.locator("#agentList .tool-card").filter({ hasText: "Browser fixture agent" }).first();
  await card.getByText("Manage agent", { exact: true }).click();
  await card.getByLabel("Agent name Browser fixture agent").fill("Unsaved agent");
  await mcp("update_agent_definition", { id: agent.id, description: "Concurrent agent edit" });
  const conflict = page.waitForResponse((response) => response.url().endsWith("/api/mobile/agents/update"));
  await card.getByRole("button", { name: "Save agent" }).click();
  expect((await conflict).status()).toBe(409);
  await expect(card.getByLabel("Agent name Browser fixture agent")).toHaveValue("Unsaved agent");
  expect((await request.post("/api/mobile/agents/delete", { headers, data: { id: agent.id, expected: fields } })).status()).toBe(409);
  await page.getByRole("button", { name: "Refresh", exact: true }).click();
  card = page.locator("#agentList .tool-card").filter({ hasText: "Browser fixture agent" }).first();
  await card.getByText("Manage agent", { exact: true }).click();
  await card.getByLabel("Agent name Browser fixture agent").fill("Renamed fixture agent");
  await card.getByLabel("Employment status Browser fixture agent").selectOption("inactive");
  await card.getByLabel("Agent enabled").uncheck();
  await card.getByRole("button", { name: "Save agent" }).click();
  await expect(page.locator("#status")).toHaveText("Agent saved");
  const saved = (await mcp("list_agent_definitions", {})).find((entry) => entry.id === agent.id);
  expect(saved).toMatchObject({ name: "Renamed fixture agent", role: fields.role, description: "Concurrent agent edit", allowed_tools: fields.allowed_tools, boundaries: fields.boundaries, enabled: false, employment_status: "inactive" });
  const renamed = page.locator("#agentList .tool-card").filter({ hasText: "Renamed fixture agent" }).first();
  await renamed.getByText("Manage agent", { exact: true }).click();
  page.once("dialog", (dialog) => dialog.accept());
  await renamed.getByRole("button", { name: "Delete agent" }).click();
  await expect(page.locator("#status")).toHaveText("Agent deleted");
  expect((await mcp("list_team_memberships", {})).some((entry) => entry.agent_id === agent.id)).toBe(false);
  expect((await mcp("list_agent_skill_links", {})).some((entry) => entry.agent_id === agent.id)).toBe(false);
});

test("invalid boundaries imported through MCP stop agent execution", async ({ request }) => {
  test.skip(process.env.ARUVI_ISOLATED_WORKFLOW !== "1", "Requires a disposable server");
  const headers = { Authorization: `Bearer ${process.env.ARUVI_MOBILE_API_TOKEN}` };
  async function post(path, data) {
    const response = await request.post(path, { headers, data });
    expect(response.ok(), await response.text()).toBeTruthy();
    return response.json();
  }
  async function mcp(action, args) {
    const response = await request.post("/api/mcp", { headers: { Authorization: `Bearer ${process.env.ARUVI_MCP_API_TOKEN}`, Accept: "application/json, text/event-stream" },
      data: { jsonrpc: "2.0", id: 1, method: "tools/call", params: { name: "aruvi_agents", arguments: { action, arguments: args } } } });
    const result = (await response.json()).result;
    expect(result.isError).not.toBeTruthy();
    return result.structuredContent.result;
  }
  const provider = await post("/api/mobile/providers", { name: "Boundary test provider", base_url: process.env.ARUVI_TEST_MODEL_URL.replace(/\/v1\/?$/, "/workflow/v1") });
  const model = await post("/api/mobile/models", { provider_id: provider.id, name: "browser-test-model" });
  const product = await post("/api/mobile/products", { name: "Invalid boundary product" });
  const team = await post("/api/mobile/teams", { name: "Invalid boundary team", department: "", description: "", enabled: true, max_concurrent_workflows: 2 });
  const agent = await post("/api/mobile/agents", { name: "Invalid boundary agent", role: "requirement_analysis", description: "", prompt_template_ref: "", allowed_tools: [], skill_tags: [], boundaries: {}, enabled: true, employment_status: "active" });
  await post("/api/mobile/team-members", { team_id: team.id, agent_id: agent.id, title: "", is_lead: false });
  await post("/api/mobile/team-assignments", { team_id: team.id, product_id: product.id, scope_type: "product", scope_id: product.id });
  await post("/api/mobile/agent-model-bindings", { agent_id: agent.id, model_id: model.id });
  await mcp("update_agent_definition", { id: agent.id, boundaries: { blocked_path: ["secret"] } });
  const work = await post("/api/mobile/work-items", { product_id: product.id, title: "Invalid boundary work" });
  const response = await request.post(`/api/mobile/work-items/${work.id}/approve`, { headers, data: {} });
  expect(response.ok()).toBe(false);
  expect(await response.text()).toContain("Invalid agent execution boundaries");
  await mcp("update_agent_definition", { id: agent.id, enabled: false });
});

test("browser edits and resets stage routing without overwriting concurrent changes", async ({ page, request }) => {
  test.skip(process.env.ARUVI_ISOLATED_WORKFLOW !== "1", "Requires a disposable server");
  const headers = { Authorization: `Bearer ${process.env.ARUVI_MOBILE_API_TOKEN}` };
  async function mcp(action, args) {
    const response = await request.post("/api/mcp", { headers: { Authorization: `Bearer ${process.env.ARUVI_MCP_API_TOKEN}`, Accept: "application/json, text/event-stream" },
      data: { jsonrpc: "2.0", id: 1, method: "tools/call", params: { name: "aruvi_agents", arguments: { action, arguments: args } } } });
    const result = (await response.json()).result;
    expect(result.isError).not.toBeTruthy();
    return result.structuredContent.result;
  }
  const fields = { primary_roles: ["specialist", "developer"], fallback_roles: ["coding"], coordinator_required: false };
  const payload = { stage_name: "coding", expected: null, fields };
  expect((await request.get("/api/mobile/routing-policies")).status()).toBe(401);
  expect((await request.post("/api/mobile/routing-policies", { data: payload })).status()).toBe(401);
  expect((await request.post("/api/mobile/routing-policies", { headers, data: { ...payload, stage_name: "draft" } })).status()).toBe(400);
  expect((await request.post("/api/mobile/routing-policies", { headers, data: { ...payload, fields: { ...fields, primary_roles: [" "] } } })).status()).toBe(400);
  await page.goto("/remote");
  await page.locator("#tokenInput").fill(process.env.ARUVI_MOBILE_API_TOKEN);
  await page.locator("#saveButton").click();
  await page.getByRole("button", { name: "Routing", exact: true }).click();
  await expect(page.getByText("Coordinator review runs only when an active coordinator is available", { exact: false })).toBeVisible();
  const card = page.locator("#routingList .tool-card").filter({ hasText: "coding · default" }).first();
  await card.getByText("Manage routing", { exact: true }).click();
  await card.getByLabel("Primary roles (JSON array) coding").fill("invalid json");
  await card.getByRole("button", { name: "Save routing" }).click();
  await expect(page.locator("#status")).toContainText("JSON arrays");
  await card.getByLabel("Primary roles (JSON array) coding").fill(JSON.stringify(fields.primary_roles));
  await card.getByLabel("Fallback roles (JSON array) coding").fill(JSON.stringify(fields.fallback_roles));
  await card.getByLabel("Request coordinator review when available").uncheck();
  const created = page.waitForResponse((response) => response.url().endsWith("/api/mobile/routing-policies") && response.request().method() === "POST");
  await card.getByRole("button", { name: "Save routing" }).click();
  const policy = await (await created).json();
  expect(policy).toMatchObject({ stage_name: "coding", ...fields });
  await expect(page.locator("#status")).toHaveText("Routing saved");
  expect((await request.post("/api/mobile/routing-policies", { headers, data: payload })).status()).toBe(409);
  let current = page.locator("#routingList .tool-card").filter({ hasText: "coding · custom" }).first();
  await current.getByText("Manage routing", { exact: true }).click();
  await current.getByLabel("Primary roles (JSON array) coding").fill('["unsaved_role"]');
  await mcp("upsert_workflow_stage_policy", { stage_name: "coding", primary_roles: ["concurrent_role"], fallback_roles: [], coordinator_required: true });
  const conflict = page.waitForResponse((response) => response.url().endsWith("/api/mobile/routing-policies") && response.request().method() === "POST");
  await current.getByRole("button", { name: "Save routing" }).click();
  expect((await conflict).status()).toBe(409);
  await expect(current.getByLabel("Primary roles (JSON array) coding")).toHaveValue('["unsaved_role"]');
  const oldSnapshot = { id: policy.id, ...fields };
  expect((await request.post("/api/mobile/routing-policies/delete", { headers, data: { stage_name: "coding", expected: oldSnapshot } })).status()).toBe(409);
  await page.getByRole("button", { name: "Refresh", exact: true }).click();
  current = page.locator("#routingList .tool-card").filter({ hasText: "coding · custom" }).first();
  await current.getByText("Manage routing", { exact: true }).click();
  await expect(current.getByLabel("Primary roles (JSON array) coding")).toHaveValue(/concurrent_role/);
  await current.getByLabel("Fallback roles (JSON array) coding").fill('["fallback_role"]');
  await current.getByRole("button", { name: "Save routing" }).click();
  await expect(page.locator("#status")).toHaveText("Routing saved");
  const saved = (await mcp("list_workflow_stage_policies", {})).find((item) => item.stage_name === "coding");
  expect(saved).toMatchObject({ primary_roles: ["concurrent_role"], fallback_roles: ["fallback_role"], coordinator_required: true });
  current = page.locator("#routingList .tool-card").filter({ hasText: "coding · custom" }).first();
  await current.getByText("Manage routing", { exact: true }).click();
  page.once("dialog", (dialog) => dialog.accept());
  await current.getByRole("button", { name: "Reset routing to default" }).click();
  await expect(page.locator("#status")).toHaveText("Routing reset");
  expect((await mcp("list_workflow_stage_policies", {})).some((item) => item.stage_name === "coding")).toBe(false);
});

test("workflow replaces a disabled coordinator before the next review", async ({ page, request }) => {
  test.skip(process.env.ARUVI_ISOLATED_WORKFLOW !== "1", "Requires a disposable server and simulated provider");
  test.setTimeout(180000);
  const headers = { Authorization: `Bearer ${process.env.ARUVI_MOBILE_API_TOKEN}` };
  async function post(path, data) {
    const response = await request.post(path, { headers, data });
    expect(response.ok(), await response.text()).toBeTruthy();
    return response.json();
  }
  async function mcp(name, action, args) {
    const response = await request.post("/api/mcp", { headers: { Authorization: `Bearer ${process.env.ARUVI_MCP_API_TOKEN}`, Accept: "application/json, text/event-stream" },
      data: { jsonrpc: "2.0", id: 1, method: "tools/call", params: { name, arguments: { action, arguments: args } } } });
    const result = (await response.json()).result;
    expect(result.isError).not.toBeTruthy();
    return result.structuredContent.result;
  }
  await request.post("http://127.0.0.1:19876/coordinator/reset");
  const provider = await post("/api/mobile/providers", { name: "Coordinator fixture provider", base_url: process.env.ARUVI_TEST_MODEL_URL.replace(/\/v1\/?$/, "/coordinator/v1") });
  const model = await post("/api/mobile/models", { provider_id: provider.id, name: "browser-test-model" });
  const existing = await (await request.get("/api/mobile/agents", { headers })).json();
  for (const agent of existing.filter((item) => item.enabled && item.employment_status === "active")) await post("/api/mobile/agent-model-bindings", { agent_id: agent.id, model_id: model.id });
  const product = await post("/api/mobile/products", { name: "Coordinator fixture product" });
  const repository = await post("/api/mobile/repositories", { name: "Coordinator fixture repository", local_path: process.env.ARUVI_TEST_REPOSITORY_PATH });
  await post("/api/mobile/repository-attachments", { product_id: product.id, repository_id: repository.id });
  const team = await post("/api/mobile/teams", { name: "Coordinator fixture team", department: "", description: "", enabled: true, max_concurrent_workflows: 2 });
  await post("/api/mobile/team-assignments", { team_id: team.id, product_id: product.id, scope_type: "product", scope_id: product.id });
  async function createAgent(name, role) {
    const agent = await post("/api/mobile/agents", { name, role, description: "", prompt_template_ref: "", allowed_tools: [], skill_tags: [], boundaries: {}, enabled: true, employment_status: "active" });
    await post("/api/mobile/agent-model-bindings", { agent_id: agent.id, model_id: model.id });
    return agent;
  }
  const first = await createAgent("Coordinator first", "manager");
  const second = await createAgent("Coordinator second", "coordinator");
  await post("/api/mobile/team-members", { team_id: team.id, agent_id: first.id, title: "", is_lead: true });
  await post("/api/mobile/team-members", { team_id: team.id, agent_id: second.id, title: "", is_lead: false });
  await mcp("aruvi_settings", "set_setting", { key: "workflow.auto_approve_plan", value: "false" });
  await post("/api/mobile/workflow-test-config", { product_id: product.id, mode: "disabled" });
  await page.goto("/remote");
  await page.locator("#tokenInput").fill(process.env.ARUVI_MOBILE_API_TOKEN);
  await page.locator("#saveButton").click();
  await page.getByRole("button", { name: "Work", exact: true }).click();
  await page.locator("#workProductSelect").selectOption(product.id);
  await page.locator("#workTitleInput").fill("Coordinator handoff fixture");
  await page.locator("#workDescriptionInput").fill("Exercise coordinator reassignment with a simulated provider.");
  await page.locator("#workAcceptanceInput").fill("A generated fixture exists.");
  const created = page.waitForResponse((response) => response.url().endsWith("/api/mobile/work-items") && response.request().method() === "POST");
  await page.locator("#createWorkButton").click();
  const work = await (await created).json();
  const started = page.waitForResponse((response) => response.url().endsWith(`/work-items/${work.id}/approve`), { timeout: 60000 });
  await page.locator("#approveWorkButton").click();
  const start = await started;
  expect(start.ok(), await start.text()).toBeTruthy();
  await expect(page.locator("#deliveryStatus")).toHaveText("pending plan approval", { timeout: 60000 });
  const before = await (await request.get(`/api/mobile/work-items/${work.id}/delivery`, { headers })).json();
  const initialRuns = await mcp("aruvi_workflows", "list_agent_runs_for_workflow", { workflow_run_id: before.workflow_run.id });
  expect(initialRuns.filter((run) => run.stage === "coordinator_review").length).toBeGreaterThanOrEqual(2);
  expect(initialRuns.filter((run) => run.stage === "coordinator_review").every((run) => run.agent_id === first.id)).toBe(true);
  await mcp("aruvi_agents", "update_agent_definition", { id: first.id, enabled: false });
  const advanced = page.waitForResponse((response) => response.url().includes("/api/mobile/workflows/") && response.url().endsWith("/action"), { timeout: 90000 });
  await page.getByRole("button", { name: "Approve Gate" }).click();
  const advance = await advanced;
  expect(advance.ok(), await advance.text()).toBeTruthy();
  await expect(page.locator("#deliveryStatus")).toHaveText("pending test review", { timeout: 90000 });
  const after = await (await request.get(`/api/mobile/work-items/${work.id}/delivery`, { headers })).json();
  expect(after.workflow_run.coordinator_agent_id).toBe(second.id);
  const runs = await mcp("aruvi_workflows", "list_agent_runs_for_workflow", { workflow_run_id: after.workflow_run.id });
  const reviews = runs.filter((run) => run.stage === "coordinator_review");
  expect(reviews.filter((run) => run.agent_id === first.id)).toHaveLength(initialRuns.filter((run) => run.stage === "coordinator_review").length);
  expect(reviews.some((run) => run.agent_id === second.id)).toBe(true);
});

test("workflow commits and pushes generated files to a disposable local remote", async ({ page, request }) => {
  test.skip(process.env.ARUVI_ISOLATED_WORKFLOW !== "1" || process.env.ARUVI_TEST_PUSH_REPOSITORY_PATH !== "/repositories/push-fixture", "Requires the disposable local Git fixture");
  test.setTimeout(180000);
  const headers = { Authorization: `Bearer ${process.env.ARUVI_MOBILE_API_TOKEN}` };
  async function post(path, data) {
    const response = await request.post(path, { headers, data });
    expect(response.ok(), await response.text()).toBeTruthy();
    return response.json();
  }
  async function mcp(name, action, args) {
    const response = await request.post("/api/mcp", { headers: { Authorization: `Bearer ${process.env.ARUVI_MCP_API_TOKEN}`, Accept: "application/json, text/event-stream" },
      data: { jsonrpc: "2.0", id: 1, method: "tools/call", params: { name, arguments: { action, arguments: args } } } });
    const result = (await response.json()).result;
    expect(result.isError).not.toBeTruthy();
    return result.structuredContent.result;
  }
  await request.post("http://127.0.0.1:19876/workflow/reset");
  const provider = await post("/api/mobile/providers", { name: "Push fixture provider", base_url: process.env.ARUVI_TEST_MODEL_URL.replace(/\/v1\/?$/, "/workflow/v1") });
  const model = await post("/api/mobile/models", { provider_id: provider.id, name: "browser-test-model" });
  const agents = await (await request.get("/api/mobile/agents", { headers })).json();
  for (const agent of agents.filter((item) => item.enabled && item.employment_status === "active")) await post("/api/mobile/agent-model-bindings", { agent_id: agent.id, model_id: model.id });
  await mcp("aruvi_settings", "set_setting", { key: "workflow.auto_approve_plan", value: "false" });
  await mcp("aruvi_settings", "set_setting", { key: "workflow.auto_approve_test_review", value: "false" });
  const product = await post("/api/mobile/products", { name: "Push fixture product" });
  const repository = await post("/api/mobile/repositories", { name: "Push fixture repository", local_path: process.env.ARUVI_TEST_PUSH_REPOSITORY_PATH });
  await post("/api/mobile/repository-attachments", { product_id: product.id, repository_id: repository.id });
  await post("/api/mobile/workflow-test-config", { product_id: product.id, mode: "custom", argv: ["/bin/true"] });
  await page.goto("/remote");
  await page.locator("#tokenInput").fill(process.env.ARUVI_MOBILE_API_TOKEN);
  await page.locator("#saveButton").click();
  await page.getByRole("button", { name: "Work", exact: true }).click();
  await page.locator("#workProductSelect").selectOption(product.id);
  await page.locator("#workTitleInput").fill("Push generated fixture");
  await page.locator("#workDescriptionInput").fill("Create a generated file and deliver it to the local fixture remote.");
  await page.locator("#workAcceptanceInput").fill("The generated file is present in the local remote.");
  const created = page.waitForResponse((response) => response.url().endsWith("/api/mobile/work-items") && response.request().method() === "POST");
  await page.locator("#createWorkButton").click();
  const work = await (await created).json();
  const started = page.waitForResponse((response) => response.url().endsWith(`/work-items/${work.id}/approve`), { timeout: 60000 });
  await page.locator("#approveWorkButton").click();
  const start = await started;
  expect(start.ok(), await start.text()).toBeTruthy();
  await expect(page.locator("#deliveryStatus")).toHaveText("pending plan approval", { timeout: 60000 });
  const coding = page.waitForResponse((response) => response.url().includes("/api/mobile/workflows/") && response.url().endsWith("/action"), { timeout: 90000 });
  await page.getByRole("button", { name: "Approve Gate" }).click();
  const codingResponse = await coding;
  expect(codingResponse.ok(), await codingResponse.text()).toBeTruthy();
  await expect(page.locator("#deliveryStatus")).toHaveText("pending test review", { timeout: 90000 });
  const push = page.waitForResponse((response) => response.url().includes("/api/mobile/workflows/") && response.url().endsWith("/action"), { timeout: 90000 });
  await page.getByRole("button", { name: "Approve Gate" }).click();
  const pushResponse = await push;
  expect(pushResponse.ok(), await pushResponse.text()).toBeTruthy();
  await expect(page.locator("#deliveryStatus")).toHaveText("done", { timeout: 90000 });
});

for (const scenario of ["staged", "changed", "remote"]) test(`workflow handles failed ${scenario} delivery`, async ({ page, request }) => {
  test.skip(process.env.ARUVI_ISOLATED_WORKFLOW !== "1", "Requires disposable local Git fixtures");
  test.setTimeout(180000);
  const headers = { Authorization: `Bearer ${process.env.ARUVI_MOBILE_API_TOKEN}` };
  async function post(path, data) {
    const response = await request.post(path, { headers, data });
    expect(response.ok(), await response.text()).toBeTruthy();
    return response.json();
  }
  async function mcp(name, action, args) {
    const response = await request.post("/api/mcp", { headers: { Authorization: `Bearer ${process.env.ARUVI_MCP_API_TOKEN}`, Accept: "application/json, text/event-stream" },
      data: { jsonrpc: "2.0", id: 1, method: "tools/call", params: { name, arguments: { action, arguments: args } } } });
    const result = (await response.json()).result;
    expect(result.isError).not.toBeTruthy();
    return result.structuredContent.result;
  }
  await request.post("http://127.0.0.1:19876/workflow/reset");
  const provider = await post("/api/mobile/providers", { name: `Push ${scenario} provider`, base_url: process.env.ARUVI_TEST_MODEL_URL.replace(/\/v1\/?$/, "/workflow/v1") });
  const model = await post("/api/mobile/models", { provider_id: provider.id, name: "browser-test-model" });
  const agents = await (await request.get("/api/mobile/agents", { headers })).json();
  for (const agent of agents.filter((item) => item.enabled && item.employment_status === "active")) await post("/api/mobile/agent-model-bindings", { agent_id: agent.id, model_id: model.id });
  await mcp("aruvi_settings", "set_setting", { key: "workflow.auto_approve_plan", value: "false" });
  await mcp("aruvi_settings", "set_setting", { key: "workflow.auto_approve_test_review", value: "false" });
  const product = await post("/api/mobile/products", { name: `Push ${scenario} product` });
  const repository = await post("/api/mobile/repositories", { name: `Push ${scenario} repo`, local_path: `/repositories/push-${scenario}-fixture` });
  await post("/api/mobile/repository-attachments", { product_id: product.id, repository_id: repository.id });
  await post("/api/mobile/workflow-test-config", { product_id: product.id, mode: "custom", argv: ["/bin/true"] });
  await page.goto("/remote");
  await page.locator("#tokenInput").fill(process.env.ARUVI_MOBILE_API_TOKEN);
  await page.locator("#saveButton").click();
  await page.getByRole("button", { name: "Work", exact: true }).click();
  await page.locator("#workProductSelect").selectOption(product.id);
  await page.locator("#workTitleInput").fill(`Push ${scenario} fixture`);
  await page.locator("#workDescriptionInput").fill("Exercise safe delivery in a disposable repository.");
  const created = page.waitForResponse((response) => response.url().endsWith("/api/mobile/work-items") && response.request().method() === "POST");
  await page.locator("#createWorkButton").click();
  const work = await (await created).json();
  const started = page.waitForResponse((response) => response.url().endsWith(`/work-items/${work.id}/approve`), { timeout: 60000 });
  await page.locator("#approveWorkButton").click();
  const start = await started; expect(start.ok(), await start.text()).toBeTruthy();
  await expect(page.locator("#deliveryStatus")).toHaveText("pending plan approval", { timeout: 60000 });
  const coding = page.waitForResponse((response) => response.url().includes("/api/mobile/workflows/") && response.url().endsWith("/action"), { timeout: 90000 });
  await page.getByRole("button", { name: "Approve Gate" }).click();
  const coded = await coding; expect(coded.ok(), await coded.text()).toBeTruthy();
  await expect(page.locator("#deliveryStatus")).toHaveText("pending test review", { timeout: 90000 });
  if (scenario === "changed") await mcp("aruvi_repositories", "write_repository_file", { repository_id: repository.id, relative_path: "src/generated.txt", content: "edited by user after coding\n" });
  const delivery = page.waitForResponse((response) => response.url().includes("/api/mobile/workflows/") && response.url().endsWith("/action"), { timeout: 90000 });
  await page.getByRole("button", { name: "Approve Gate" }).click();
  const rejected = await delivery;
  expect(rejected.ok()).toBe(false);
  const error = await rejected.text();
  expect(error).toContain(scenario === "staged" ? "staged changes" : scenario === "changed" ? "changed after coding" : "Failed to push");
  const state = await (await request.get(`/api/mobile/work-items/${work.id}/delivery`, { headers })).json();
  expect(state.workflow_run.status).toBe("failed");
  expect(state.retryable_git_delivery).toBe(true);
  expect((await request.post(`/api/mobile/workflows/${state.workflow_run.id}/retry-git-delivery`)).status()).toBe(401);
  if (scenario === "staged") return;
  if (scenario === "changed") await mcp("aruvi_repositories", "write_repository_file", { repository_id: repository.id, relative_path: "src/generated.txt", content: "generated by workflow test\n" });
  if (scenario === "remote") {
    await mcp("aruvi_repositories", "write_repository_file", { repository_id: repository.id, relative_path: "allow-push", content: "allow fixture push\n" });
    await expect.poll(async () => JSON.stringify(await mcp("aruvi_repositories", "list_repository_tree", { repository_id: repository.id })), { timeout: 10000 }).toContain("origin-ready");
  }
  const retryButton = page.getByRole("button", { name: "Retry Git delivery" });
  await expect(retryButton).toBeVisible();
  page.once("dialog", (dialog) => dialog.accept());
  const retry = page.waitForResponse((response) => response.url().endsWith("/retry-git-delivery"), { timeout: 90000 });
  await retryButton.click();
  const retryResponse = await retry;
  expect(retryResponse.ok(), await retryResponse.text()).toBeTruthy();
  await expect(page.locator("#deliveryStatus")).toHaveText("done", { timeout: 90000 });
  const finished = await (await request.get(`/api/mobile/work-items/${work.id}/delivery`, { headers })).json();
  expect(finished.workflow_run.status).toBe("completed");
  expect(finished.retryable_git_delivery).toBe(false);
});
