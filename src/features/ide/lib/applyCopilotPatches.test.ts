import { afterEach, expect, it, vi } from "vitest";
import { useEditorStore } from "../../../state/editorStore";
import { applyCopilotPatches } from "./applyCopilotPatches";

const initial = useEditorStore.getInitialState();
afterEach(() => useEditorStore.setState(initial, true));

function open(path: string) {
  useEditorStore.getState().openFile({ id: `repo:${path}`, path, name: path, content: "original", language: "text" });
}

it("does not apply any patch when a target has unsaved changes", async () => {
  open("b.txt"); useEditorStore.getState().updateFileContent("repo:b.txt", "unsaved");
  const apply = vi.fn(async () => {}), read = vi.fn(async () => "disk");
  await expect(applyCopilotPatches("repo", [{ path: "a.txt", patch: "a" }, { path: "b.txt", patch: "b" }], { apply, read })).rejects.toThrow("unsaved changes");
  expect(apply).not.toHaveBeenCalled();
  expect(useEditorStore.getState().openFiles[0].content).toBe("unsaved");
});

it("preserves edits made while a patch is being applied", async () => {
  open("a.txt");
  let release!: () => void;
  const wait = new Promise<void>((resolve) => { release = resolve; });
  const apply = vi.fn(async () => { await wait; }), read = vi.fn(async () => "patched disk");
  const result = applyCopilotPatches("repo", [{ path: "a.txt", patch: "a" }, { path: "b.txt", patch: "b" }], { apply, read });
  useEditorStore.getState().updateFileContent("repo:a.txt", "edited during apply");
  release();
  await expect(result).rejects.toThrow("Patch applied on disk");
  expect(useEditorStore.getState().openFiles[0]).toMatchObject({ content: "edited during apply", isDirty: true });
  expect(apply).toHaveBeenCalledTimes(1);
});

it("stops before a later target edited during an earlier patch", async () => {
  open("b.txt");
  const apply = vi.fn(async () => { useEditorStore.getState().updateFileContent("repo:b.txt", "new edit"); });
  await expect(applyCopilotPatches("repo", [{ path: "a.txt", patch: "a" }, { path: "b.txt", patch: "b" }], { apply, read: async () => "disk" })).rejects.toThrow("Earlier patches may already be applied");
  expect(apply).toHaveBeenCalledTimes(1);
  expect(useEditorStore.getState().openFiles[0]).toMatchObject({ content: "new edit", isDirty: true });
});

it("refreshes a clean open file after a successful patch", async () => {
  open("a.txt");
  await applyCopilotPatches("repo", [{ path: "a.txt", patch: "a" }], { apply: async () => {}, read: async () => "patched disk" });
  expect(useEditorStore.getState().openFiles[0]).toMatchObject({ content: "patched disk", isDirty: false });
});
