import { test, expect } from "@playwright/test";
import { copyFile, readFile } from "node:fs/promises";
import { execFileSync } from "node:child_process";
import { createHmac } from "node:crypto";

const token = process.env.ARUVI_MOBILE_API_TOKEN;
const mcpToken = process.env.ARUVI_MCP_API_TOKEN;

test("browser rotates bridge tokens without exposing saved values", async ({ page, request }) => {
  test.skip(process.env.ARUVI_TEST_BRIDGE_TOKEN_ROTATION !== "1", "Requires disposable server with saved mobile token");
  const next = process.env.ARUVI_TEST_NEW_MOBILE_TOKEN;
  const nextMcp = process.env.ARUVI_TEST_NEW_MCP_TOKEN;
  const headers = { Authorization: `Bearer ${token}` };
  const before = await (await request.get("/api/mobile/bridge-tokens", { headers })).text();
  expect(before).not.toContain(token);
  expect(before).not.toContain(mcpToken);
  expect(JSON.parse(before).mobile.environment_override).toBe(false);
  expect(JSON.parse(before).mcp.environment_override).toBe(true);
  await page.goto("/remote");
  await page.locator("#tokenInput").fill(token);
  await page.locator("#saveButton").click();
  await page.getByRole("button", { name: "Activity", exact: true }).click();
  await page.getByText("Manage bridge tokens", { exact: true }).click();
  await page.getByRole("button", { name: "Load token status" }).click();
  await expect(page.locator("#status")).toHaveText("Bridge token status loaded");
  await expect(page.locator("#bridgeMcpTokenStatus")).toContainText("environment variable overrides");
  await page.locator("#bridgeMcpToken").fill(nextMcp);
  const changed = await request.post("/api/mcp", { headers: { Authorization: `Bearer ${mcpToken}` },
    data: { jsonrpc: "2.0", id: 2, method: "tools/call", params: { name: "aruvi_settings", arguments: { action: "set_setting", arguments: { key: "mcp.api_token", value: `${nextMcp}-concurrent` } } } } });
  expect((await changed.json()).result.isError).not.toBeTruthy();
  page.once("dialog", (dialog) => dialog.accept());
  await page.getByRole("button", { name: "Replace MCP token" }).click();
  await expect(page.locator("#status")).toContainText("Bridge token changed");
  await expect(page.locator("#bridgeMcpToken")).toHaveValue(nextMcp);
  page.once("dialog", (dialog) => dialog.accept());
  await page.getByRole("button", { name: "Discard and reload token status" }).click();
  await expect(page.locator("#bridgeMcpToken")).toHaveValue("");
  await page.locator("#bridgeMcpToken").fill(nextMcp);
  page.once("dialog", (dialog) => dialog.accept());
  await page.getByRole("button", { name: "Replace MCP token" }).click();
  await expect(page.locator("#status")).toContainText("environment still takes precedence");
  await expect(page.locator("#bridgeMcpToken")).toHaveValue("");
  const ping = { jsonrpc: "2.0", id: 1, method: "ping" };
  expect((await request.post("/api/mcp", { headers: { Authorization: `Bearer ${mcpToken}` }, data: ping })).status()).toBe(200);
  expect((await request.post("/api/mcp", { headers: { Authorization: `Bearer ${nextMcp}` }, data: ping })).status()).toBe(401);
  await page.locator("#bridgeMobileToken").fill(next);
  page.once("dialog", (dialog) => dialog.accept());
  await page.getByRole("button", { name: "Replace mobile token" }).click();
  await expect(page.locator("#status")).toHaveText("Bridge token updated");
  await expect(page.locator("#bridgeMobileToken")).toHaveValue("");
  await expect(page.locator("#tokenInput")).toHaveValue(next);
  expect((await request.get("/api/mobile/bridge-tokens", { headers })).status()).toBe(401);
  const after = await request.get("/api/mobile/bridge-tokens", { headers: { Authorization: `Bearer ${next}` } });
  expect(after.ok()).toBeTruthy();
  const text = await after.text();
  expect(text).not.toContain(next);
  expect(text).not.toContain(nextMcp);
});

test("browser configures bridge bind with conflict protection", async ({ page, request }) => {
  test.skip(!token || !mcpToken || process.env.ARUVI_TEST_INSPECT !== "1", "Requires a disposable server");
  const headers = { Authorization: `Bearer ${token}` };
  expect((await request.get("/api/mobile/bridge-bind")).status()).toBe(401);
  await page.goto("/remote");
  await page.locator("#tokenInput").fill(token);
  await page.locator("#saveButton").click();
  await page.getByRole("button", { name: "Activity", exact: true }).click();
  await page.getByText("Configure server bind", { exact: true }).click();
  await page.getByRole("button", { name: "Load bind settings" }).click();
  await expect(page.locator("#status")).toHaveText("Bind settings loaded");
  await page.locator("#bridgeBindHost").fill("localhost");
  await page.locator("#bridgeBindPort").fill("9001");
  const mcp = await request.post("/api/mcp", { headers: { Authorization: `Bearer ${mcpToken}` },
    data: { jsonrpc: "2.0", id: 1, method: "tools/call", params: { name: "aruvi_settings", arguments: { action: "set_setting", arguments: { key: "mobile.bind_port", value: "9000" } } } } });
  expect((await mcp.json()).result.isError).not.toBeTruthy();
  await page.getByRole("button", { name: "Save bind settings" }).click();
  await expect(page.locator("#status")).toContainText("Bridge settings changed");
  await expect(page.locator("#bridgeBindHost")).toHaveValue("localhost");
  await expect(page.locator("#bridgeBindPort")).toHaveValue("9001");
  const current = await (await request.get("/api/mobile/bridge-bind", { headers })).json();
  expect(current.raw[1]).toBe("9000");
  page.once("dialog", (dialog) => dialog.accept());
  await page.getByRole("button", { name: "Discard and reload bind settings" }).click();
  await expect(page.locator("#bridgeBindPort")).toHaveValue("9000");
  await page.locator("#bridgeBindHost").fill("::1");
  await page.locator("#bridgeBindPort").fill("9002");
  await page.getByRole("button", { name: "Save bind settings" }).click();
  await expect(page.locator("#status")).toContainText("Restart the server");
  expect((await (await request.get("/api/mobile/bridge-bind", { headers })).json()).raw).toEqual(["::1", "9002"]);
});

test("optional code editor preserves tabs and uses guarded saves", async ({page, request}) => {
  test.skip(process.env.ARUVI_TEST_INSPECT !== "1", "Requires a disposable remote fixture");
  const loader = await request.get("/assets/monaco/loader.js");
  expect(loader.ok()).toBeTruthy();
  expect(loader.headers()["content-type"]).toContain("javascript");
  expect(loader.headers()["x-content-type-options"]).toBe("nosniff");
  expect((await request.get("/assets/monaco/.git/config")).status()).toBe(404);
  expect((await request.get("/assets/monaco/editor/editor.main.json")).status()).toBe(404);
  const headers = {Authorization:`Bearer ${token}`};
  const registered = await request.post("/api/mobile/repositories", {headers, data:{name:"Code editor fixture",local_path:process.env.ARUVI_TEST_CODE_EDITOR_PATH || "/repositories/code-editor-fixture"}});
  expect(registered.ok(), await registered.text()).toBeTruthy();
  const repository = await registered.json();
  let failLoader = true;
  const external = [];
  const origin = new URL(process.env.ARUVI_REMOTE_URL || "http://127.0.0.1:8787").origin;
  page.on("request", (req) => { if (!req.url().startsWith(origin + "/") && !req.url().startsWith("blob:")) external.push(req.url()); });
  await page.route("**/assets/monaco/loader.js", (route) => {
    if (failLoader) { failLoader = false; return route.fulfill({status:404,body:"unavailable"}); }
    return route.continue();
  });
  await page.goto("/remote"); await page.locator("#tokenInput").fill(token); await page.locator("#saveButton").click();
  await page.getByRole("button", {name:"Workspaces",exact:true}).click();
  const downloadOpenCopy = async () => {
    const started = page.waitForEvent("download");
    await page.getByRole("button", {name:"Download open copy"}).click();
    const download = await started;
    return { name: download.suggestedFilename(), content: await readFile(await download.path(), "utf8") };
  };
  await expect(page.getByRole("button", {name:"Download open copy"})).toBeDisabled();
  await page.locator("#repositoryList .tool-card").filter({hasText:"Code editor fixture"}).getByRole("button", {name:"Browse workspace"}).click();
  await page.locator("#repositoryTree").getByRole("button", {name:"first.js",exact:true}).click();
  await expect(page.getByLabel("Selected file contents")).toHaveValue("const first = 1;\n");
  await page.getByRole("button", {name:"Use code editor"}).click();
  await expect(page.locator("#codeEditorStatus")).toContainText("unavailable");
  await expect(page.getByLabel("Selected file contents")).toBeVisible();
  await page.getByRole("button", {name:"Use code editor"}).click();
  await expect(page.locator("#codeEditorStatus")).toHaveText("Code editor active", {timeout:30000});
  await expect(page.locator("#repositoryCodeEditor .monaco-editor")).toBeVisible();
  await expect.poll(() => page.evaluate(() => codeEditor.getModel()?.getLanguageId())).toBe("javascript");
  const input = page.locator("#repositoryCodeEditor textarea.inputarea");
  await input.focus(); await page.keyboard.press("ControlOrMeta+A"); await page.keyboard.insertText("const first = 42;\n");
  await expect(page.locator("#repositoryFileTabs")).toContainText("first.js • unsaved");
  expect(await downloadOpenCopy()).toEqual({ name:"first.js", content:"const first = 42;\n" });
  const original = await request.post("/api/mcp", {headers:{Authorization:`Bearer ${mcpToken}`},data:{jsonrpc:"2.0",id:31,method:"tools/call",params:{name:"aruvi_repositories",arguments:{action:"read_repository_file",arguments:{repository_id:repository.id,relative_path:"first.js"}}}}});
  expect((await original.json()).result.structuredContent.result.content).toBe("const first = 1;\n");
  let releaseFile, arrivedFile;
  const fileHeld = new Promise((resolve) => { arrivedFile = resolve; });
  await page.route("**/api/mobile/repository-inspect", async (route) => {
    const body = route.request().postDataJSON();
    if (body?.action === "read_repository_file" && body.relative_path === "second.js") {
      await new Promise((resolve) => { releaseFile = resolve; arrivedFile(); });
    }
    return route.continue();
  });
  await page.locator("#repositoryTree").getByRole("button", {name:"second.js",exact:true}).click();
  await fileHeld;
  try {
    await expect.poll(() => page.evaluate(() => codeEditor.getOption(monaco.editor.EditorOption.readOnly))).toBe(true);
    await input.focus(); await page.keyboard.insertText("should-not-appear");
  } finally { releaseFile(); }
  await expect(page.locator("#repositoryFileTabs").getByRole("button", {name:"second.js",exact:true})).toHaveAttribute("aria-pressed", "true");
  await input.focus(); await page.keyboard.press("ControlOrMeta+A"); await page.keyboard.insertText("const second = 99;\n");
  await page.locator("#repositoryFileTabs").getByRole("button", {name:"first.js • unsaved"}).click();
  await expect(page.locator("#repositoryFileTabs").getByRole("button", {name:"first.js • unsaved"})).toHaveAttribute("aria-pressed", "true");
  await input.focus(); await page.keyboard.press("ControlOrMeta+Z");
  await expect.poll(() => page.evaluate(() => codeEditor.getValue())).not.toBe("const first = 42;\n");
  await page.keyboard.press("ControlOrMeta+Shift+Z");
  await expect.poll(() => page.evaluate(() => codeEditor.getValue())).toBe("const first = 42;\n");
  await page.keyboard.press("ControlOrMeta+S");
  await expect(page.locator("#status")).toHaveText("File saved");
  const result = await request.post("/api/mcp", {headers:{Authorization:`Bearer ${mcpToken}`},data:{jsonrpc:"2.0",id:1,method:"tools/call",params:{name:"aruvi_repositories",arguments:{action:"read_repository_file",arguments:{repository_id:repository.id,relative_path:"first.js"}}}}});
  expect((await result.json()).result.structuredContent.result.content).toBe("const first = 42;\n");
  const externalEdit = await request.post("/api/mobile/repository-file/save", {headers,data:{repository_id:repository.id,relative_path:"first.js",expected_content:"const first = 42;\n",content:"const first = 88;\n"}});
  expect(externalEdit.ok()).toBeTruthy();
  await input.focus(); await page.keyboard.press("ControlOrMeta+A"); await page.keyboard.insertText("const first = 7;\n");
  await page.keyboard.press("ControlOrMeta+S");
  await expect(page.locator("#status")).toContainText("File changed since it was opened");
  const preserved = await request.post("/api/mcp", {headers:{Authorization:`Bearer ${mcpToken}`},data:{jsonrpc:"2.0",id:2,method:"tools/call",params:{name:"aruvi_repositories",arguments:{action:"read_repository_file",arguments:{repository_id:repository.id,relative_path:"first.js"}}}}});
  expect((await preserved.json()).result.structuredContent.result.content).toBe("const first = 88;\n");
  await page.getByRole("button", {name:"Discard file edits"}).click();
  await page.locator("#repositoryFileTabs").getByRole("button", {name:"second.js • unsaved"}).click();
  await page.getByRole("button", {name:"Use plain text editor"}).click();
  await expect(page.getByLabel("Selected file contents")).toHaveValue("const second = 99;\n");
  await page.getByRole("button", {name:"Discard file edits"}).click();
  await page.locator("#repositoryTree").getByRole("button", {name:"mixed.js",exact:true}).click();
  await expect(page.locator("#repositoryFileTabs").getByRole("button", {name:"mixed.js",exact:true})).toHaveAttribute("aria-pressed", "true");
  await page.getByRole("button", {name:"Use code editor"}).click();
  await expect.poll(() => page.evaluate(() => codeEditor.getOption(monaco.editor.EditorOption.readOnly))).toBe(true);
  expect(await downloadOpenCopy()).toEqual({ name:"mixed.js", content:"one\r\ntwo\n" });
  await page.locator("#repositoryTree").getByRole("button", {name:"windows.js",exact:true}).click();
  await expect(page.locator("#repositoryFileTabs").getByRole("button", {name:"windows.js",exact:true})).toHaveAttribute("aria-pressed", "true");
  await expect.poll(() => page.evaluate(() => codeEditor.getOption(monaco.editor.EditorOption.readOnly))).toBe(false);
  await input.focus(); await page.keyboard.press("ControlOrMeta+A"); await page.keyboard.insertText("alpha\nbeta\n");
  expect(await downloadOpenCopy()).toEqual({ name:"windows.js", content:"alpha\r\nbeta\r\n" });
  await input.focus(); await page.keyboard.press("ControlOrMeta+S");
  await expect(page.locator("#status")).toHaveText("File saved");
  const windows = await request.post("/api/mcp", {headers:{Authorization:`Bearer ${mcpToken}`},data:{jsonrpc:"2.0",id:3,method:"tools/call",params:{name:"aruvi_repositories",arguments:{action:"read_repository_file",arguments:{repository_id:repository.id,relative_path:"windows.js"}}}}});
  expect((await windows.json()).result.structuredContent.result.content).toBe("alpha\r\nbeta\r\n");
  expect(external).toEqual([]);
});

