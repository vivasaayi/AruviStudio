import { test, expect } from "./support/test.js";

test("desktop IDE preserves a buffer when its workspace or file changes", async ({ page }) => {
  await page.addInitScript(() => {
    const original = window.__ARUVI_E2E__.invoke.bind(window.__ARUVI_E2E__);
    window.fixtureWorkspace = { root: "/fixture/original", content: "original content", writes: [] };
    const repo = () => ({ id: "fixture-repo", name: "Fixture workspace", local_path: window.fixtureWorkspace.root, remote_url: "", default_branch: "main" });
    window.__ARUVI_E2E__.invoke = async (command, args) => {
      if (command === "list_repositories") return [repo()];
      if (command === "list_repository_attachments") return [{ repository_id: "fixture-repo" }];
      if (command === "resolve_repository_for_scope" || command === "resolve_repository_for_work_item") return repo();
      if (command === "list_repository_tree") return [{ name: "file.txt", relative_path: "file.txt", node_type: "file", children: [], size_bytes: 16 }];
      if (command === "read_repository_file_snapshot") return { content: window.fixtureWorkspace.content, workspace_root: window.fixtureWorkspace.root };
      if (command === "write_repository_file_guarded") {
        window.fixtureWorkspace.writes.push(args);
        if (args.expectedRoot !== window.fixtureWorkspace.root) throw new Error("Workspace path changed. Reopen the workspace before writing");
        if (args.expectedContent !== window.fixtureWorkspace.content) throw new Error("File changed since it was opened");
        window.fixtureWorkspace.content = args.content; return null;
      }
      return original(command, args);
    };
  });
  await page.goto("/ide");
  await page.getByText("file.txt", { exact: true }).first().click();
  const save = page.getByRole("button", { name: "Save (⌘/Ctrl+S)" });
  await expect(save).toBeEnabled();
  await page.evaluate(() => { window.fixtureWorkspace.root = "/fixture/changed"; });
  await save.click();
  await expect(page.getByText(/Workspace path changed/)).toBeVisible();
  const first = await page.evaluate(() => window.fixtureWorkspace.writes[0]);
  expect(first).toMatchObject({ repositoryId: "fixture-repo", relativePath: "file.txt", expectedRoot: "/fixture/original", expectedContent: "original content", content: "original content" });
  await page.evaluate(() => { window.fixtureWorkspace.root = "/fixture/original"; window.fixtureWorkspace.content = "external edit"; });
  await save.click();
  await expect(page.getByText(/File changed since it was opened/)).toBeVisible();
  expect(await page.evaluate(() => window.fixtureWorkspace.content)).toBe("external edit");
});

test("desktop Copilot rejects a workspace change during model generation", async ({ page }) => {
  await page.addInitScript(() => {
    const original = window.__ARUVI_E2E__.invoke.bind(window.__ARUVI_E2E__);
    const callbacks = new Map(), listeners = new Map(); let nextId = 1;
    window.__TAURI_INTERNALS__ = {
      transformCallback(callback) { const id = nextId++; callbacks.set(id, callback); return id; },
      async invoke(command, args) {
        if (command === "plugin:event|listen") { listeners.set(args.handler, { event: args.event, handler: callbacks.get(args.handler) }); return args.handler; }
        if (command === "plugin:event|unlisten") { listeners.delete(args.eventId); return; }
        throw new Error(`Unexpected Tauri command: ${command}`);
      },
    };
    window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener(_event, id) { listeners.delete(id); } };
    window.fixtureEmitCopilot = (event, payload) => { for (const [id, entry] of listeners) if (entry.event === event) entry.handler({ event, id, payload }); };
    window.fixtureWorkspace = { root: "/fixture/original", streamId: "", previews: [], applies: 0 };
    const repo = () => ({ id: "fixture-repo", name: "Fixture workspace", local_path: window.fixtureWorkspace.root, remote_url: "", default_branch: "main" });
    window.__ARUVI_E2E__.invoke = async (command, args) => {
      if (command === "list_repositories") return [repo()];
      if (command === "list_repository_attachments") return [{ repository_id: "fixture-repo" }];
      if (command === "resolve_repository_for_scope" || command === "resolve_repository_for_work_item") return repo();
      if (command === "list_repository_tree") return [];
      if (command === "get_repository_root_snapshot") return window.fixtureWorkspace.root;
      if (command === "start_desktop_copilot_stream") { window.fixtureWorkspace.streamId = args.request.stream_id; return args.request.stream_id; }
      if (command === "preview_repository_patch_snapshot") {
        window.fixtureWorkspace.previews.push(args);
        if (args.expectedRoot !== window.fixtureWorkspace.root) throw new Error("Workspace path changed. Reopen the workspace before writing");
        return { exists: false, base_sha256: "fixture", result_sha256: "fixture", result_content: "hello", workspace_root: window.fixtureWorkspace.root };
      }
      if (command === "apply_reviewed_repository_patch") { window.fixtureWorkspace.applies++; return "fixture"; }
      return original(command, args);
    };
  });
  await page.goto("/ide");
  await expect(page.getByText("Fixture workspace", { exact: true }).first()).toBeVisible();
  await page.getByRole("button", { name: "Propose Patch" }).click();
  await page.getByPlaceholder("Describe the exact change; Copilot will return a structured patch proposal...").fill("Create a file");
  await page.getByRole("button", { name: "Generate Proposal" }).click();
  await expect.poll(() => page.evaluate(() => window.fixtureWorkspace.streamId)).not.toBe("");
  await page.evaluate(() => {
    const id = window.fixtureWorkspace.streamId;
    window.fixtureWorkspace.root = "/fixture/changed";
    window.fixtureEmitCopilot("chat_stream_chunk", { stream_id: id, delta: '{"type":"patch_proposal","summary":"Fixture","patches":[{"path":"a.txt","patch":"@@ -0,0 +1 @@\\n+hello"}]}' });
    window.fixtureEmitCopilot("chat_stream_done", { stream_id: id });
  });
  await expect(page.getByText(/Workspace path changed/)).toBeVisible();
  const result = await page.evaluate(() => ({ previews: window.fixtureWorkspace.previews, applies: window.fixtureWorkspace.applies }));
  expect(result.previews[0].expectedRoot).toBe("/fixture/original");
  expect(result.applies).toBe(0);
});
