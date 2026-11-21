// Disposable native-server/browser fixture. The fake CLI only prints its environment.
import assert from "node:assert/strict";
import { access, mkdtemp, mkdir, realpath, writeFile, chmod, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { randomUUID } from "node:crypto";
import { execFileSync, spawn } from "node:child_process";
import net from "node:net";
import { chromium } from "@playwright/test";

const root = await realpath(await mkdtemp(join(tmpdir(), "aruvi-external-cli-")));
const bin = join(root, "bin"), home = join(root, "cli-home"), repo = join(root, "repo"), overrideRepo = join(root, "override");
let server, secondServer, container, secondContainer, volume, browser;
try {
  await Promise.all([mkdir(bin), mkdir(home), mkdir(repo), mkdir(overrideRepo)]);
  if (process.env.ARUVI_CLI_TEST_IMAGE) await Promise.all([root, bin, home, repo, overrideRepo].map((path) => chmod(path, 0o777)));
  await writeFile(join(bin, "codex"), "#!/bin/sh\nprintf 'fixture-cli-started\\nworkspace:%s\\n' \"$PWD\"\n/usr/bin/env | /usr/bin/sort\n/bin/sleep 8\n/usr/bin/head -c 1100000 /dev/zero | /usr/bin/tr '\\000' x\n");
  await chmod(join(bin, "codex"), 0o755);
  const port = await new Promise((resolvePort, reject) => {
    const socket = net.createServer(); socket.on("error", reject);
    socket.listen(0, "127.0.0.1", () => { const port = socket.address().port; socket.close(() => resolvePort(port)); });
  });
  const base = `http://127.0.0.1:${port}`, token = "external-cli-fixture-token";
  const binary = process.env.ARUVI_CLI_TEST_LINUX_BINARY || resolve(dirname(fileURLToPath(import.meta.url)), "../src-tauri/target/debug/aruvi-studio-server");
  if (!process.env.ARUVI_CLI_TEST_IMAGE || process.env.ARUVI_CLI_TEST_LINUX_BINARY) await access(binary);
  const environment = {
    ...process.env, PATH:`${bin}:/usr/bin:/bin`, ARUVI_APP_DATA_DIR:root, ARUVI_DB_PATH:join(root,"studio.db"),
    ARUVI_WEBHOOK_HOST:"127.0.0.1", ARUVI_WEBHOOK_PORT:String(port), ARUVI_MOBILE_API_TOKEN:token,
    ARUVI_MCP_API_TOKEN:token, ARUVI_EXTERNAL_CLI_ENABLED:"1", ARUVI_EXTERNAL_CLI_HOME:home,
    ARUVI_TEST_PRIVATE_SECRET:"must-not-leak",
  };
  if (process.env.ARUVI_CLI_TEST_IMAGE) {
    volume = `aruvi-cli-fixture-${randomUUID()}`;
    execFileSync("docker",["volume","create",volume],{stdio:"ignore"});
    environment.ARUVI_APP_DATA_DIR = "/data";
    environment.ARUVI_DB_PATH = "/data/studio.db";
  }
  function startContainer(selectedPort) {
      const options = ["run","--rm","-d","--entrypoint","/usr/local/bin/aruvi-studio-server","-p",`127.0.0.1:${selectedPort}:8787`,"-v",`${root}:${root}`];
      options.push("--mount",`type=volume,src=${volume},dst=/data`);
      if (process.env.ARUVI_CLI_TEST_LINUX_BINARY) options.push("-v",`${binary}:/usr/local/bin/aruvi-studio-server:ro`);
      for (const key of ["PATH","ARUVI_APP_DATA_DIR","ARUVI_DB_PATH","ARUVI_MOBILE_API_TOKEN","ARUVI_MCP_API_TOKEN","ARUVI_EXTERNAL_CLI_ENABLED","ARUVI_EXTERNAL_CLI_HOME","ARUVI_TEST_PRIVATE_SECRET"]) options.push("-e",`${key}=${environment[key]}`);
      options.push("-e","ARUVI_WEBHOOK_HOST=0.0.0.0","-e","ARUVI_WEBHOOK_PORT=8787",process.env.ARUVI_CLI_TEST_IMAGE);
      return execFileSync("docker",options,{encoding:"utf8"}).trim();
  }
  async function startServer() {
    if (process.env.ARUVI_CLI_TEST_IMAGE) container = startContainer(port);
    else server = spawn(binary, [], {stdio:"ignore", env:environment});
    for (let attempt = 0; ; attempt++) {
      if (server && server.exitCode !== null) throw new Error("Fixture server exited before readiness");
      try { if ((await fetch(`${base}/health`)).ok) break; } catch {}
      if (attempt >= 100) throw new Error("Fixture server did not start");
      await new Promise((done) => setTimeout(done, 100));
    }
  }
  async function stopServer() {
    if (container) { execFileSync("docker",["stop","--time","2",container],{stdio:"ignore"}); container = undefined; }
    if (server && server.exitCode === null) { server.kill("SIGTERM"); await new Promise((done) => server.once("exit",done)); }
    server = undefined;
  }
  await startServer();
  async function call(tool, action, args) {
    const response = await fetch(`${base}/api/mcp`, {method:"POST",headers:{Authorization:`Bearer ${token}`,"Content-Type":"application/json"},body:JSON.stringify({jsonrpc:"2.0",id:1,method:"tools/call",params:{name:tool,arguments:{action,arguments:args}}})});
    assert.equal(response.status, 200); const body = await response.json();
    assert.ok(!body.result?.isError, JSON.stringify(body.result)); return body.result.structuredContent.result;
  }
  function sqlite(program, ...args) {
    if (volume) return execFileSync("docker",["run","--rm","--network","none","--mount",`type=volume,src=${volume},dst=/data`,"python:3.12-slim","python","-c",program,"/data/studio.db",...args],{encoding:"utf8"}).trim();
    return execFileSync("python3",["-c",program,join(root,"studio.db"),...args],{encoding:"utf8"}).trim();
  }
  const product = await call("aruvi_catalog", "create_product", {name:"External CLI fixture"});
  const repository = await call("aruvi_repositories", "register_repository", {name:"CLI fixture",local_path:repo});
  await call("aruvi_repositories", "attach_repository", {scope_type:"product",scope_id:product.id,repository_id:repository.id});
  const item = await call("aruvi_work_items", "create_work_item", {product_id:product.id,title:"CLI fixture task"});
  await call("aruvi_work_items", "update_work_item", {id:item.id,status:"approved"});
  const override = await call("aruvi_repositories", "register_repository", {name:"CLI override",local_path:overrideRepo});
  await call("aruvi_work_items", "assign_work_item_workspace", {id:item.id,repository_id:override.id});
  const before = await call("aruvi_work_items", "get_work_item", {id:item.id});
  const path = `/api/mobile/work-items/${item.id}/external-cli`;
  assert.equal((await fetch(base+path)).status,401);
  browser = await chromium.launch({executablePath:process.env.ARUVI_CHROME_PATH || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",headless:true});
  const page = await browser.newPage(); await page.goto(base+"/remote");
  await page.locator("#tokenInput").fill(token); await page.locator("#saveButton").click();
  await page.getByRole("button",{name:"Work",exact:true}).click();
  await page.locator("#workProductSelect").selectOption(product.id);
  await page.locator("#workList").getByRole("button",{name:/CLI fixture task/}).click();
  page.once("dialog", (dialog) => dialog.accept());
  await page.getByRole("button",{name:"Run CLI",exact:true}).click();
  await page.locator("#status").getByText("External CLI started",{exact:true}).waitFor();
  sqlite("import sqlite3,sys; c=sqlite3.connect(sys.argv[1]); c.execute(\"INSERT INTO bulk_import_jobs (id,source_path,import_format,status) VALUES ('fixture-bulk','fixture.json','json','running')\"); c.commit(); c.close()");
  {
    const secondPort = await new Promise((resolvePort, reject) => {
      const socket = net.createServer(); socket.on("error",reject);
      socket.listen(0,"127.0.0.1",() => { const selected = socket.address().port; socket.close(() => resolvePort(selected)); });
    });
    if (process.env.ARUVI_CLI_TEST_IMAGE) secondContainer = startContainer(secondPort);
    else secondServer = spawn(binary, [], {stdio:"ignore",env:{...environment, ARUVI_WEBHOOK_PORT:String(secondPort)}});
    for (let attempt=0;;attempt++) {
      if (secondServer && secondServer.exitCode !== null) throw new Error("Second fixture server exited");
      try { if ((await fetch(`http://127.0.0.1:${secondPort}/health`)).ok) break; } catch {}
      if (attempt>=100) throw new Error("Second fixture server did not start");
      await new Promise((done) => setTimeout(done,100));
    }
    const during = await call("aruvi_work_items", "list_external_cli_runs", {work_item_id:item.id});
    assert.equal(during[0]?.status,"running", "Another Studio process must not recover a live run");
    assert.equal(sqlite("import sqlite3,sys; c=sqlite3.connect(sys.argv[1]); print(c.execute(\"SELECT status FROM bulk_import_jobs WHERE id='fixture-bulk'\").fetchone()[0]); c.close()"),"running");
  }
  const duplicate = await fetch(base+path, {method:"POST",headers:{Authorization:`Bearer ${token}`,"Content-Type":"application/json"},body:JSON.stringify({provider:"codex"})});
  assert.equal(duplicate.status,400);
  assert.match(await duplicate.text(), /CLI run or workflow is active/);
  const deleteResponse = await fetch(`${base}/api/mcp`, {method:"POST",headers:{Authorization:`Bearer ${token}`,"Content-Type":"application/json"},body:JSON.stringify({jsonrpc:"2.0",id:2,method:"tools/call",params:{name:"aruvi_work_items",arguments:{action:"delete_work_item",arguments:{id:item.id}}}})});
  const deleted = await deleteResponse.json(); assert.equal(deleted.result.isError, true);
  assert.match(JSON.stringify(deleted.result), /CLI run/);
  const changeWorkspace = await fetch(`${base}/api/mcp`, {method:"POST",headers:{Authorization:`Bearer ${token}`,"Content-Type":"application/json"},body:JSON.stringify({jsonrpc:"2.0",id:3,method:"tools/call",params:{name:"aruvi_work_items",arguments:{action:"assign_work_item_workspace",arguments:{id:item.id,repository_id:repository.id}}}})});
  const changed = await changeWorkspace.json(); assert.equal(changed.result.isError,true);
  assert.match(JSON.stringify(changed.result),/CLI run/);
  const workflow = await fetch(`${base}/api/mobile/work-items/${item.id}/workflow/start`, {method:"POST",headers:{Authorization:`Bearer ${token}`,"Content-Type":"application/json"},body:"{}"});
  assert.equal(workflow.status,409);
  assert.match(await workflow.text(), /external CLI run is active/i);
  let runs;
  for (let attempt=0; attempt<300; attempt++) {
    runs = await call("aruvi_work_items", "list_external_cli_runs", {work_item_id:item.id});
    if (runs[0]?.status !== "running") break;
    await new Promise((done) => setTimeout(done,100));
  }
  assert.equal(runs[0]?.status,"completed");
  assert.equal((await call("aruvi_work_items", "get_work_item", {id:item.id})).active_repo_id, before.active_repo_id);
  const events = await call("aruvi_work_items", "list_external_cli_events", {run_id:runs[0].id});
  const output = events.map((event) => event.message).join("\n");
  assert.match(output,/fixture-cli-started/); assert.match(output,/\[output truncated\]/);
  assert.ok(output.includes(`workspace:${await realpath(overrideRepo)}`));
  assert.ok(output.length < 1_100_000); assert.ok(events.length <= 270);
  assert.doesNotMatch(output,/must-not-leak|external-cli-fixture-token/);
  await page.getByRole("button",{name:"Refresh runs"}).click();
  await page.locator("#externalCliRuns").getByRole("button",{name:/Codex CLI · completed/}).click();
  await page.locator("#externalCliEvents").getByText(/fixture-cli-started/).waitFor();
  const missing = await call("aruvi_work_items", "run_external_cli", {work_item_id:item.id,provider:"claude"});
  let failed;
  for (let attempt=0; attempt<300; attempt++) {
    const history = await call("aruvi_work_items", "list_external_cli_runs", {work_item_id:item.id});
    failed = history.find((run) => run.id === missing.id);
    if (failed?.status !== "running") break;
    await new Promise((done) => setTimeout(done,100));
  }
  assert.equal(failed?.status,"failed"); assert.match(failed.error_message,/Failed to launch/);
  const defaultItem = await call("aruvi_work_items", "create_work_item", {product_id:product.id,title:"Default workspace task"});
  const defaultRun = await call("aruvi_work_items", "run_external_cli", {work_item_id:defaultItem.id,provider:"codex"});
  assert.equal(defaultRun.cwd, repo);
  assert.equal((await call("aruvi_work_items", "get_work_item", {id:defaultItem.id})).active_repo_id, null);
  let defaultStatus;
  for (let attempt=0; attempt<300; attempt++) {
    const history = await call("aruvi_work_items", "list_external_cli_runs", {work_item_id:defaultItem.id});
    defaultStatus = history.find((run) => run.id === defaultRun.id)?.status;
    if (defaultStatus !== "running") break;
    await new Promise((done) => setTimeout(done,100));
  }
  assert.equal(defaultStatus,"completed");
  if (secondContainer) { execFileSync("docker",["stop","--time","2",secondContainer],{stdio:"ignore"}); secondContainer = undefined; }
  if (secondServer && secondServer.exitCode === null) { secondServer.kill("SIGTERM"); await new Promise((done) => secondServer.once("exit",done)); }
  secondServer = undefined;
  await stopServer();
  const inject = "import sqlite3,sys; c=sqlite3.connect(sys.argv[1]); c.execute(\"UPDATE external_cli_runs SET status='running', error_message=NULL, ended_at=NULL WHERE id=?\", (sys.argv[2],)); c.commit(); c.close()";
  sqlite(inject,defaultRun.id);
  environment.ARUVI_EXTERNAL_CLI_ENABLED = "0";
  await startServer();
  const restored = await call("aruvi_work_items", "list_external_cli_runs", {work_item_id:item.id});
  assert.equal(restored.find((run) => run.id === runs[0].id)?.status,"completed");
  assert.equal(restored.find((run) => run.id === missing.id)?.status,"failed");
  const recovered = await call("aruvi_work_items", "list_external_cli_runs", {work_item_id:defaultItem.id});
  assert.equal(recovered.find((run) => run.id === defaultRun.id)?.status,"failed");
  assert.equal(sqlite("import sqlite3,sys; c=sqlite3.connect(sys.argv[1]); print(c.execute(\"SELECT status FROM bulk_import_jobs WHERE id='fixture-bulk'\").fetchone()[0]); c.close()"),"failed");
  const recoveredEvents = await call("aruvi_work_items", "list_external_cli_events", {run_id:defaultRun.id});
  assert.ok(recoveredEvents.some((event) => event.message.includes("restarted before the external CLI process")));
  const disabled = await fetch(base+path, {method:"POST",headers:{Authorization:`Bearer ${token}`,"Content-Type":"application/json"},body:JSON.stringify({provider:"codex"})});
  assert.equal(disabled.status,400); assert.match(await disabled.text(),/not enabled/);
  console.log("Browser and MCP verified fake CLI execution, multi-process coordination and restart recovery without inheriting server secrets.");
} finally {
  if (browser) await browser.close();
  if (secondContainer) { try { execFileSync("docker",["rm","-f",secondContainer],{stdio:"ignore"}); } catch {} }
  if (secondServer && secondServer.exitCode === null) { secondServer.kill("SIGTERM"); await new Promise((done) => secondServer.once("exit",done)); }
  if (container) { try { execFileSync("docker",["rm","-f",container],{stdio:"ignore"}); } catch {} }
  if (server && server.exitCode === null) { server.kill("SIGTERM"); await new Promise((done) => server.once("exit",done)); }
  if (volume) { try { execFileSync("docker",["volume","rm",volume],{stdio:"ignore"}); } catch {} }
  await rm(root,{recursive:true,force:true});
}