test("browser and MCP edit workspaces with stale snapshot protection", async ({page,request}) => {
  test.skip(process.env.ARUVI_TEST_INSPECT !== "1", "Requires a disposable remote fixture");
  const headers={Authorization:`Bearer ${token}`}, mcpHeaders={Authorization:`Bearer ${mcpToken}`};
  const registered=await request.post("/api/mobile/repositories",{headers,data:{name:"Workspace edit fixture",local_path:process.env.ARUVI_TEST_CODE_EDITOR_PATH || "/repositories/code-editor-fixture"}});
  expect(registered.status()).toBe(201); const repository=await registered.json();
  const uri=`/api/mobile/repositories/${encodeURIComponent(repository.id)}/update`;
  const fields={name:"Browser draft",local_path:repository.local_path,remote_url:repository.remote_url,default_branch:repository.default_branch};
  expect((await request.post(uri,{data:{expected:repository,fields}})).status()).toBe(401);
  expect((await request.post(uri,{headers,data:{expected:repository,fields:{...fields,local_path:"/etc"}}})).status()).toBe(400);
  await page.goto("/remote"); await page.locator("#tokenInput").fill(token); await page.locator("#saveButton").click();
  await page.getByRole("button",{name:"Workspaces",exact:true}).click();
  let card=page.locator("#repositoryList .tool-card").filter({hasText:repository.local_path});
  await card.getByText("Edit workspace",{exact:true}).click(); await card.getByLabel("Edit display name").fill("Browser draft");
  const rpc=await request.post("/api/mcp",{headers:mcpHeaders,data:{jsonrpc:"2.0",id:50,method:"tools/call",params:{name:"aruvi_repositories",arguments:{action:"update_repository",arguments:{id:repository.id,name:"Concurrent workspace edit",local_path:repository.local_path,remote_url:repository.remote_url,default_branch:repository.default_branch,expected:repository}}}}});
  expect((await rpc.json()).result.isError).not.toBeTruthy();
  const conflict=page.waitForResponse(response=>response.url().endsWith("/update") && response.request().method()==="POST");
  await card.getByRole("button",{name:"Save workspace"}).click(); expect((await conflict).status()).toBe(409);
  await expect(card.getByLabel("Edit display name")).toHaveValue("Browser draft");
  await page.locator("#refreshWorkspacesButton").click(); card=page.locator("#repositoryList .tool-card").filter({hasText:repository.local_path});
  await card.getByText("Edit workspace",{exact:true}).click(); await card.getByLabel("Edit display name").fill("Browser updated workspace");
  const saved=page.waitForResponse(response=>response.url().endsWith("/update") && response.request().method()==="POST");
  await card.getByRole("button",{name:"Save workspace"}).click(); expect((await saved).status()).toBe(200);
  const rows=await (await request.get("/api/mobile/repositories",{headers})).json();
  expect(rows.find(item=>item.id===repository.id).name).toBe("Browser updated workspace");
});

test("browser and MCP import JSON and CSV from remote clients", async ({page, request}) => {
  test.skip(process.env.ARUVI_TEST_INSPECT !== "1", "Requires a disposable remote fixture");
  const headers = {Authorization:`Bearer ${token}`};
  const rpcHeaders = {Authorization:`Bearer ${mcpToken}`};
  expect((await request.get("/api/mobile/bulk-import")).status()).toBe(401);
  expect((await request.post("/api/mobile/bulk-import", {headers, data:{format:"json",content:"x".repeat(1048577)}})).status()).toBe(400);
  await page.goto("/remote"); await page.locator("#tokenInput").fill(token); await page.locator("#saveButton").click();
  await page.getByRole("button", {name:"Products",exact:true}).click();
  await page.getByText("Import JSON or CSV", {exact:true}).click();
  const content = JSON.stringify({product:{id:"remote-import-product",name:"Remote import"},productAreas:[{id:"remote-import-area",name:"Remote area"}]});
  await page.locator("#bulkImportFile").setInputFiles({name:"import.json",mimeType:"application/json",buffer:Buffer.from(content)});
  page.once("dialog", dialog => dialog.accept());
  const submitted = page.waitForResponse(response => response.url().endsWith("/api/mobile/bulk-import") && response.request().method()==="POST");
  await page.getByRole("button", {name:"Start import"}).click(); const response = await submitted;
  expect(response.status()).toBe(202); const job = await response.json(); expect(job.sourcePath).toBe("<inline>");
  await expect.poll(async () => (await (await request.get(`/api/mobile/bulk-import/${encodeURIComponent(job.id)}`, {headers})).json()).job.status).toBe("completed");
  const mcp = await request.post("/api/mcp", {headers:rpcHeaders,data:{jsonrpc:"2.0",id:45,method:"tools/call",params:{name:"aruvi_catalog",arguments:{action:"submit_inline_bulk_import",arguments:{format:"csv",content:"record_type,id,name\nproduct,remote-csv-product,Remote CSV\n"}}}}});
  const result=(await mcp.json()).result; expect(result.isError).not.toBeTruthy();
  await expect.poll(async () => (await (await request.get(`/api/mobile/bulk-import/${encodeURIComponent(result.structuredContent.result.id)}`, {headers})).json()).job.status).toBe("completed");
  const products=await request.get("/api/mobile/products",{headers}); const names=(await products.json()).map(item => item.name);
  expect(names).toContain("Remote import"); expect(names).toContain("Remote CSV");
});

test("browser approves and rejects work with stale decision protection", async ({ page, request }) => {
  test.skip(process.env.ARUVI_TEST_INSPECT !== "1", "Requires a disposable remote fixture");
  const headers = {Authorization:`Bearer ${token}`};
  const post = async (path, data) => { const response = await request.post(path, {headers,data}); expect(response.ok(), await response.text()).toBeTruthy(); return response.json(); };
  const call = async (name, action, args) => {
    const response = await request.post("/api/mcp", {headers:{Authorization:`Bearer ${mcpToken}`},data:{jsonrpc:"2.0",id:1,method:"tools/call",params:{name,arguments:{action,arguments:args}}}});
    const body = await response.json(); expect(body.result.isError,JSON.stringify(body.result)).not.toBeTruthy(); return body.result.structuredContent.result;
  };
  await call("aruvi_settings","set_setting",{key:"workflow.auto_start_after_work_item_approval",value:"false"});
  try {
    const product = await post("/api/mobile/products",{name:"Decision fixture"});
    const item = await post("/api/mobile/work-items",{product_id:product.id,title:"Decision task"});
    await page.goto("/remote"); await page.locator("#tokenInput").fill(token); await page.locator("#saveButton").click();
    await page.getByRole("button",{name:"Work",exact:true}).click(); await page.locator("#workProductSelect").selectOption(product.id);
    await page.locator("#workList .work-card").filter({hasText:"Decision task"}).click();
    page.once("dialog",dialog => dialog.accept()); await page.getByRole("button",{name:"Approve",exact:true}).click();
    await expect(page.locator("#status")).toHaveText("Work item approved");
    expect((await (await request.get(`/api/mobile/work-items/${item.id}`,{headers})).json()).status).toBe("approved");
    await call("aruvi_work_items","update_work_item",{id:item.id,description:"Concurrent edit"});
    page.once("dialog",dialog => dialog.accept("Needs revision")); await page.getByRole("button",{name:"Reject",exact:true}).click();
    await expect(page.locator("#status")).toContainText("changed");
    await page.locator("#refreshDeliveryButton").click();
    page.once("dialog",dialog => dialog.accept("Needs revision")); await page.getByRole("button",{name:"Reject",exact:true}).click();
    await expect(page.locator("#status")).toHaveText("Work item rejected");
    const delivery = await (await request.get(`/api/mobile/work-items/${item.id}/delivery`,{headers})).json();
    expect(delivery.work_item.status).toBe("draft");
    expect(delivery.approvals.filter((entry) => entry.approval_type === "task_approval").map((entry) => entry.status).sort()).toEqual(["approved","rejected"]);
  } finally {
    await call("aruvi_settings","set_setting",{key:"workflow.auto_start_after_work_item_approval",value:"true"});
  }
});

test("browser manages versioned agent templates and detects MCP edits", async ({ page, request }) => {
  test.skip(process.env.ARUVI_TEST_INSPECT !== "1", "Requires a disposable remote fixture");
  const mcp = async (action, args) => {
    const response = await request.post("/api/mcp", {headers:{Authorization:`Bearer ${mcpToken}`},data:{jsonrpc:"2.0",id:1,method:"tools/call",params:{name:"aruvi_agents",arguments:{action,arguments:args}}}});
    const result = (await response.json()).result; expect(result.isError,JSON.stringify(result)).not.toBeTruthy(); return result.structuredContent.result;
  };
  await page.goto("/remote"); await page.locator("#tokenInput").fill(token); await page.locator("#saveButton").click();
  await page.getByRole("button",{name:"Agents",exact:true}).click();
  const create = page.locator("#createPromptTemplate");
  await create.getByLabel("Template name").fill("Browser template fixture");
  await create.getByLabel("Template instructions").fill("Explain decisions clearly.");
  const response = page.waitForResponse((r)=>r.url().endsWith("/api/mobile/prompt-templates") && r.request().method()==="POST");
  await create.getByRole("button",{name:"Create template"}).click();
  const template = await (await response).json();
  await expect(page.locator("#status")).toHaveText("Template created");
  let card = page.locator("#promptTemplateList details").filter({hasText:"Browser template fixture"});
  await card.locator("summary").click();
  await card.getByRole("button",{name:"Use for new agent"}).click();
  await expect(page.locator("#createAgentEditor").getByLabel("Prompt template reference")).toHaveValue(`template:${template.id}`);
  await card.getByLabel("Template instructions Browser template fixture").fill("Unsaved draft");
  await mcp("update_prompt_template", {id:template.id,expected_version:1,name:template.name,role:"*",template_text:"Changed through MCP"});
  await card.getByRole("button",{name:"Save template"}).click();
  await expect(page.locator("#status")).toContainText("changed");
  await expect(card.getByLabel("Template instructions Browser template fixture")).toHaveValue("Unsaved draft");
  await page.getByRole("button",{name:"Refresh templates"}).click();
  card = page.locator("#promptTemplateList details").filter({hasText:"Browser template fixture"});
  await card.locator("summary").click();
  await expect(card.getByLabel("Template instructions Browser template fixture")).toHaveValue("Changed through MCP");
  const agentForm = page.locator("#createAgentEditor");
  await agentForm.getByLabel("Agent name",{exact:true}).fill("Template assigned fixture");
  const agentResponse = page.waitForResponse((r)=>r.url().endsWith("/api/mobile/agents") && r.request().method()==="POST");
  await agentForm.getByRole("button",{name:"Create agent"}).click();
  const agent = await (await agentResponse).json(); expect(agent.prompt_template_ref).toBe(`template:${template.id}`);
  const blocked = await request.post("/api/mobile/prompt-templates/delete",{headers:{Authorization:`Bearer ${token}`},data:{id:template.id,expected_version:2}});
  expect(blocked.status()).toBe(409);
  await mcp("delete_agent_definition",{id:agent.id});
  page.once("dialog",dialog=>dialog.accept()); await card.getByRole("button",{name:"Delete template"}).click();
  await expect(page.locator("#status")).toHaveText("Template deleted");
});

test("browser reorders work with drag and keyboard buttons while detecting stale order", async ({ page, request }) => {
  test.skip(process.env.ARUVI_TEST_INSPECT !== "1", "Requires a disposable remote fixture");
  const headers = {Authorization:`Bearer ${token}`};
  const post = async (path, data) => { const response = await request.post(path,{headers,data}); expect(response.ok(),await response.text()).toBeTruthy(); return response.json(); };
  const product = await post("/api/mobile/products", {name:"Order fixture"});
  for (const title of ["Order A", "Order B", "Order C"]) await post("/api/mobile/work-items", {product_id:product.id,title});
  const cards = page.locator("#workList .work-card");
  await page.goto("/remote"); await page.locator("#tokenInput").fill(token); await page.locator("#saveButton").click();
  await page.getByRole("button",{name:"Work",exact:true}).click(); await page.locator("#workProductSelect").selectOption(product.id);
  await expect(cards).toHaveCount(3); await expect(page.locator("#refreshWorkButton")).toBeEnabled();
  let titles = await cards.locator(".work-title").allTextContents();
  const original = [...titles];
  await page.getByRole("button",{name:`Move ${titles[1]} up`}).click();
  await expect(page.locator("#status")).toHaveText("Work item order saved");
  titles = await cards.locator(".work-title").allTextContents(); expect(titles).toEqual([original[1],original[0],original[2]]);
  await cards.nth(2).dragTo(cards.nth(0));
  await expect(page.locator("#status")).toHaveText("Work item order saved");
  titles = await cards.locator(".work-title").allTextContents(); expect(titles).toEqual([original[2],original[1],original[0]]);
  await page.locator("#workStatusFilter").selectOption("draft");
  await expect(page.getByRole("button",{name:`Move ${titles[1]} up`})).toBeDisabled();
  await page.locator("#workStatusFilter").selectOption(""); await expect(page.locator("#refreshWorkButton")).toBeEnabled();
  const response = await request.get(`/api/mobile/work-items?product_id=${product.id}&limit=100`,{headers});
  const fresh = (await response.json()).items;
  const moved = await request.post("/api/mobile/work-items/move",{headers,data:{product_id:product.id,offset:0,items:fresh.slice(0,2).map(({id,sort_order})=>({id,sort_order})),source_id:fresh[1].id}});
  expect(moved.ok(),await moved.text()).toBeTruthy();
  await page.getByRole("button",{name:`Move ${titles[1]} up`}).click();
  await expect(page.locator("#status")).toContainText("order changed");
  await page.locator("#refreshWorkButton").click(); await expect(page.locator("#status")).toHaveText("Connected");
  expect(await cards.locator(".work-title").allTextContents()).toEqual([original[1],original[2],original[0]]);
});

test("browser filters work and applies atomic batch decisions", async ({ page, request }) => {
  test.skip(process.env.ARUVI_TEST_INSPECT !== "1", "Requires a disposable remote fixture");
  const headers = {Authorization:`Bearer ${token}`};
  const post = async (path, data) => { const response = await request.post(path,{headers,data}); expect(response.ok(),await response.text()).toBeTruthy(); return response.json(); };
  const call = async (name, action, args) => {
    const response = await request.post("/api/mcp", {headers:{Authorization:`Bearer ${mcpToken}`},data:{jsonrpc:"2.0",id:1,method:"tools/call",params:{name,arguments:{action,arguments:args}}}});
    const body=await response.json(); expect(body.result.isError,JSON.stringify(body.result)).not.toBeTruthy(); return body.result.structuredContent.result;
  };
  await call("aruvi_settings","set_setting",{key:"workflow.auto_start_after_work_item_approval",value:"false"});
  try {
    const product=await post("/api/mobile/products",{name:"Batch fixture"});
    const a=await post("/api/mobile/work-items",{product_id:product.id,title:"Batch A"});
    const b=await post("/api/mobile/work-items",{product_id:product.id,title:"Batch B"});
    const c=await post("/api/mobile/work-items",{product_id:product.id,title:"Batch C"});
    await call("aruvi_work_items","update_work_item",{id:c.id,status:"approved"});
    await page.goto("/remote"); await page.locator("#tokenInput").fill(token); await page.locator("#saveButton").click();
    await page.getByRole("button",{name:"Work",exact:true}).click(); await page.locator("#workProductSelect").selectOption(product.id);
    await page.locator("#workStatusFilter").selectOption("draft");
    await expect(page.locator("#workList .work-card")).toHaveCount(2);
    await expect(page.locator("#workList")).not.toContainText("Batch C");
    await page.getByLabel("Select Batch A",{exact:true}).check(); await page.getByLabel("Select Batch B",{exact:true}).check();
    await call("aruvi_work_items","update_work_item",{id:b.id,description:"Concurrent change"});
    page.once("dialog",dialog=>dialog.accept()); await page.getByRole("button",{name:"Approve selected"}).click();
    await expect(page.locator("#status")).toContainText("changed");
    expect((await (await request.get(`/api/mobile/work-items/${a.id}`,{headers})).json()).status).toBe("draft");
    await page.locator("#refreshWorkButton").click();
    await page.getByLabel("Select Batch A",{exact:true}).check(); await page.getByLabel("Select Batch B",{exact:true}).check();
    page.once("dialog",dialog=>dialog.accept()); await page.getByRole("button",{name:"Approve selected"}).click();
    await expect(page.locator("#status")).toHaveText("2 work item(s) approved");
    await expect(page.locator("#workList .work-card")).toHaveCount(0);
    await page.locator("#workStatusFilter").selectOption("approved");
    await expect(page.locator("#workList .work-card")).toHaveCount(3);
    await page.getByLabel("Select Batch A",{exact:true}).check(); await page.getByLabel("Select Batch B",{exact:true}).check();
    page.once("dialog",dialog=>dialog.accept("Revise scope")); await page.getByRole("button",{name:"Reject selected"}).click();
    await expect(page.locator("#status")).toHaveText("2 work item(s) rejected");
    expect((await (await request.get(`/api/mobile/work-items/${a.id}`,{headers})).json()).status).toBe("draft");
    expect((await (await request.get(`/api/mobile/work-items/${b.id}`,{headers})).json()).status).toBe("draft");
  } finally {
    await call("aruvi_settings","set_setting",{key:"workflow.auto_start_after_work_item_approval",value:"true"});
  }
});

test("browser deletes a work item only after confirmation and stale checks", async ({ page, request }) => {
  test.skip(process.env.ARUVI_TEST_INSPECT !== "1", "Requires a disposable remote fixture");
  const headers = { Authorization:`Bearer ${token}` };
  const post = async (path, data) => { const response = await request.post(path, { headers, data }); expect(response.ok(), await response.text()).toBeTruthy(); return response.json(); };
  const product = await post("/api/mobile/products", {name:"Delete work fixture"});
  const parent = await post("/api/mobile/work-items", {product_id:product.id,title:"Delete parent fixture"});
  const child = await post("/api/mobile/work-items", {product_id:product.id,title:"Retained child fixture",parent_work_item_id:parent.id,work_item_type:"task"});
  await page.goto("/remote"); await page.locator("#tokenInput").fill(token); await page.locator("#saveButton").click();
  await page.getByRole("button", {name:"Work",exact:true}).click(); await page.locator("#workProductSelect").selectOption(product.id);
  await page.locator("#workList .work-card").filter({hasText:"Delete parent fixture"}).click();
  let prompt = "";
  page.once("dialog", (dialog) => { prompt = dialog.message(); return dialog.dismiss(); });
  await page.getByRole("button", {name:"Delete work item"}).click();
  await expect.poll(() => prompt).toContain("Direct child work items will be detached");
  expect((await request.get(`/api/mobile/work-items/${parent.id}`, {headers})).ok()).toBeTruthy();
  const mcp = await request.post("/api/mcp", {headers:{Authorization:`Bearer ${mcpToken}`},data:{jsonrpc:"2.0",id:1,method:"tools/call",params:{name:"aruvi_work_items",arguments:{action:"update_work_item",arguments:{id:parent.id,description:"Concurrent edit"}}}}});
  expect((await mcp.json()).result.isError).not.toBeTruthy();
  page.once("dialog", (dialog) => dialog.accept());
  await page.getByRole("button", {name:"Delete work item"}).click();
  await expect(page.locator("#status")).toContainText("changed");
  expect((await request.get(`/api/mobile/work-items/${parent.id}`, {headers})).ok()).toBeTruthy();
  await page.locator("#refreshDeliveryButton").click();
  page.once("dialog", (dialog) => dialog.accept());
  await page.getByRole("button", {name:"Delete work item"}).click();
  await expect(page.locator("#status")).toHaveText("Work item deleted");
  expect((await request.get(`/api/mobile/work-items/${parent.id}`, {headers})).ok()).toBeFalsy();
  expect((await (await request.get(`/api/mobile/work-items/${child.id}`, {headers})).json()).parent_work_item_id).toBeNull();
});

test("work item workspace can be assigned in browser and MCP with conflict protection", async ({ page, request }) => {
  test.skip(process.env.ARUVI_TEST_INSPECT !== "1", "Requires a disposable remote fixture");
  const headers = { Authorization: `Bearer ${token}` };
  async function create(path, data) {
    const response = await request.post(path, { headers, data });
    expect(response.ok(), await response.text()).toBeTruthy(); return response.json();
  }
  const product = await create("/api/mobile/products", { name: "Workspace selection fixture" });
  const item = await create("/api/mobile/work-items", { product_id: product.id, title: "Workspace task" });
  const first = await create("/api/mobile/repositories", { name: "Choice A", local_path: "/repositories/workspace-choice-a" });
  const second = await create("/api/mobile/repositories", { name: "Choice B", local_path: "/repositories/workspace-choice-b" });
  const mcp = async (arguments_) => {
    const response = await request.post("/api/mcp", { headers: { Authorization: `Bearer ${mcpToken}` }, data: { jsonrpc:"2.0", id:1, method:"tools/call", params:{name:"aruvi_work_items", arguments:{action:"assign_work_item_workspace", arguments:arguments_}} } });
    return (await response.json()).result;
  };
  await page.goto("/remote");
  await page.locator("#tokenInput").fill(token); await page.locator("#saveButton").click();
  await page.getByRole("button", { name: "Work", exact: true }).click();
  await page.locator("#workProductSelect").selectOption(product.id);
  await page.getByRole("button", { name: "Change work item workspace" }).click();
  await page.getByLabel("Work item workspace", { exact: true }).selectOption(first.id);
  await page.getByLabel("Work item branch option").selectOption("work_item");
  await expect(page.locator("#workWorkspacePreview")).toContainText("work/workspace-task");
  await page.getByRole("button", { name: "Save work item workspace" }).click();
  await expect(page.locator("#status")).toHaveText("Work item workspace saved");
  let saved = await (await request.get(`/api/mobile/work-items/${item.id}`, { headers })).json();
  expect(saved).toMatchObject({ repo_override_id:first.id, active_repo_id:first.id, branch_name:"work/workspace-task" });
  await page.getByRole("button", { name: "Change work item workspace" }).click();
  const changed = await mcp({ id:item.id, repository_id:second.id, branch_name:"feature/from-mcp" });
  expect(changed.isError).not.toBeTruthy();
  await page.getByRole("button", { name: "Save work item workspace" }).click();
  await expect(page.locator("#status")).toContainText("changed");
  saved = await (await request.get(`/api/mobile/work-items/${item.id}`, { headers })).json();
  expect(saved).toMatchObject({ repo_override_id:second.id, branch_name:"feature/from-mcp" });
  await page.getByRole("button", { name: "Cancel workspace edit" }).click();
  await page.locator("#refreshDeliveryButton").click();
  await page.getByRole("button", { name: "Change work item workspace" }).click();
  await page.getByRole("button", { name: "Use scope default" }).click();
  await expect(page.locator("#status")).toHaveText("Work item workspace saved");
  saved = await (await request.get(`/api/mobile/work-items/${item.id}`, { headers })).json();
  expect(saved).toMatchObject({ repo_override_id:null, active_repo_id:null, branch_name:null });
});

test("work list loads later pages and can retry a failed page", async ({ page, request }) => {
  test.skip(process.env.ARUVI_TEST_INSPECT !== "1", "Requires a disposable remote fixture");
  test.setTimeout(60000);
  const headers = { Authorization: `Bearer ${token}` };
  const productResponse = await request.post("/api/mobile/products", { headers, data: { name: "Paged work fixture" } });
  expect(productResponse.ok()).toBeTruthy();
  const product = await productResponse.json();
  const other = await (await request.post("/api/mobile/products", { headers, data: { name: "Other paged product" } })).json();
  expect((await request.post("/api/mobile/work-items", { headers, data: { product_id: other.id, title: "Other product item" } })).ok()).toBeTruthy();
  for (let i = 0; i < 101; i++) {
    const created = await request.post("/api/mobile/work-items", { headers, data: { product_id: product.id, title: `Paged item ${String(i).padStart(3, "0")}` } });
    expect(created.ok(), await created.text()).toBeTruthy();
  }
  const first = await (await request.get(`/api/mobile/work-items?product_id=${product.id}&limit=100`, { headers })).json();
  const second = await (await request.get(`/api/mobile/work-items?product_id=${product.id}&limit=100&offset=100`, { headers })).json();
  expect(first.items).toHaveLength(100); expect(first.has_more).toBe(true);
  expect(second.items).toHaveLength(1); expect(second.has_more).toBe(false);
  let failPage = true, holdPage = false, releasePage, pageArrived;
  const heldPage = new Promise((resolve) => { pageArrived = resolve; });
  await page.route("**/api/mobile/work-items?*", async (route) => {
    const url = new URL(route.request().url());
    if (url.searchParams.get("product_id") === product.id && url.searchParams.get("offset") === "100" && failPage) {
      failPage = false; return route.fulfill({ status: 503, body: "Temporary page failure" });
    }
    if (url.searchParams.get("product_id") === product.id && url.searchParams.get("offset") === "100" && holdPage) {
      await new Promise((resolve) => { releasePage = resolve; pageArrived(); });
    }
    return route.continue();
  });
  await page.goto("/remote");
  await page.locator("#tokenInput").fill(token);
  await page.locator("#saveButton").click();
  await page.getByRole("button", { name: "Work", exact: true }).click();
  await page.locator("#workProductSelect").selectOption(product.id);
  await expect(page.locator("#workList .work-card")).toHaveCount(100);
  const more = page.getByRole("button", { name: "Load more work items" });
  await expect(more).toBeVisible();
  await more.click();
  await expect(page.locator("#status")).toContainText("Temporary page failure");
  await expect(page.locator("#workList .work-card")).toHaveCount(100);
  await more.click();
  await expect(page.locator("#workList .work-card")).toHaveCount(101);
  await expect(page.locator("#workList")).toContainText(second.items[0].title);
  await expect(more).toBeHidden();
  await page.locator("#refreshWorkButton").click();
  await expect(page.locator("#workList .work-card")).toHaveCount(100);
  await expect(more).toBeVisible();
  holdPage = true;
  await more.click();
  await heldPage;
  const delayedResponse = page.waitForResponse((response) => response.url().includes(`product_id=${product.id}`) && response.url().includes("offset=100"));
  try {
    await page.locator("#workProductSelect").selectOption(other.id);
    await expect(page.locator("#workList")).toContainText("Other product item");
  } finally { releasePage(); }
  await delayedResponse;
  await expect(page.locator("#workList .work-card")).toHaveCount(1);
  await expect(page.locator("#workList")).not.toContainText(second.items[0].title);
});

test("remote UI connects and opens each operational section", async ({ page }) => {
  test.skip(!token, "Set ARUVI_MOBILE_API_TOKEN for a running remote server");
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto("/remote");
  await page.locator("#tokenInput").fill(token);
  await page.locator("#saveButton").click();
  await page.locator("#healthButton").click();
  await expect(page.locator("#status")).toHaveText("Connected");
  await page.getByRole("button", { name: "Activity", exact: true }).click();
  await expect(page.locator("#bridgeConfiguration")).toContainText(`${new URL(page.url()).origin}/api/mcp`);
  await expect(page.locator("#bridgeConfiguration")).toContainText("Configured server bind:");
  await expect(page.locator("#bridgeConfiguration")).not.toContainText(token);

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
  await page.locator("#draftTree .tree-node").first().click();
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
  await page.locator("#draftTree .tree-node").first().click();
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
  const matchRequests = [];
  page.on("request", (req) => { if (req.url().endsWith("/api/mobile/planner/match-actions") && req.method() === "POST") matchRequests.push(req.postDataJSON()); });
  try {
    await page.goto("/remote");
    await page.locator("#tokenInput").fill(token);
    await page.locator("#saveButton").click();
    await expect(page.locator("#plannerProductSelect option").filter({ hasText: product.name })).toHaveCount(1);
    await page.locator("#plannerProductSelect").selectOption(product.id);
    await page.locator("#composer").fill("Show the current plan");
    await page.locator("#sendButton").click();
    await expect(page.locator("#draftTree")).toContainText(product.name);
    await page.locator("#draftTree .tree-node").first().click();
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
    await expect.poll(() => matchRequests.find((entry) => entry.node?.label === "Draft area")?.node?.ancestors).toEqual([{ label: "Renamed draft", node_type: "product" }]);
    const matchPayload = { ...suggestionPayload, actions: [{ type: "create_product_area", name: "Draft area" }, { type: "create_product_area", name: "Other" }] };
    expect((await request.post("/api/mobile/planner/match-actions", { data: matchPayload })).status()).toBe(401);
    const matched = await (await request.post("/api/mobile/planner/match-actions", { headers, data: matchPayload })).json();
    expect(matched).toEqual([matchPayload.actions[0]]);
    const mcpMatch = await request.post("/api/mcp", { headers: { Authorization: `Bearer ${mcpToken}` }, data: { jsonrpc: "2.0", id: 4, method: "tools/call", params: { name: "aruvi_planner", arguments: { action: "match_planner_actions", arguments: matchPayload } } } });
    expect((await mcpMatch.json()).result.structuredContent.result).toEqual(matched);
    const scopedPayload = { node: { label: "Inventory", node_type: "capability", ancestors: [{ label: "Store", node_type: "product" }, { label: "Warehouse", node_type: "product_area" }] }, actions: [
      { type: "update_capability", target: { productName: "Store", productAreaName: "Warehouse", capabilityName: "Inventory" } },
      { type: "update_capability", target: { productName: "Store", productAreaName: "Sales", capabilityName: "Inventory" } },
      { type: "update_capability", target: { capabilityName: "Inventory" } },
    ] };
    const scoped = await (await request.post("/api/mobile/planner/match-actions", { headers, data: scopedPayload })).json();
    expect(scoped).toEqual([scopedPayload.actions[0], scopedPayload.actions[2]]);
    const scopedMcp = await request.post("/api/mcp", { headers: { Authorization: `Bearer ${mcpToken}` }, data: { jsonrpc: "2.0", id: 5, method: "tools/call", params: { name: "aruvi_planner", arguments: { action: "match_planner_actions", arguments: scopedPayload } } } });
    expect((await scopedMcp.json()).result.structuredContent.result).toEqual(scoped);
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
  test.setTimeout(120000);
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
  const copilotModel = copilotProvider ? await call("aruvi_models", "create_model_definition", { provider_id: copilotProvider.id, name: "independent-copilot-model" }) : null;
  let repositoryId;
  let secondRepositoryId;
  let staleCopilotPayload;
  try {
    const deniedSnapshot = await request.post("/api/mcp", { data: { jsonrpc: "2.0", id: 20, method: "tools/call", params: { name: "aruvi_catalog", arguments: { action: "get_product_book_source", arguments: { product_id: product.id } } } } });
    expect(deniedSnapshot.status()).toBe(401);
    const ownReference = await call("aruvi_catalog", "create_reference", { scope_type: "product_area", scope_id: area.id, title: "Snapshot area reference" });
    const otherReference = await call("aruvi_catalog", "create_reference", { scope_type: "product_area", scope_id: otherArea.id, title: "Other snapshot reference" });
    const snapshot = await call("aruvi_catalog", "get_product_book_source", { product_id: product.id });
    expect(snapshot.tree.product.id).toBe(product.id);
    expect(snapshot.tree.roots.some((node) => node.id === area.id)).toBe(true);
    expect(snapshot.references.some((reference) => reference.id === ownReference.id)).toBe(true);
    expect(snapshot.references.some((reference) => reference.id === otherReference.id)).toBe(false);
    expect(snapshot.work_items).toEqual([]);
    const renderedBook = await call("aruvi_catalog", "render_product_book", { product_id: product.id, format: "book", render_mode: "epub", trim_preset: "a5" });
    expect(renderedBook.productName).toBe(name);
    expect(renderedBook.html).toContain("Snapshot area reference");
    expect(renderedBook.html).not.toContain("Other snapshot reference");
    expect(renderedBook.trimPreset.id).toBe("a5");
    expect(renderedBook.tocItems.length).toBeGreaterThan(1);
    for (const trim of ["trade-6x9", "us-letter"]) {
      const book = await call("aruvi_catalog", "render_product_book", { product_id: product.id, format: "book", render_mode: "print", trim_preset: trim });
      expect(book.trimPreset.id).toBe(trim);
      expect(book.html).toContain("Snapshot area reference");
    }
    const renderedDocs = await call("aruvi_catalog", "render_product_book", { product_id: product.id, format: "docs" });
    expect(renderedDocs.html).toContain("Snapshot area reference");
    expect(renderedDocs.html).not.toContain("Other snapshot reference");
    const invalidRender = await request.post("/api/mcp", {
      headers: { Authorization: `Bearer ${mcpToken}`, Accept: "application/json, text/event-stream" },
      data: { jsonrpc: "2.0", id: 21, method: "tools/call", params: { name: "aruvi_catalog", arguments: { action: "render_product_book", arguments: { product_id: product.id, trim_preset: "invalid" } } } },
    });
    expect((await invalidRender.json()).result.isError).toBe(true);
    const invalidDocsMode = await request.post("/api/mcp", {
      headers: { Authorization: `Bearer ${mcpToken}`, Accept: "application/json, text/event-stream" },
      data: { jsonrpc: "2.0", id: 22, method: "tools/call", params: { name: "aruvi_catalog", arguments: { action: "render_product_book", arguments: { product_id: product.id, format: "docs", render_mode: "epub" } } } },
    });
    expect((await invalidDocsMode.json()).result.isError).toBe(true);
    const artifactArgs = { product_id: product.id, format: "epub", trim_preset: "a5" };
    const exported = await call("aruvi_catalog", "export_product_book", artifactArgs);
    expect(exported.media_type).toBe("application/epub+zip");
    const mcpEpub = Buffer.from(exported.data_base64, "base64");
    expect(mcpEpub.subarray(0, 4).toString("hex")).toBe("504b0304");
    const extractBook = (bytes) => execFileSync("python3", ["-c", "import io,sys,zipfile; print(zipfile.ZipFile(io.BytesIO(sys.stdin.buffer.read())).read('OEBPS/book.xhtml').decode())"], { input: bytes, encoding: "utf8" });
    expect(extractBook(mcpEpub)).toContain("Snapshot area reference");
    expect(extractBook(mcpEpub)).not.toContain("Other snapshot reference");
    const mcpPdf = await call("aruvi_catalog", "export_product_book", { product_id: product.id, format: "pdf", trim_preset: "a5" });
    expect(mcpPdf.media_type).toBe("application/pdf");
    expect(Buffer.from(mcpPdf.data_base64, "base64").subarray(0, 5).toString()).toBe("%PDF-");
    const artifactPath = `/api/mobile/products/${product.id}/book-export`;
    expect((await request.post(artifactPath, { data: { format: "epub" } })).status()).toBe(401);
    expect((await request.post(artifactPath, { headers: { Authorization: `Bearer ${token}` }, data: { format: "invalid" } })).status()).toBe(400);
    const htmlDownload = await request.post(artifactPath, { headers: { Authorization: `Bearer ${token}` }, data: { format: "book_html" } });
    expect(htmlDownload.status()).toBe(200);
    expect(htmlDownload.headers()["content-disposition"]).toContain("attachment;");
    const html = await htmlDownload.text();
    expect(html).toContain("Snapshot area reference");
    expect(html).not.toContain("Other snapshot reference");
    await page.goto("/remote");
    await page.locator("#tokenInput").fill(token);
    if (copilotProvider) { await page.locator("#providerInput").fill(copilotProvider.id); await page.locator("#modelInput").fill("browser-test-model"); }
    await page.locator("#saveButton").click();
    await page.getByRole("button", { name: "Products", exact: true }).click();
    await page.locator("#productList .product-card").filter({ has: page.getByText(name, { exact: true }) }).click();
    await expect(page.locator("#productBookDownload")).toBeEnabled();
    for (const format of ["docs_html", "book_html"]) {
      await page.locator("#productBookFormat").selectOption(format);
      const promise = page.waitForEvent("download");
      await page.locator("#productBookDownload").click();
      const downloadedHtml = await promise;
      const contents = await readFile(await downloadedHtml.path(), "utf8");
      expect(downloadedHtml.suggestedFilename()).toMatch(/\.html$/);
      expect(contents).toContain("Snapshot area reference");
      expect(contents).not.toContain("Other snapshot reference");
    }
    await page.locator("#productBookFormat").selectOption("epub");
    await page.locator("#productBookTrim").selectOption("a5");
    const downloadPromise = page.waitForEvent("download");
    await page.locator("#productBookDownload").click();
    const download = await downloadPromise;
    expect(download.suggestedFilename()).toMatch(/-book\.epub$/);
    const downloadPath = await download.path();
    const epubBytes = await readFile(downloadPath);
    expect(epubBytes.subarray(0, 4).toString("hex")).toBe("504b0304");
    expect(extractBook(epubBytes)).toContain("Snapshot area reference");
    expect(extractBook(epubBytes)).not.toContain("Other snapshot reference");
    if (process.env.ARUVI_TEST_BOOK_EPUB_CAPTURE) await copyFile(downloadPath, process.env.ARUVI_TEST_BOOK_EPUB_CAPTURE);
    await page.locator("#productBookFormat").selectOption("pdf");
    for (const preset of ["trade-6x9", "a5", "us-letter"]) {
      await page.locator("#productBookTrim").selectOption(preset);
      const pdfPromise = page.waitForEvent("download");
      await page.locator("#productBookDownload").click();
      const pdfDownload = await pdfPromise;
      expect(pdfDownload.suggestedFilename()).toMatch(/-book\.pdf$/);
      const pdfPath = await pdfDownload.path();
      expect((await readFile(pdfPath)).subarray(0, 5).toString()).toBe("%PDF-");
      if (process.env.ARUVI_TEST_BOOK_PDF_CAPTURE_DIR) await copyFile(pdfPath, `${process.env.ARUVI_TEST_BOOK_PDF_CAPTURE_DIR}/${preset}.pdf`);
    }
    const embeddedPng = "iVBORw0KGgoAAAANSUhEUgAAACAAAAAgCAYAAABzenr0AAAANUlEQVR4nO3QQQ0AMAwDsa5Awp/lpqHIxyFwkU+SO8VtM+4AAQIECBAgQIAAAQIECBAg8Akey+kBmSTYPGgAAAAASUVORK5CYII=";
    const embeddedJpeg = (await readFile(new URL("./fixtures/book-photo.jpg", import.meta.url))).toString("base64");
    const embeddedGif = "R0lGODdhEQANAJEAAAAAAB4eHv///wAAACH5BAkAAAMALAAAAAARAA0AAAINjI+py+0Po5y02otfAQA7";
    await call("aruvi_catalog", "update_product", { id: product.id, description: `![Inline diagram](data:image/png;base64,${embeddedPng})\n![Repeated diagram](data:image/png;base64,${embeddedPng})\n![Inline photo](data:image/jpeg;base64,${embeddedJpeg})\n![Inline GIF](data:image/gif;base64,${embeddedGif})` });
    const imageEpub = await call("aruvi_catalog", "export_product_book", artifactArgs);
    const imageEpubBytes = Buffer.from(imageEpub.data_base64, "base64");
    const imageManifest = JSON.parse(execFileSync("python3", ["-c", "import io,json,sys,zipfile; z=zipfile.ZipFile(io.BytesIO(sys.stdin.buffer.read())); print(json.dumps({'images':[n for n in z.namelist() if n.startswith('OEBPS/images/')], 'manifest':z.read('OEBPS/content.opf').decode(), 'book':z.read('OEBPS/book.xhtml').decode()}))"], { input: imageEpubBytes, encoding: "utf8" }));
    expect(imageManifest.images).toHaveLength(3);
    expect(imageManifest.manifest).toContain("image/png");
    expect(imageManifest.manifest).toContain("image/jpeg");
    expect(imageManifest.manifest).toContain("image/gif");
    const extractedGif = execFileSync("python3", ["-c", "import base64,io,sys,zipfile; z=zipfile.ZipFile(io.BytesIO(sys.stdin.buffer.read())); names=[n for n in z.namelist() if n.startswith('OEBPS/images/') and n.endswith('.gif')]; assert len(names)==1; sys.stdout.write(base64.b64encode(z.read(names[0])).decode())"], { input:imageEpubBytes, encoding:"utf8" });
    expect(extractedGif).toBe(embeddedGif);
    expect(imageManifest.book).toContain("Inline diagram");
    expect(imageManifest.book).toContain("Inline photo");
    expect(imageManifest.book).toContain("Inline GIF");
    expect(imageManifest.book).not.toContain("data:image/png");
    const imagePdf = await call("aruvi_catalog", "export_product_book", { product_id: product.id, format: "pdf", trim_preset: "a5" });
    expect(Buffer.from(imagePdf.data_base64, "base64").subarray(0, 5).toString()).toBe("%PDF-");
    await page.locator("#productBookFormat").selectOption("epub");
    const imageEpubPromise = page.waitForEvent("download");
    await page.locator("#productBookDownload").click();
    const imageEpubDownload = await imageEpubPromise;
    if (process.env.ARUVI_TEST_BOOK_IMAGE_CAPTURE_DIR) await copyFile(await imageEpubDownload.path(), `${process.env.ARUVI_TEST_BOOK_IMAGE_CAPTURE_DIR}/inline-image.epub`);
    await page.locator("#productBookFormat").selectOption("pdf");
    const imagePdfPromise = page.waitForEvent("download");
    await page.locator("#productBookDownload").click();
    const imagePdfDownload = await imagePdfPromise;
    if (process.env.ARUVI_TEST_BOOK_IMAGE_CAPTURE_DIR) await copyFile(await imagePdfDownload.path(), `${process.env.ARUVI_TEST_BOOK_IMAGE_CAPTURE_DIR}/inline-image.pdf`);
    const animatedGif = "R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAAsAAAAAAEAAQAAAgFEADs=";
    await call("aruvi_catalog", "update_product", { id: product.id, description: `![Animated](data:image/gif;base64,${animatedGif})` });
    for (const format of ["epub", "pdf"]) {
      const rejected = await request.post(artifactPath, { headers:{ Authorization:`Bearer ${token}` }, data:{ format } });
      expect(rejected.status()).toBe(400);
      expect(await rejected.text()).toContain("Animated GIF");
    }
    const animatedMcp = await request.post("/api/mcp", { headers:{ Authorization:`Bearer ${mcpToken}` }, data:{ jsonrpc:"2.0",id:24,method:"tools/call",params:{name:"aruvi_catalog",arguments:{action:"export_product_book",arguments:{product_id:product.id,format:"epub"}}} } });
    expect((await animatedMcp.json()).result.isError).toBeTruthy();
    await call("aruvi_catalog", "update_product", { id: product.id, description: "![Diagram](images/diagram.png)" });
    const imageFailure = await request.post(artifactPath, { headers: { Authorization: `Bearer ${token}` }, data: { format: "epub" } });
    expect(imageFailure.status()).toBe(400);
    expect(await imageFailure.text()).toContain("images");
    const pdfImageFailure = await request.post(artifactPath, { headers: { Authorization: `Bearer ${token}` }, data: { format: "pdf" } });
    expect(pdfImageFailure.status()).toBe(400);
    expect(await pdfImageFailure.text()).toContain("images");
    const mcpImageFailure = await request.post("/api/mcp", {
      headers: { Authorization: `Bearer ${mcpToken}`, Accept: "application/json, text/event-stream" },
      data: { jsonrpc: "2.0", id: 23, method: "tools/call", params: { name: "aruvi_catalog", arguments: { action: "export_product_book", arguments: artifactArgs } } },
    });
    expect((await mcpImageFailure.json()).result.isError).toBe(true);
    await call("aruvi_catalog", "update_product", { id: product.id, description: "" });
    await page.getByRole("button", { name: "Workspaces", exact: true }).click();
    if (copilotProvider) {
      const body = { provider_id: copilotProvider.id, model_name: "browser-test-model", messages: [{ role: "user", content: "Explain code" }] };
      expect((await request.post("/api/mobile/ide/copilot", { data: body })).status()).toBe(401);
      expect((await request.post("/api/mobile/ide/copilot/stream", { data: body })).status()).toBe(401);
      for (const invalid of [
        { ...body, max_tokens: 32769 },
        { ...body, temperature: 2.1 },
        { ...body, messages: [{ role: "tool", content: "not supported" }] },
        { ...body, messages: [{ role: "user", content: "x".repeat(256001) }] },
      ]) {
        for (const path of ["/api/mobile/ide/copilot", "/api/mobile/ide/copilot/stream"]) {
          expect((await request.post(path, { headers: { Authorization: `Bearer ${token}` }, data: invalid })).status()).toBe(400);
        }
        const rejected = await request.post("/api/mcp", { headers: { Authorization: `Bearer ${mcpToken}` }, data: { jsonrpc: "2.0", id: 88, method: "tools/call", params: { name: "aruvi_models", arguments: { action: "run_general_copilot", arguments: invalid } } } });
        expect((await rejected.json()).result.isError).toBeTruthy();
      }
      await page.getByLabel("Copilot question").fill("Explain code without a file");
      await page.getByRole("button", { name: "Ask Copilot", exact: true }).click();
      await expect(page.locator("#repositoryCopilotResult")).toContainText("Container provider reply");
      await page.getByLabel("Stream Copilot response").check();
      await page.getByLabel("Copilot question").fill("Continue without a file");
      await page.getByRole("button", { name: "Ask Copilot", exact: true }).click();
      await expect(page.locator("#repositoryCopilotResult")).toContainText("First second");
      await page.getByLabel("Stream Copilot response").uncheck();
    }
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
      await page.getByLabel("Filter workspace files").fill("nested/binary.bin");
      const binaryDownload = page.waitForEvent("download");
      await page.getByRole("button", { name: "Download saved nested/binary.bin" }).click();
      const downloaded = await binaryDownload;
      expect(downloaded.suggestedFilename()).toBe("binary.bin");
      const chunks = []; for await (const chunk of await downloaded.createReadStream()) chunks.push(chunk);
      expect(Buffer.concat(chunks)).toEqual(Buffer.from([0, 255, 17, 128, 1]));
      await page.getByLabel("Filter workspace files").fill("nested/boundary.bin");
      const boundaryEvent = page.waitForEvent("download");
      await page.getByRole("button", { name: "Download saved nested/boundary.bin" }).click();
      const boundary = await boundaryEvent, boundaryChunks = [];
      for await (const chunk of await boundary.createReadStream()) boundaryChunks.push(chunk);
      expect(Buffer.concat(boundaryChunks)).toEqual(Buffer.alloc(8 * 1024 * 1024));
      await page.getByLabel("Filter workspace files").fill("nested/too-large.bin");
      await page.getByRole("button", { name: "Download saved nested/too-large.bin" }).click();
      await expect(page.locator("#status")).toContainText("8 MiB");
      const binaryArgs = { repository_id:repositoryId, relative_path:"nested/binary.bin", expected_root:process.env.ARUVI_TEST_REPOSITORY_PATH };
      const binary = await call("aruvi_repositories", "read_repository_binary", binaryArgs);
      expect(Buffer.from(binary.content_base64, "base64")).toEqual(Buffer.from([0, 255, 17, 128, 1]));
      const binaryBody = { action:"read_repository_binary", ...binaryArgs };
      expect((await request.post("/api/mobile/repository-inspect", { data:binaryBody })).status()).toBe(401);
      expect((await request.post("/api/mobile/repository-inspect", { headers:{ Authorization:`Bearer ${token}` }, data:{...binaryBody, expected_root:"/stale"} })).status()).toBe(409);
      for (const path of ["../outside", "outside-link", "patch-alias.txt"]) {
        const rejected = await request.post("/api/mobile/repository-inspect", { headers:{ Authorization:`Bearer ${token}` }, data:{...binaryBody, relative_path:path} });
        expect(rejected.status()).toBe(400);
      }
      await page.getByLabel("Filter workspace files").fill("NESTED/target");
      await expect(page.locator("#repositoryTree").getByRole("button", { name: "target-file.txt", exact: true })).toBeVisible();
      await expect(page.locator("#repositoryTree button").filter({ hasText: "inspection.txt" })).toBeHidden();
      await page.locator("#repositoryTree").getByRole("button", { name: "target-file.txt", exact: true }).click();
      await expect(page.getByLabel("Selected file contents")).toHaveValue("nested fixture\n");
      if (copilotProvider) {
        await expect(page.locator("#repositoryCopilotResult")).toBeEmpty();
        await page.getByRole("button", { name: /Close .*target-file\.txt/ }).click();
        await expect(page.locator("#repositoryCopilotResult")).toContainText("Continue without a file");
        await page.getByRole("button", { name: "New conversation", exact: true }).click();
        await page.locator("#repositoryTree").getByRole("button", { name: "target-file.txt", exact: true }).click();
        await expect(page.getByLabel("Selected file contents")).toHaveValue("nested fixture\n");
      }
      await page.getByLabel("Filter workspace files").fill("no-such-file");
      await expect(page.locator("#repositoryTreeEmpty")).toBeVisible();
      await page.getByLabel("Filter workspace files").fill("");
      await expect(page.locator("#repositoryTreeEmpty")).toBeHidden();
      await page.locator("#repositoryTree").getByRole("button", { name: "inspection.txt", exact: true }).click();
      await expect(page.getByLabel("Selected file contents")).toHaveValue("workspace inspection fixture\n");
      await page.getByLabel("Selected file contents").fill("saved from browser\n");
      await page.getByLabel("Selected file contents").press("ControlOrMeta+s");
      await expect(page.locator("#status")).toHaveText("File saved");
      expect((await call("aruvi_repositories", "read_repository_file", { repository_id: repositoryId, relative_path: "inspection.txt" })).content).toBe("saved from browser\n");
      await page.getByLabel("Selected file contents").fill("unsaved during refresh\n");
      await call("aruvi_repositories", "write_repository_file", { repository_id: repositoryId, relative_path: "new-after-open.txt", content: "new file\n" });
      await page.getByRole("button", { name: "Refresh file tree" }).click();
      await expect(page.locator("#repositoryTree").getByRole("button", { name: "new-after-open.txt", exact: true })).toBeVisible();
      await expect(page.getByLabel("Selected file contents")).toHaveValue("unsaved during refresh\n");
      await page.getByRole("button", { name: "Discard file edits" }).click();
      await expect(page.getByLabel("Selected file contents")).toHaveValue("saved from browser\n");
      const patchFile = "patch-safety-fixture.txt";
      await call("aruvi_repositories", "write_repository_file", { repository_id: repositoryId, relative_path: patchFile, content: "alpha\nbeta\n" });
      const patchHash = (await call("aruvi_repositories", "get_repository_file_sha256", { repository_id: repositoryId, relative_path: patchFile })).sha256;
      const identityRequest = { action: "matching_repository_files", repository_id: repositoryId, relative_path: patchFile, open_paths: ["patch-alias.txt", "inspection.txt"] };
      expect((await request.post("/api/mobile/repository-inspect", { data: identityRequest })).status()).toBe(401);
      const identities = await request.post("/api/mobile/repository-inspect", { headers: { Authorization: `Bearer ${token}` }, data: identityRequest });
      expect(await identities.json()).toEqual(["patch-alias.txt"]);
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
        await page.getByText("Copilot settings", { exact: true }).click();
        await page.locator("#repositoryCopilotModel").selectOption(JSON.stringify([copilotProvider.id, "independent-copilot-model"]));
        await page.getByLabel("Include file context").uncheck();
        await page.getByLabel("Copilot temperature").fill("0.5");
        await page.getByLabel("Copilot max tokens").fill("512");
        await page.getByLabel("Additional Copilot instructions").fill("Use concise explanations.");
        await page.getByLabel("Copilot question").fill("Explain without the file");
        await page.getByRole("button", { name: "Ask Copilot", exact: true }).click();
        await expect(page.locator("#status")).toHaveText("Copilot reply ready");
        const settingsRequest = await (await request.get(process.env.ARUVI_TEST_MODEL_URL.replace(/\/v1\/?$/, "/copilot/last"))).json();
        expect(settingsRequest.model).toBe("independent-copilot-model");
        expect(await page.locator("#modelInput").inputValue()).toBe("browser-test-model");
        await call("aruvi_models", "update_model_definition", { id: copilotModel.id, enabled: false });
        await page.getByRole("button", { name: "Refresh Copilot models" }).click();
        await expect(page.getByRole("option", { name: "Previously selected model is unavailable" })).toHaveAttribute("disabled", "");
        await page.getByLabel("Copilot question").fill("Must not route to the connection model");
        await page.getByRole("button", { name: "Ask Copilot", exact: true }).click();
        await expect(page.locator("#status")).toHaveText("Selected Copilot model is unavailable; choose another model");
        await call("aruvi_models", "update_model_definition", { id: copilotModel.id, enabled: true });
        await page.getByRole("button", { name: "Refresh Copilot models" }).click();
        await expect(page.getByRole("option", { name: new RegExp("independent-copilot-model") })).not.toHaveAttribute("disabled", "");
        expect(settingsRequest.temperature).toBe(0.5);
        expect(settingsRequest.max_tokens).toBe(512);
        expect(JSON.stringify(settingsRequest.messages)).toContain("Use concise explanations.");
        expect(JSON.stringify(settingsRequest.messages)).not.toContain("saved from browser");
        expect(JSON.stringify(settingsRequest.messages)).not.toContain("inspection.txt");
        await page.getByLabel("Include file context").check();
        await page.getByLabel("Copilot temperature").fill("0.2");
        await page.getByLabel("Copilot max tokens").fill("4096");
        await page.getByLabel("Additional Copilot instructions").fill("");
        await page.locator("#repositoryCopilotModel").selectOption("");
        await page.getByText("Copilot settings", { exact: true }).click();
        await page.getByRole("button", { name: "Use code editor" }).click();
        await expect(page.locator("#codeEditorStatus")).toHaveText("Code editor active", { timeout: 30000 });
        const copilotEditorInput = page.locator("#repositoryCodeEditor textarea.inputarea");
        await copilotEditorInput.focus();
        await page.keyboard.press("ControlOrMeta+A");
        await page.keyboard.insertText("unsaved working copy\n");
        await expect(page.getByRole("button", { name: "Propose changes" })).toBeDisabled();
        await page.getByLabel("Copilot question").fill("Explain my unsaved edits");
        const unsavedResponse = page.waitForResponse((response) => response.url().endsWith("/api/mobile/repository-file/copilot") && response.request().method() === "POST");
        await page.getByRole("button", { name: "Ask about unsaved file" }).click();
        expect((await unsavedResponse).ok()).toBeTruthy();
        await expect(page.locator("#status")).toHaveText("Copilot reply ready");
        const unsaved = await (await request.get(process.env.ARUVI_TEST_MODEL_URL.replace(/\/v1\/?$/, "/copilot/last"))).json();
        expect(JSON.stringify(unsaved.messages)).toContain("unsaved working copy");
        expect(JSON.stringify(unsaved.messages)).toContain("Unsaved file data");
        expect((await call("aruvi_repositories", "read_repository_file", { repository_id: repositoryId, relative_path: "inspection.txt" })).content).toContain("saved from browser");
        await page.getByRole("button", { name: "Discard file edits" }).click();
        await page.getByRole("button", { name: "Use plain text editor" }).click();
        await page.getByLabel("Copilot question").fill("Explain this file");
        await page.getByRole("button", { name: "Ask about saved file" }).click();
        await expect(page.locator("#repositoryCopilotResult")).toContainText("Container provider reply");
        const captured = await (await request.get(process.env.ARUVI_TEST_MODEL_URL.replace(/\/v1\/?$/, "/copilot/last"))).json();
        expect(JSON.stringify(captured.messages)).not.toContain("unsaved working copy");
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
        await page.getByRole("button", { name: "Refresh file tree" }).click();
        await page.locator("#repositoryTree").getByRole("button", { name: "patch-alias.txt", exact: true }).click();
        await expect(page.getByLabel("Selected file contents")).toHaveValue("alpha\nmiddle\nbeta\n");
        await page.getByLabel("Selected file contents").fill("unsaved patch target\n");
        await page.locator("#repositoryTree").getByRole("button", { name: "inspection.txt", exact: true }).click();
        await page.getByLabel("Stream Copilot response").check();
        await page.getByLabel("Copilot question").fill("Propose fixture changes");
        await page.getByRole("button", { name: "Propose changes" }).click();
        await expect(page.locator("#repositoryPatchReview")).toContainText("Two fixture changes");
        await page.getByRole("button", { name: "Apply reviewed changes" }).click();
        await expect(page.locator("#status")).toContainText("Save or discard edits to all files in the patch first");
        expect((await call("aruvi_repositories", "read_repository_file", { repository_id: repositoryId, relative_path: patchFile })).content).toBe("alpha\nmiddle\nbeta\n");
        await page.locator("#repositoryFileTabs").getByRole("button", { name: "patch-alias.txt • unsaved" }).click();
        await expect(page.getByLabel("Selected file contents")).toHaveValue("unsaved patch target\n");
        await page.getByRole("button", { name: "Discard file edits" }).click();
        await page.locator("#repositoryTree").getByRole("button", { name: "inspection.txt", exact: true }).click();
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
        await expect(page.locator("#repositoryFileTabs")).not.toContainText("patch-alias.txt");
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
        const forbiddenUnsavedPatch = await request.post("/api/mobile/repository-file/copilot", { headers: { Authorization: `Bearer ${token}` }, data: { ...payload, working_content: "unsaved", patch_mode: true } });
        expect(forbiddenUnsavedPatch.status()).toBe(400);
        const invalidSettings = await request.post("/api/mobile/repository-file/copilot", { headers: { Authorization: `Bearer ${token}` }, data: { ...payload, context_budget: 12001 } });
        expect(invalidSettings.status()).toBe(400);
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
        expect((await call("aruvi_models", "run_file_copilot", { ...payload, working_content: "MCP unsaved fixture" })).content).toBe("Container provider reply");
        const mcpWorking = await (await request.get(process.env.ARUVI_TEST_MODEL_URL.replace(/\/v1\/?$/, "/copilot/last"))).json();
        expect(JSON.stringify(mcpWorking.messages)).toContain("MCP unsaved fixture");
        await call("aruvi_models", "run_file_copilot", { ...payload, context_budget: 5, temperature: 0.5, max_tokens: 512 });
        const customBudgetRequest = await (await request.get(process.env.ARUVI_TEST_MODEL_URL.replace(/\/v1\/?$/, "/copilot/last"))).json();
        expect(JSON.stringify(customBudgetRequest.messages)).toContain("truncated for model context");
        expect(JSON.stringify(customBudgetRequest.messages)).not.toContain("saved from browser");
        expect(customBudgetRequest.temperature).toBe(0.5); expect(customBudgetRequest.max_tokens).toBe(512);
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
        const generalProposal = JSON.parse((await call("aruvi_models", "run_general_copilot", { provider_id: copilotProvider.id, model_name: "browser-test-model", messages: [{ role: "user", content: "Propose fixture changes" }], patch_mode: true })).content);
        expect(generalProposal.patches).toHaveLength(2);

        // Return to a workspace without an open file and exercise the same
        // reviewed patch flow, including a concurrent edit after preview.
        while (await page.locator('#repositoryFileTabs button[aria-label^="Close "]').count()) {
          await page.locator('#repositoryFileTabs button[aria-label^="Close "]').first().click();
        }
        await call("aruvi_repositories", "write_repository_file", { repository_id: repositoryId, relative_path: patchFile, content: "alpha\nmiddle\nbeta\n" });
        await call("aruvi_repositories", "write_repository_file", { repository_id: repositoryId, relative_path: "proposal-created.txt", content: "" });
        await page.getByLabel("Copilot question").fill("Propose fixture changes without a file");
        await page.getByRole("button", { name: "Propose changes" }).click();
        await expect(page.locator("#repositoryPatchReview")).toContainText("Two fixture changes");
        await call("aruvi_repositories", "write_repository_file", { repository_id: repositoryId, relative_path: patchFile, content: "concurrent no-file edit\n" });
        await page.getByRole("button", { name: "Apply reviewed changes" }).click();
        await expect(page.locator("#status")).toContainText("Applied 0 of 2 patches; stopped");
        expect((await call("aruvi_repositories", "read_repository_file", { repository_id: repositoryId, relative_path: patchFile })).content).toBe("concurrent no-file edit\n");
        await call("aruvi_repositories", "write_repository_file", { repository_id: repositoryId, relative_path: patchFile, content: "alpha\nmiddle\nbeta\n" });
        await page.getByLabel("Stream Copilot response").check();
        await page.getByLabel("Copilot question").fill("Propose fixture changes again without a file");
        await page.getByRole("button", { name: "Propose changes" }).click();
        await expect(page.locator("#repositoryPatchReview")).toContainText("Two fixture changes");
        await page.getByRole("button", { name: "Apply reviewed changes" }).click();
        await expect(page.locator("#status")).toHaveText("Applied 2 reviewed patch(es)");
        await page.getByLabel("Stream Copilot response").uncheck();
        expect((await call("aruvi_repositories", "read_repository_file", { repository_id: repositoryId, relative_path: patchFile })).content).toBe("alpha\nreviewed\nbeta\n");
        await page.locator("#repositoryTree").getByRole("button", { name: "inspection.txt", exact: true }).click();
        await expect(page.getByLabel("Selected file contents")).toHaveValue("saved from browser\n");
      }
      await page.getByLabel("Selected file contents").fill("unsaved browser edit\n");
      await expect(page.getByRole("button", { name: "Ask about unsaved file" })).toBeEnabled();
      await page.locator("#repositoryTree").getByRole("button", { name: "windows.txt", exact: true }).click();
      await expect(page.getByLabel("Selected file contents")).toHaveValue("windows line\n");
      await page.getByLabel("Selected file contents").fill("second unsaved edit\n");
      await expect(page.locator("#repositoryFileTabs")).toContainText("inspection.txt • unsaved");
      await page.locator("#repositoryFileTabs").getByRole("button", { name: "inspection.txt • unsaved" }).click();
      await expect(page.getByLabel("Selected file contents")).toHaveValue("unsaved browser edit\n");
      await page.locator("#repositoryFileTabs").getByRole("button", { name: "windows.txt • unsaved" }).click();
      await expect(page.getByLabel("Selected file contents")).toHaveValue("second unsaved edit\n");
      await row.getByRole("button", { name: "Browse workspace" }).click();
      await expect(page.locator("#status")).toContainText("Save or discard file edits before opening another workspace");
      await expect(page.getByLabel("Selected file contents")).toHaveValue("second unsaved edit\n");
      page.once("dialog", (dialog) => dialog.dismiss());
      await page.getByRole("button", { name: "Close windows.txt" }).click();
      await expect(page.getByLabel("Selected file contents")).toHaveValue("second unsaved edit\n");
      await page.getByRole("button", { name: "Discard file edits" }).click();
      await page.locator("#repositoryFileTabs").getByRole("button", { name: "inspection.txt • unsaved" }).click();
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
      await page.locator("#repositoryTree").getByRole("button", { name: "windows.txt", exact: true }).click();
      await expect(page.getByLabel("Selected file contents")).toHaveValue("windows line\n");
      await page.getByLabel("Selected file contents").fill("updated windows\n");
      await page.getByRole("button", { name: "Save file", exact: true }).click();
      await expect(page.locator("#status")).toHaveText("File saved");
      expect((await call("aruvi_repositories", "read_repository_file", { repository_id: repositoryId, relative_path: "windows.txt" })).content).toBe("updated windows\r\n");
      await page.locator("#repositoryTree").getByRole("button", { name: "mixed.txt", exact: true }).click();
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
  await expect(page.locator("#workflowAutomation")).toContainText("syntax checks alone do not verify behavior");
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

test("external planner defaults save atomically and reject stale MCP edits", async ({ page, request }) => {
  test.skip(!token || !mcpToken || process.env.ARUVI_TEST_INSPECT !== "1", "Requires disposable remote fixture");
  const url = "/api/mobile/planner-defaults", headers = { Authorization: `Bearer ${token}` };
  expect((await request.get(url)).status()).toBe(401);
  expect((await request.post(url, { data: { expected: { provider: null, model: null }, provider: "x", model: "y" } })).status()).toBe(401);
  await page.goto("/remote"); await page.locator("#tokenInput").fill(token); await page.locator("#saveButton").click();
  const loaded = page.waitForResponse((response) => response.url().endsWith(url) && response.request().method() === "GET");
  await page.getByRole("button", { name: "Routing", exact: true }).click(); await loaded;
  await page.getByLabel("Default planner provider ID").fill("provider-a");
  await page.getByLabel("Default planner model name").fill("model-a");
  await page.getByRole("button", { name: "Save planner defaults" }).click();
  await expect(page.locator("#status")).toHaveText("Planner defaults saved");
  expect(await (await request.get(url, { headers })).json()).toEqual({ provider: "provider-a", model: "model-a", product: "" });
  const mcp = await request.post("/api/mcp", { headers: { Authorization: `Bearer ${mcpToken}`, Accept: "application/json, text/event-stream" }, data: { jsonrpc: "2.0", id: 1, method: "tools/call", params: { name: "aruvi_settings", arguments: { action: "set_setting", arguments: { key: "planner.default_model_name", value: "model-external" } } } } });
  expect((await mcp.json()).result.isError).not.toBeTruthy();
  await page.getByLabel("Default planner provider ID").fill("provider-unsaved");
  const conflict = page.waitForResponse((response) => response.url().endsWith(url) && response.request().method() === "POST");
  await page.getByRole("button", { name: "Save planner defaults" }).click();
  expect((await conflict).status()).toBe(409);
  expect(await (await request.get(url, { headers })).json()).toEqual({ provider: "provider-a", model: "model-external", product: "" });
  await expect(page.getByLabel("Default planner provider ID")).toHaveValue("provider-unsaved");
  await page.getByRole("button", { name: "Refresh defaults" }).click();
  await expect(page.getByLabel("Default planner model name")).toHaveValue("model-external");
});

test("external planner webhook requires configured signature verification", async ({ request }) => {
  test.skip(!mcpToken || process.env.ARUVI_TEST_INSPECT !== "1", "Requires disposable remote fixture");
  const url = "/webhooks/twilio/whatsapp", form = { Body: "hello", From: "whatsapp:+15550000000" };
  expect((await request.post(url, { form })).status()).toBe(401);
  for (const [key, value] of [["twilio.auth_token", "fixture-secret"], ["twilio.webhook_base_url", "https://example.test"]]) {
    const response = await request.post("/api/mcp", { headers: { Authorization: `Bearer ${mcpToken}`, Accept: "application/json, text/event-stream" }, data: { jsonrpc: "2.0", id: 1, method: "tools/call", params: { name: "aruvi_settings", arguments: { action: "set_setting", arguments: { key, value } } } } });
    expect((await response.json()).result.isError).not.toBeTruthy();
  }
  expect((await request.post(url, { form, headers: { "X-Twilio-Signature": "invalid" } })).status()).toBe(401);
  const signature = createHmac("sha1", "fixture-secret").update("https://example.test" + url + "Body" + form.Body + "From" + form.From).digest("base64");
  expect((await request.post(url, { form, headers: { "X-Twilio-Signature": signature } })).status()).toBe(200);
});

test("signed external channel turns use the configured planner model", async ({ page, request }) => {
  test.skip(!token || !mcpToken || !process.env.ARUVI_TEST_MODEL_URL || process.env.ARUVI_TEST_INSPECT !== "1", "Requires disposable model fixture");
  async function call(namespace, action, args) {
    const response = await request.post("/api/mcp", { headers: { Authorization: `Bearer ${mcpToken}`, Accept: "application/json, text/event-stream" }, data: { jsonrpc: "2.0", id: 1, method: "tools/call", params: { name: namespace, arguments: { action, arguments: args } } } });
    const result = (await response.json()).result; expect(result.isError).not.toBeTruthy(); return result.structuredContent.result;
  }
  const provider = await call("aruvi_models", "create_provider", { name: "Channel planner fixture", base_url: process.env.ARUVI_TEST_MODEL_URL.replace(/\/v1$/, "/channel/v1") });
  const product = await call("aruvi_catalog", "create_product", { name: "External channel fixture product" });
  await call("aruvi_models", "create_model_definition", { provider_id: provider.id, name: "channel-fixture-model" });
  await call("aruvi_settings", "set_setting", { key: "twilio.auth_token", value: "fixture-secret" });
  await call("aruvi_settings", "set_setting", { key: "twilio.webhook_base_url", value: "https://example.test" });
  await page.goto("/remote"); await page.locator("#tokenInput").fill(token); await page.locator("#saveButton").click();
  const loaded = page.waitForResponse((response) => response.url().endsWith("/api/mobile/planner-defaults") && response.request().method() === "GET");
  await page.getByRole("button", { name: "Routing", exact: true }).click(); await loaded;
  await page.getByLabel("Default planner provider ID").fill(provider.id);
  await page.getByLabel("Default planner model name").fill("channel-fixture-model");
  await page.getByLabel("External channel product").selectOption(product.id);
  await page.getByRole("button", { name: "Save planner defaults" }).click();
  await expect(page.locator("#status")).toHaveText("Planner defaults saved");
  for (const [path, form] of [
    ["/webhooks/twilio/whatsapp", { Body: "Please help me plan a garden", From: "whatsapp:+15551111111" }],
    ["/webhooks/twilio/voice/gather", { CallSid: "fixture-call", From: "+15552222222", SpeechResult: "Please help me plan a kitchen" }],
  ]) {
    const input = "https://example.test" + path + Object.keys(form).sort().map((key) => key + form[key]).join("");
    const signature = createHmac("sha1", "fixture-secret").update(input).digest("base64");
    const response = await request.post(path, { form, headers: { "X-Twilio-Signature": signature } });
    expect(response.status()).toBe(200);
    expect(await response.text()).toContain("Channel planner fixture reply");
  }
  const otherProduct = await call("aruvi_catalog", "create_product", { name: "Other channel fixture product" });
  await call("aruvi_settings", "set_setting", { key: "planner.channel_product_id", value: otherProduct.id });
  const path = "/webhooks/twilio/whatsapp", form = { Body: "Continue the garden plan", From: "whatsapp:+15551111111" };
  const signature = createHmac("sha1", "fixture-secret").update("https://example.test" + path + "Body" + form.Body + "From" + form.From).digest("base64");
  expect(await (await request.post(path, { form, headers: { "X-Twilio-Signature": signature } })).text()).toContain("Channel planner fixture reply");
  const calls = await (await request.get("http://127.0.0.1:19876/channel/requests")).json();
  expect(calls).toHaveLength(3);
  expect(calls.every((entry) => entry.model === "channel-fixture-model")).toBe(true);
  expect(JSON.stringify(calls[0].messages)).toContain("plan a garden");
  expect(JSON.stringify(calls[1].messages)).toContain("plan a kitchen");
  expect(JSON.stringify(calls[2].messages)).toContain("External channel fixture product");
  expect(JSON.stringify(calls[2].messages)).not.toContain("Other channel fixture product");
});

test("Docker Desktop host address is an explicit provider draft", async ({ page, request }) => {
  test.skip(process.env.ARUVI_TEST_INSPECT !== "1", "Requires a disposable remote fixture");
  const headers = { Authorization: `Bearer ${token}` };
  const name = `Docker host fixture ${Date.now()}`;
  const created = await request.post("/api/mobile/providers", { headers, data: { name, base_url: "http://localhost:1234/v1" } });
  expect(created.ok(), await created.text()).toBeTruthy();
  const provider = await created.json();
  try {
    await page.goto("/remote");
    await page.locator("#tokenInput").fill(token);
    await page.locator("#saveButton").click();
    await page.getByRole("button", { name: "Models", exact: true }).click();
    const input = page.getByLabel("API base URL", { exact: true });
    await input.fill("http://127.0.0.1:4321/v1");
    await page.locator("#providerDockerHost").click();
    await expect(input).toHaveValue("http://host.docker.internal:4321/v1");
    await input.fill("http://[::1]:9876/custom/v1");
    await page.locator("#providerDockerHost").click();
    await expect(input).toHaveValue("http://host.docker.internal:9876/custom/v1");
    await input.fill("http://localhost:1234/v1?secret=synthetic");
    await page.locator("#providerDockerHost").click();
    await expect(input).toHaveValue("http://localhost:1234/v1?secret=synthetic");
    await expect(page.locator("#status")).toContainText("without credentials, query or fragment");
    await input.fill("https://remote.example/v1");
    await page.locator("#providerDockerHost").click();
    await expect(input).toHaveValue("https://remote.example/v1");
    await expect(page.locator("#status")).toContainText("loopback");
    const card = page.locator("#providerList .tool-card").filter({ hasText: name });
    await card.getByText("Manage provider", { exact: true }).click();
    await card.getByRole("button", { name: "Use Docker Desktop host address" }).click();
    await expect(card.getByLabel("Edit provider URL " + name)).toHaveValue("http://host.docker.internal:1234/v1");
    let providers = await (await request.get("/api/mobile/providers", { headers })).json();
    expect(providers.find(item => item.id === provider.id).base_url).toBe("http://localhost:1234/v1");
    await card.getByRole("button", { name: "Save provider" }).click();
    await expect(page.locator("#status")).toHaveText("Provider saved");
    providers = await (await request.get("/api/mobile/providers", { headers })).json();
    expect(providers.find(item => item.id === provider.id).base_url).toBe("http://host.docker.internal:1234/v1");
  } finally {
    const response = await request.post("/api/mcp", { headers: { Authorization: `Bearer ${mcpToken}` }, data: { jsonrpc: "2.0", id: 1, method: "tools/call", params: { name: "aruvi_models", arguments: { action: "delete_provider", arguments: { id: provider.id } } } } });
    expect((await response.json()).result.isError).not.toBeTruthy();
  }
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
    for (const [preset, tags] of [
      ["deepseek-flash", "coding,planning,analysis"],
      ["deepseek-v4-pro", "coding,planning,analysis"],
      ["deepseek-voice-transcription", "speech_to_text,transcription,audio"],
      ["whisper-base.en", "speech_to_text,transcription,audio"],
    ]) {
      await page.getByLabel("Context window (optional)").fill("1024");
      await page.getByLabel("Model preset (optional)").selectOption(preset);
      await expect(page.getByLabel("Provider for model")).toHaveValue(providerId);
      await expect(page.getByLabel("Model name", { exact: true })).toHaveValue(preset);
      await expect(page.getByLabel("Context window (optional)")).toHaveValue("");
      await expect(page.getByLabel("Capability tags (comma-separated)")).toHaveValue(tags);
      await expect(page.getByLabel("Notes", { exact: true })).not.toHaveValue("");
    }
    if (process.env.ARUVI_TEST_MODEL_URL) {
      let releaseOld, startedOld;
      const oldStarted = new Promise((resolve) => { startedOld = resolve; });
      const oldGate = new Promise((resolve) => { releaseOld = resolve; });
      let discoveryRequests = 0;
      await page.route("**/api/mobile/providers/discover", async (route) => {
        discoveryRequests++;
        const old = discoveryRequests === 1;
        if (old) { startedOld(); await oldGate; }
        await route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify({ models: [{ id: old ? "stale-model" : "fresh-model", metadata: { task: "transcription" } }], next_cursor: null }) });
      });
      await page.getByRole("button", { name: "Discover available models" }).click();
      await oldStarted;
      await page.locator("#modelProvider").evaluate((select) => select.add(new Option("Other fixture", "other-fixture")));
      await page.getByLabel("Provider for model").selectOption("other-fixture");
      await page.getByLabel("Provider for model").selectOption(providerId);
      await page.getByRole("button", { name: "Discover available models" }).click();
      await expect(page.locator("#discoveredModels")).toContainText("fresh-model");
      await expect(page.locator('#discoveredModels option[value="fresh-model"]')).toHaveText("fresh-model — provider task: transcription");
      await page.locator("#discoveredModels").selectOption("fresh-model");
      await expect(page.getByLabel("Model name", { exact: true })).toHaveValue("fresh-model");
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
    await page.getByLabel("Context window (optional)").fill("4096");
    await page.getByLabel("Capability tags (comma-separated)").fill("coding, testing");
    await page.getByLabel("Notes", { exact: true }).fill("Initial notes");
    const modelResponse = page.waitForResponse((r) => r.url().endsWith("/api/mobile/models") && r.request().method() === "POST");
    await page.getByRole("button", { name: "Add model", exact: true }).click();
    const model = await modelResponse;
    expect(model.ok()).toBeTruthy();
    const createdModel = await model.json();
    expect(createdModel).toMatchObject({ context_window: 4096, capability_tags: ["coding", "testing"], notes: "Initial notes" });
    modelId = createdModel.id;
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
    await modelCard.getByLabel("Capability tags browser-test-model").fill("planning, analysis");
    await modelCard.getByLabel("Model notes browser-test-model").fill("Updated notes");
    await modelCard.getByRole("button", { name: "Save model" }).click();
    await expect(page.locator("#status")).toHaveText("Model saved");
    let models = await (await request.get("/api/mobile/models", { headers: { Authorization: `Bearer ${token}` } })).json();
    expect(models.find((item) => item.id === modelId)).toMatchObject({ name: "renamed-browser-model", context_window: 8192, capability_tags: ["planning", "analysis"], notes: "Updated notes" });
    modelCard = page.locator("#modelList .tool-card").filter({ hasText: "renamed-browser-model" }).filter({ hasText: name });
    await modelCard.getByText("Manage model", { exact: true }).click();
    await modelCard.getByLabel("Edit model name renamed-browser-model").fill("unsaved-model-name");
    await call("aruvi_models", "update_model_definition", { id: modelId, capability_tags: ["concurrent"] });
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

test("configured reverse-proxy origin requires matching host and MCP token", async ({ request }) => {
  test.skip(!mcpToken || process.env.ARUVI_TEST_INSPECT !== "1", "Requires disposable origin fixture");
  const body = { jsonrpc: "2.0", id: 1, method: "ping" };
  const send = (host, origin, tokenValue) => request.post("/api/mcp", {
    headers: { Host: host, Origin: origin, ...(tokenValue ? { Authorization: `Bearer ${tokenValue}` } : {}) }, data: body,
  });
  expect((await send("studio.example", "https://studio.example", mcpToken)).status()).toBe(200);
  expect((await send("studio.example", "https://studio.example", null)).status()).toBe(401);
  expect((await send("studio.example", "http://studio.example", mcpToken)).status()).toBe(403);
  expect((await send("other.example", "https://studio.example", mcpToken)).status()).toBe(403);
  expect((await send("other.example", "https://other.example", mcpToken)).status()).toBe(403);
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
  await expect(page.locator("#modelCallSummary")).toHaveText("2 sessions · 3 calls · 3 loaded · filter applies to loaded calls");
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

test("activity loads older model calls and refresh returns to latest", async ({ page }) => {
  test.skip(!token, "Set ARUVI_MOBILE_API_TOKEN for a running remote server");
  const first = Array.from({ length: 100 }, (_, i) => ({ id: `call-${String(999 - i).padStart(3, "0")}`, source_kind: "desktop_chat", source_label: "Current", model_name: "fixture", status: "completed", created_at: "2026-01-02 00:00:00" }));
  let olderRequests = 0;
  await page.route("**/api/mobile/model-calls?*", (route) => {
    const params = new URL(route.request().url()).searchParams;
    if (params.has("before_id")) {
      expect(params.get("before_id")).toBe("call-900");
      expect(params.get("before_created_at")).toBe("2026-01-02 00:00:00");
      olderRequests++;
      if (olderRequests === 1) return route.fulfill({ status: 500, body: "fixture failure" });
      return route.fulfill({ json: [{ ...first[0], id: "old-call", source_label: "Older call", created_at: "2026-01-01 00:00:00" }] });
    }
    return route.fulfill({ json: first });
  });
  await page.goto("/remote");
  await page.locator("#tokenInput").fill(token);
  await page.locator("#saveButton").click();
  await page.getByRole("button", { name: "Activity", exact: true }).click();
  await page.getByRole("button", { name: "Load older calls" }).click();
  await expect(page.locator("#status")).toContainText("Could not load older calls");
  await expect(page.locator("#modelCallSummary")).toContainText("100 loaded · auto-refresh paused");
  await page.getByRole("button", { name: "Load older calls" }).click();
  await expect(page.locator("#modelCallSummary")).toContainText("101 loaded · auto-refresh paused");
  await expect(page.locator("#modelCallList")).toContainText("Older call");
  await expect(page.getByRole("button", { name: "Load older calls" })).toBeHidden();
  expect(olderRequests).toBe(2);
  await page.locator("#refreshModelCallsButton").click();
  await expect(page.locator("#modelCallSummary")).toContainText("100 loaded · filter applies to loaded calls");
  await expect(page.locator("#modelCallList")).not.toContainText("Older call");
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
    const exportItems = await mcp("aruvi_work_items", "list_product_export_work_items", { product_id: product.id });
    expect(exportItems.map((item) => item.id).sort()).toEqual([parent.id, scoped.id, child.id].sort());
    expect(exportItems.find((item) => item.id === child.id).parent_work_item_id).toBe(parent.id);
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
    await page.getByLabel("Product area purpose (optional)").fill("Initial purpose");
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
    expect(area.product_area.purpose).toBe("Initial purpose");
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
    await feature.getByLabel("Edit acceptance criteria for Browser feature").fill("It should work");
    await feature.getByLabel("Edit technical notes for Browser feature").fill("Use a bounded queue");
    await feature.getByRole("button", { name: "Save details" }).click();
    await expect(page.locator("#productTree")).toContainText("Renamed feature");
    const current = await (await request.get(`/api/mobile/products/${product.id}/tree`, { headers: { Authorization: `Bearer ${token}` } })).json();
    const savedFeature = current.product_areas[0].features[0].children[0].capability;
    expect(savedFeature.description).toBe("Browser-edited description");
    expect(savedFeature.acceptance_criteria).toBe("It should work");
    expect(savedFeature.technical_notes).toBe("Use a bounded queue");
    const guarded = {node_type:"capability",id:savedFeature.id,expected_name:savedFeature.name,expected_description:savedFeature.description,name:savedFeature.name,description:savedFeature.description,expected_acceptance_criteria:savedFeature.acceptance_criteria,acceptance_criteria:"Stale",expected_technical_notes:savedFeature.technical_notes,technical_notes:"Stale"};
    await catalog("update_capability", {id:savedFeature.id, technical_notes:"Changed through MCP"});
    expect((await request.post(`/api/mobile/products/${product.id}/hierarchy/update`, {headers:{Authorization:`Bearer ${token}`},data:guarded})).status()).toBe(409);
    expect((await request.post(`/api/mobile/products/${other.id}/hierarchy/update`, {headers:{Authorization:`Bearer ${token}`},data:guarded})).status()).toBe(409);
    expect((await request.post(`/api/mobile/products/${product.id}/hierarchy/update`, {headers:{Authorization:`Bearer ${token}`},data:{...guarded,expected_technical_notes:null}})).status()).toBe(400);
    await page.locator("#productTree summary").filter({ hasText: "Browser area" }).click();
    const areaElement = page.locator("#productTree > details").first();
    const areaEdit = areaElement.locator(":scope > details > summary").filter({ hasText: "Edit details" }).locator("..");
    await areaEdit.locator(":scope > summary").click();
    await areaElement.getByLabel("Edit name for Browser area").fill("Unsaved area edit");
    await areaElement.getByLabel("Edit purpose for Browser area").fill("Unsaved purpose");
    await catalog("update_product_area", { id: area.product_area.id, purpose: "Concurrent purpose" });
    const conflict = page.waitForResponse((response) => response.url().endsWith("/hierarchy/update") && response.request().method() === "POST");
    await areaEdit.getByRole("button", { name: "Save details" }).click();
    expect((await conflict).status()).toBe(409);
    await expect(areaElement.getByLabel("Edit name for Browser area")).toHaveValue("Unsaved area edit");
    const latest = await (await request.get(`/api/mobile/products/${product.id}/tree`, { headers: { Authorization: `Bearer ${token}` } })).json();
    expect(latest.product_areas[0].product_area.purpose).toBe("Concurrent purpose");
    expect(latest.product_areas[0].product_area.name).toBe("Browser area");
    await page.locator('#refreshProductsButton').click();
    await page.getByRole('button',{name:'Load product tree'}).click();
    await page.locator('#productTree > details > summary').first().click();
    const refreshedArea = page.locator('#productTree > details').first();
    await refreshedArea.locator(':scope > details > summary').filter({hasText:'Edit details'}).click();
    await refreshedArea.getByLabel('Edit purpose for Browser area').fill('Updated purpose');
    await refreshedArea.getByRole('button',{name:'Save details'}).click();
    await expect(page.locator('#status')).toHaveText('Hierarchy details saved');
    const savedArea = await (await request.get(`/api/mobile/products/${product.id}/tree`, {headers:{Authorization:`Bearer ${token}`}})).json();
    expect(savedArea.product_areas[0].product_area.purpose).toBe('Updated purpose');
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

test("browser manages example catalog with conflict protection", async ({ page, request }) => {
  test.skip(!token || !mcpToken || process.env.ARUVI_TEST_INSPECT !== "1", "Requires disposable remote fixture");
  const url = "/api/mobile/catalog-settings", headers = { Authorization: `Bearer ${token}` };
  expect((await request.get(url)).status()).toBe(401);
  expect((await request.post("/api/mobile/catalog-seed-examples")).status()).toBe(401);
  await page.goto("/remote"); await page.locator("#tokenInput").fill(token); await page.locator("#saveButton").click();
  await page.getByRole("button", { name: "Routing", exact: true }).click();
  const toggle = page.getByLabel("Hide example products", { exact: true });
  await expect(toggle).toBeEnabled();
  const initial = await (await request.get(url, { headers })).json();
  const firstSave = page.waitForResponse((response) => response.url().endsWith(url) && response.request().method() === "POST");
  await toggle.setChecked(!initial.enabled);
  expect((await firstSave).status()).toBe(200);
  await expect(toggle).toBeEnabled();
  await expect.poll(async () => (await (await request.get(url, { headers })).json()).enabled).toBe(!initial.enabled);
  const mcp = await request.post("/api/mcp", { headers: { Authorization: `Bearer ${mcpToken}`, Accept: "application/json, text/event-stream" }, data: { jsonrpc: "2.0", id: 1, method: "tools/call", params: { name: "aruvi_settings", arguments: { action: "set_setting", arguments: { key: "catalog.hide_example_products", value: String(initial.enabled) } } } } });
  expect((await mcp.json()).result.isError).not.toBeTruthy();
  const conflict = page.waitForResponse((response) => response.url().endsWith(url) && response.request().method() === "POST");
  // The app immediately restores the old value on 409, so setChecked's
  // post-click state assertion races the intentional rollback.
  await toggle.click();
  expect((await conflict).status()).toBe(409);
  await expect(toggle).toHaveJSProperty("checked", !initial.enabled);
  await page.getByRole("button", { name: "Refresh catalog settings" }).click();
  await expect(toggle).toHaveJSProperty("checked", initial.enabled);
  page.once("dialog", (dialog) => dialog.accept());
  const seeded = page.waitForResponse((response) => response.url().endsWith("/api/mobile/catalog-seed-examples"));
  await page.getByRole("button", { name: "Seed / repair examples" }).click();
  expect((await seeded).status()).toBe(200);
});

test("planner contact asks before sending and displays routed outcomes", async ({ page, request }) => {
  test.skip(!token, "Set ARUVI_MOBILE_API_TOKEN for an isolated server");
  const rejected = await request.post("/api/mobile/planner/contact", { data: { to: "+15550000000", content: "fixture" } });
  expect(rejected.status()).toBe(401);
  const invalid = await request.post("/api/mobile/planner/contact", { headers: { Authorization: `Bearer ${token}` }, data: { to: "+15550000000", content: "fixture", preferred_channel: "invalid" } });
  expect(invalid.status()).toBe(400);
  const unconfigured = await request.post("/api/mobile/planner/contact", { headers: { Authorization: `Bearer ${token}` }, data: { to: "+15550000000", content: "fixture" } });
  expect(unconfigured.status()).toBe(400);
  expect(await unconfigured.text()).toContain("not configured");
  await page.addInitScript((value) => localStorage.setItem("aruvi.remote.token", value), token);
  const calls = [];
  await page.route("**/api/mobile/planner/contact", async (route) => {
    calls.push(route.request().postDataJSON());
    await route.fulfill({ json: { status: "blocked", channel: "voice", reason: "quiet hours" } });
  });
  await page.goto("/remote");
  await page.getByRole("button", { name: "Routing", exact: true }).click();
  await page.locator("#plannerContactTo").fill("+15550000000");
  await page.locator("#plannerContactMessage").fill("Fixture opening");
  await page.locator("#plannerContactChannel").selectOption("voice");
  page.once("dialog", (dialog) => dialog.dismiss());
  await page.locator("#plannerContactSend").click();
  expect(calls).toHaveLength(0);
  page.once("dialog", (dialog) => dialog.accept());
  await page.locator("#plannerContactSend").click();
  await expect(page.locator("#plannerContactResult")).toHaveText("blocked · voice · quiet hours");
  expect(calls).toEqual([{ to: "+15550000000", content: "Fixture opening", preferred_channel: "voice", allow_after_hours: false }]);
  expect((await request.post("/api/mobile/channels/whatsapp/contact", { data: { to: "+15550000000", content: "fixture" } })).status()).toBe(401);
  expect((await request.post("/api/mobile/channels/voice/contact", { headers: { Authorization: `Bearer ${token}` }, data: { to: "whatsapp:+15550000000" } })).status()).toBe(400);
  expect((await request.post("/api/mobile/channels/whatsapp/contact", { headers: { Authorization: `Bearer ${token}` }, data: { to: "+15550000000", content: "" } })).status()).toBe(400);
  const directUnconfigured = await request.post("/api/mobile/channels/whatsapp/contact", { headers: { Authorization: `Bearer ${token}` }, data: { to: "+15550000000", content: "fixture" } });
  expect(directUnconfigured.status()).toBe(400);
  expect(await directUnconfigured.text()).toContain("not configured");
  const direct = [];
  await page.route("**/api/mobile/channels/*/contact", async (route) => {
    direct.push({ channel: route.request().url().split("/").at(-2), body: route.request().postDataJSON() });
    await route.fulfill({ json: { status: "submitted", channel: direct.at(-1).channel, reason: "Provider accepted the request; delivery is not confirmed. Planner policy was not applied." } });
  });
  await page.locator("#plannerContactMethod").selectOption("whatsapp");
  await expect(page.locator("#plannerContactChannel")).toBeDisabled();
  page.once("dialog", (dialog) => { expect(dialog.message()).toContain("quiet hours will not apply"); return dialog.dismiss(); });
  await page.locator("#plannerContactSend").click();
  expect(direct).toHaveLength(0);
  page.once("dialog", (dialog) => dialog.accept());
  await page.locator("#plannerContactSend").click();
  await expect(page.locator("#plannerContactResult")).toContainText("submitted · whatsapp");
  expect(direct[0]).toEqual({ channel: "whatsapp", body: { to: "+15550000000", content: "Fixture opening" } });
  await page.locator("#plannerContactMethod").selectOption("voice");
  await page.locator("#plannerContactMessage").fill("");
  page.once("dialog", (dialog) => dialog.accept());
  await page.locator("#plannerContactSend").click();
  await expect(page.locator("#plannerContactResult")).toContainText("submitted · voice");
  expect(direct[1]).toEqual({ channel: "voice", body: { to: "+15550000000", content: null } });
  await page.locator("#plannerContactMethod").selectOption("planner");
  await expect(page.locator("#plannerContactChannel")).toBeEnabled();
});

test("browser manages portfolio strategy and product dependencies", async ({ page, request }) => {
  test.skip(!token || !mcpToken, "Requires disposable authenticated fixture");
  async function call(action, args) {
    const response = await request.post('/api/mcp', {headers:{Authorization:`Bearer ${mcpToken}`}, data:{jsonrpc:'2.0',id:1,method:'tools/call',params:{name:'aruvi_catalog',arguments:{action,arguments:args}}}});
    const body = await response.json();
    if (!response.ok() || body.error || body.result?.isError) throw new Error(`MCP ${action} failed`);
    return body.result.structuredContent.result;
  }
  const suffix = Date.now();
  const a = await call('create_product', {name:`Portfolio A ${suffix}`});
  const b = await call('create_product', {name:`Portfolio B ${suffix}`});
  const areaA = await call('create_product_area', {product_id:a.id, name:'Alpha area'});
  const areaB = await call('create_product_area', {product_id:b.id, name:'Beta area'});
  const capA = await call('create_capability', {product_area_id:areaA.id, name:'Alpha capability'});
  const capB = await call('create_capability', {product_area_id:areaB.id, name:'Beta capability'});
  let node;
  try {
    await page.goto('/remote'); await page.locator('#tokenInput').fill(token); await page.locator('#saveButton').click();
    await page.getByRole('button',{name:'Products',exact:true}).click();
    await expect(page.locator('#productList')).toContainText(a.name);
    await page.locator('#portfolioSection summary').click();
    await page.getByLabel('Strategy name').fill(`Strategy ${suffix}`);
    const creation = page.waitForResponse(r => r.url().endsWith('/api/mobile/portfolio') && r.request().postDataJSON()?.action === 'create_strategy_node');
    await page.getByRole('button',{name:'Add strategy node'}).click();
    const response = await creation; expect(response.ok(), await response.text()).toBeTruthy(); node = await response.json();
    await expect(page.locator('#strategyList')).toContainText(node.name);
    await page.getByText('Edit strategy', {exact:true}).click();
    await page.getByLabel('Edit strategy name').fill(`Stale strategy ${suffix}`);
    await call('update_strategy_node', {id:node.id, name:`Concurrent strategy ${suffix}`});
    await page.getByRole('button',{name:'Save strategy'}).click();
    await expect(page.locator('#status')).toContainText('refresh before saving');
    await expect(page.getByLabel('Edit strategy name')).toHaveValue(`Stale strategy ${suffix}`);
    await page.getByRole('button',{name:'Refresh portfolio'}).click();
    await expect(page.locator('#strategyList')).toContainText(`Concurrent strategy ${suffix}`);
    await page.getByText('Edit strategy', {exact:true}).click();
    await page.getByLabel('Edit strategy name').fill(`Edited strategy ${suffix}`);
    await page.getByRole('button',{name:'Save strategy'}).click();
    await expect(page.locator('#strategyList')).toContainText(`Edited strategy ${suffix}`);
    node.name = `Edited strategy ${suffix}`;
    await page.getByLabel('Product to link').selectOption(a.id); await page.getByLabel('Strategy to link').selectOption(node.id);
    await call('link_product_to_strategy',{product_id:a.id,strategy_node_id:node.id,is_primary:false});
    await page.getByLabel('Primary strategy').check(); await page.getByRole('button',{name:'Link product'}).click();
    await expect(page.locator('#status')).toContainText('links changed; refresh');
    await page.getByRole('button',{name:'Refresh portfolio'}).click();
    await expect(page.locator('#strategyLinks')).toContainText(a.name);
    await page.getByRole('button',{name:'Link product'}).click();
    await expect(page.locator('#strategyLinks')).toContainText('primary');
    let releaseOld, markStarted;
    const oldStarted = new Promise(resolve => { markStarted = resolve; });
    const oldGate = new Promise(resolve => { releaseOld = resolve; });
    await page.route('**/api/mobile/products/*/tree', async route => {
      if (route.request().url().includes(`/products/${a.id}/`)) { markStarted(); await oldGate; }
      await route.continue();
    });
    await page.getByLabel('Dependent product').selectOption(a.id); await oldStarted;
    await page.getByLabel('Dependent product').selectOption(b.id);
    await expect(page.getByLabel('Dependent capability (optional)')).toContainText('Beta area › Beta capability');
    const oldResponse = page.waitForResponse(response => response.url().includes(`/products/${a.id}/tree`));
    releaseOld();
    await (await oldResponse).finished();
    await page.unroute('**/api/mobile/products/*/tree');
    await expect(page.getByLabel('Dependent capability (optional)')).not.toContainText('Alpha area › Alpha capability');
    await page.route('**/api/mobile/products/*/tree', route => route.fulfill({status:503, body:'fixture outage'}));
    await page.getByLabel('Dependent product').selectOption(a.id);
    await expect(page.locator('#dependencyCapabilityStatus')).toContainText('Could not load');
    await expect(page.getByRole('button',{name:'Add dependency'})).toBeDisabled();
    await page.unroute('**/api/mobile/products/*/tree');
    await page.getByLabel('Dependent product').selectOption(b.id);
    await page.getByLabel('Dependent product').selectOption(a.id); await page.getByLabel('Depends on product').selectOption(b.id);
    await expect(page.getByLabel('Dependent capability (optional)')).toContainText('Alpha area › Alpha capability');
    await expect(page.getByLabel('Target capability (optional)')).toContainText('Beta area › Beta capability');
    await page.getByLabel('Dependent capability (optional)').selectOption(capA.id);
    await page.getByLabel('Target capability (optional)').selectOption(capB.id);
    await page.getByLabel('Dependency kind').selectOption('data'); await page.getByRole('button',{name:'Add dependency'}).click();
    await expect(page.locator('#dependencyList')).toContainText(b.name);
    expect((await call('list_product_dependencies',{})).some(d => d.product_id === a.id && d.depends_on_product_id === b.id && d.capability_id === capA.id && d.depends_on_capability_id === capB.id && d.dependency_kind === 'data')).toBe(true);
    page.once('dialog', dialog => dialog.accept()); await page.getByRole('button',{name:'Delete dependency'}).click();
    await expect(page.locator('#dependencyList')).not.toContainText(b.name);
    await call('link_product_to_strategy',{product_id:a.id,strategy_node_id:node.id,is_primary:false});
    page.once('dialog', dialog => dialog.accept()); await page.getByRole('button',{name:'Unlink',exact:true}).click();
    await expect(page.locator('#status')).toContainText('links changed; refresh');
    await page.getByRole('button',{name:'Refresh portfolio'}).click();
    await expect(page.locator('#strategyLinks')).not.toContainText('primary');
    page.once('dialog', dialog => dialog.accept()); await page.getByRole('button',{name:'Unlink',exact:true}).click();
    await expect(page.locator('#strategyLinks')).not.toContainText(a.name);
    await call('create_strategy_node',{parent_node_id:node.id,node_kind:'domain',name:`Concurrent child ${suffix}`});
    page.once('dialog', dialog => dialog.accept()); await page.getByRole('button',{name:'Delete strategy'}).click();
    await expect(page.locator('#status')).toContainText('refresh before deleting');
    await expect(page.locator('#strategyList')).toContainText(node.name);
    await page.getByRole('button',{name:'Refresh portfolio'}).click();
    await expect(page.locator('#strategyList')).toContainText(`Concurrent child ${suffix}`);
    page.once('dialog', dialog => dialog.accept()); await page.locator('#strategyList .tool-card').filter({has:page.getByText(`${node.name} · strategic_product_area · root`, {exact:true})}).getByRole('button',{name:'Delete strategy'}).click();
    await expect(page.locator('#strategyList')).not.toContainText(node.name); node = null;
  } finally {
    if (node) await call('delete_strategy_node',{id:node.id});
    await call('archive_product',{id:a.id}); await call('archive_product',{id:b.id});
  }
});

test("browser edits product details and preserves a conflicting draft", async ({page,request}) => {
  test.skip(!token || !mcpToken, "Requires disposable authenticated fixture");
  async function call(action,args) {
    const response = await request.post('/api/mcp',{headers:{Authorization:`Bearer ${mcpToken}`},data:{jsonrpc:'2.0',id:1,method:'tools/call',params:{name:'aruvi_catalog',arguments:{action,arguments:args}}}});
    const data = await response.json(); expect(data.result?.isError).not.toBeTruthy(); return data.result.structuredContent.result;
  }
  const product = await call('create_product',{name:`Product editor ${Date.now()}`});
  try {
    expect((await request.post(`/api/mobile/products/${product.id}/update`,{data:{expected:product,fields:product}})).status()).toBe(401);
    const headers = {Authorization:`Bearer ${token}`};
    expect((await request.post('/api/mobile/products/other/update',{headers,data:{expected:product,fields:product}})).status()).toBe(400);
    expect((await request.post(`/api/mobile/products/${product.id}/update`,{headers,data:{expected:product,fields:{...product,name:' '}}})).status()).toBe(400);
    expect((await request.post(`/api/mobile/products/${product.id}/update`,{headers,data:{expected:product,fields:{...product,status:'archived'}}})).status()).toBe(400);
    await page.goto('/remote'); await page.locator('#tokenInput').fill(token); await page.locator('#saveButton').click();
    await page.getByRole('button',{name:'Products',exact:true}).click();
    await page.locator('#productList').getByRole('button',{name:new RegExp(product.name)}).click();
    await page.locator('#productEditSection summary').click();
    const editor = page.locator('#productEditForm');
    await editor.getByLabel('Edit product name').fill('Unsaved product');
    await call('update_product',{id:product.id,vision:'Concurrent vision'});
    await page.getByRole('button',{name:'Save product'}).click();
    await expect(page.locator('#status')).toContainText('Product changed');
    await expect(editor.getByLabel('Edit product name')).toHaveValue('Unsaved product');
    await page.locator('#productEditSection summary').click();
    await page.getByRole('button',{name:'Refresh',exact:true}).first().click();
    await page.locator('#productEditSection summary').click();
    await expect(editor.getByLabel('Vision',{exact:true})).toHaveValue('Concurrent vision');
    await editor.getByLabel('Edit product name').fill('Updated product');
    await editor.getByLabel('Product description').fill('Description');
    await editor.getByLabel('Vision',{exact:true}).fill('Vision');
    await editor.getByLabel('Goals (comma-separated)').fill('First, Second');
    await editor.getByLabel('Tags (comma-separated)').fill('alpha, beta');
    await editor.getByLabel('Lifecycle').selectOption('active'); await editor.getByLabel('Health').selectOption('watch');
    await editor.getByLabel('Owner',{exact:true}).fill('Owner'); await editor.getByLabel('Investment').selectOption('invest');
    await editor.getByLabel('Roadmap').fill('Roadmap'); await editor.getByLabel('Evidence').fill('Evidence');
    await page.getByRole('button',{name:'Save product'}).click(); await expect(page.locator('#status')).toHaveText('Product saved');
    expect(await call('get_product',{id:product.id})).toMatchObject({name:'Updated product',description:'Description',vision:'Vision',goals:['First','Second'],tags:['alpha','beta'],lifecycle:'active',health:'watch',owner_label:'Owner',investment_status:'invest',roadmap:'Roadmap',evidence:'Evidence'});
  } finally { await call('archive_product',{id:product.id}); }
});

test("browser displays product-scoped references on demand", async ({page,request}) => {
  test.skip(!token || !mcpToken, "Requires disposable authenticated fixture");
  async function call(action,args) {
    const response = await request.post('/api/mcp',{headers:{Authorization:`Bearer ${mcpToken}`},data:{jsonrpc:'2.0',id:1,method:'tools/call',params:{name:'aruvi_catalog',arguments:{action,arguments:args}}}});
    const data = await response.json(); expect(data.result?.isError).not.toBeTruthy(); return data.result.structuredContent.result;
  }
  const product = await call('create_product',{name:`Reference product ${Date.now()}`});
  const other = await call('create_product',{name:`Other references ${Date.now()}`});
  try {
    const area = await call('create_product_area',{product_id:product.id,name:'Reference area'});
    const cap = await call('create_capability',{product_area_id:area.id,name:'Reference capability'});
    await call('create_reference',{scope_type:'product',scope_id:product.id,title:'Product ref',content:'Product reference content',uri:'javascript:alert(1)'});
    for (let index=0; index<51; index++) await call('create_reference',{scope_type:'product',scope_id:product.id,title:`Paged reference ${index}`});
    await call('create_reference',{scope_type:'capability',scope_id:cap.id,title:'Capability ref',content:'Scoped capability content',uri:'https://example.com/guide'});
    const url = `/api/mobile/products/${product.id}/references?scope_type=capability&scope_id=${cap.id}`;
    expect((await request.get(url)).status()).toBe(401);
    expect((await request.get(`/api/mobile/products/${other.id}/references?scope_type=capability&scope_id=${cap.id}`,{headers:{Authorization:`Bearer ${token}`}})).status()).toBe(404);
    expect((await request.get(`/api/mobile/products/${product.id}/references?scope_type=feature&scope_id=${cap.id}`,{headers:{Authorization:`Bearer ${token}`}})).status()).toBe(404);
    const create = {scope_type:'capability',scope_id:cap.id,title:'Cross scope',reference_kind:'note',uri:'',content:''};
    expect((await request.post(`/api/mobile/products/${product.id}/references`,{data:create})).status()).toBe(401);
    expect((await request.post(`/api/mobile/products/${other.id}/references`,{headers:{Authorization:`Bearer ${token}`},data:create})).status()).toBe(404);
    await page.goto('/remote'); await page.locator('#tokenInput').fill(token); await page.locator('#saveButton').click();
    await page.getByRole('button',{name:'Products',exact:true}).click();
    await page.locator('#productList').getByRole('button',{name:new RegExp(product.name)}).click();
    const productRefs = page.locator('#productReferences details'); await productRefs.locator('summary').click();
    await productRefs.getByRole('button',{name:'Load references'}).click();
    await expect(productRefs.locator('.tool-card')).toHaveCount(50);
    await productRefs.getByRole('button',{name:'Load more references'}).click();
    await expect(productRefs.locator('.tool-card')).toHaveCount(52);
    await expect(productRefs).toContainText('Product reference content');
    await expect(productRefs.locator('a')).toHaveCount(0);
    await page.getByRole('button',{name:'Load product tree'}).click();
    await page.locator('#productTree summary').filter({hasText:'Reference area'}).click();
    const node = page.locator('#productTree details').filter({has:page.locator(':scope > summary').filter({hasText:'Reference capability'})}).last();
    await node.locator(':scope > summary').click();
    const nodeRefs = node.locator(':scope > details').filter({has:page.getByText('References',{exact:true})});
    await nodeRefs.locator(':scope > summary').click(); await nodeRefs.getByRole('button',{name:'Load references'}).click();
    await expect(nodeRefs).toContainText('Scoped capability content');
    await expect(nodeRefs.locator('a')).toHaveAttribute('href','https://example.com/guide');
    await nodeRefs.getByLabel('Reference title').fill('Browser-created reference');
    await nodeRefs.getByLabel('Reference kind').selectOption('architecture');
    await nodeRefs.getByLabel('Reference content').fill('Created from the browser');
    const responsePromise = page.waitForResponse(response => response.url().endsWith('/references') && response.request().method() === 'POST');
    await nodeRefs.getByRole('button',{name:'Add reference'}).click();
    const createdResponse = await responsePromise; expect(createdResponse.status()).toBe(201);
    const created = await createdResponse.json();
    await expect(nodeRefs).toContainText('Browser-created reference');
    const deleteUrl = `/api/mobile/products/${product.id}/references/delete`;
    expect((await request.post(deleteUrl,{data:{expected:created}})).status()).toBe(401);
    expect((await request.post(`/api/mobile/products/${other.id}/references/delete`,{headers:{Authorization:`Bearer ${token}`},data:{expected:created}})).status()).toBe(409);
    expect((await request.post(deleteUrl,{headers:{Authorization:`Bearer ${token}`},data:{expected:{...created,content:'stale'}}})).status()).toBe(409);
    page.once('dialog', dialog => dialog.accept());
    await nodeRefs.locator('.tool-card').filter({hasText:'Browser-created reference'}).getByRole('button',{name:'Delete reference'}).click();
    await expect(nodeRefs).not.toContainText('Browser-created reference');
  } finally { await call('archive_product',{id:product.id}); await call('archive_product',{id:other.id}); }
});
