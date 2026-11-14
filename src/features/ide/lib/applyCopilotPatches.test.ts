import { afterEach, expect, it, vi } from "vitest";
import { useEditorStore } from "../../../state/editorStore";
import { applyCopilotPatches, reviewCopilotPatches } from "./applyCopilotPatches";

const initial = useEditorStore.getInitialState();
afterEach(() => useEditorStore.setState(initial, true));

function open(path: string) {
  useEditorStore.getState().openFile({ id: `repo:${path}`, path, name: path, content: "original", language: "text" });
}

it("does not apply any patch when a target has unsaved changes", async () => {
  open("b.txt"); useEditorStore.getState().updateFileContent("repo:b.txt", "unsaved");
  const apply = vi.fn(async () => {}), read = vi.fn(async () => "disk");
  await expect(applyCopilotPatches("repo", [{ path: "a.txt", patch: "a", base_sha256: "hash-a", expected_exists: true }, { path: "b.txt", patch: "b", base_sha256: "hash-b", expected_exists: true }], { apply, read })).rejects.toThrow("unsaved changes");
  expect(apply).not.toHaveBeenCalled();
  expect(useEditorStore.getState().openFiles[0].content).toBe("unsaved");
});

it("preserves edits made while a patch is being applied", async () => {
  open("a.txt");
  let release!: () => void;
  const wait = new Promise<void>((resolve) => { release = resolve; });
  const apply = vi.fn(async () => { await wait; }), read = vi.fn(async () => "patched disk");
  const result = applyCopilotPatches("repo", [{ path: "a.txt", patch: "a", base_sha256: "hash-a", expected_exists: true }, { path: "b.txt", patch: "b", base_sha256: "hash-b", expected_exists: true }], { apply, read });
  useEditorStore.getState().updateFileContent("repo:a.txt", "edited during apply");
  release();
  await expect(result).rejects.toThrow("Patch applied on disk");
  expect(useEditorStore.getState().openFiles[0]).toMatchObject({ content: "edited during apply", isDirty: true });
  expect(apply).toHaveBeenCalledTimes(1);
});

it("stops before a later target edited during an earlier patch", async () => {
  open("b.txt");
  const apply = vi.fn(async () => { useEditorStore.getState().updateFileContent("repo:b.txt", "new edit"); });
  await expect(applyCopilotPatches("repo", [{ path: "a.txt", patch: "a", base_sha256: "hash-a", expected_exists: true }, { path: "b.txt", patch: "b", base_sha256: "hash-b", expected_exists: true }], { apply, read: async () => "disk" })).rejects.toThrow("Earlier patches may already be applied");
  expect(apply).toHaveBeenCalledTimes(1);
  expect(useEditorStore.getState().openFiles[0]).toMatchObject({ content: "new edit", isDirty: true });
});

it("refreshes a clean open file after a successful patch", async () => {
  open("a.txt");
  await applyCopilotPatches("repo", [{ path: "a.txt", patch: "a", base_sha256: "hash-a", expected_exists: true }], { apply: async () => {}, read: async () => "patched disk" });
  expect(useEditorStore.getState().openFiles[0]).toMatchObject({ content: "patched disk", isDirty: false });
});


it("records preview hashes and existence for guarded application", async () => {
  const preview = vi.fn(async ({ relativePath }: { relativePath: string }) => ({ exists: relativePath === "a.txt", base_sha256: `hash-${relativePath}` }));
  const proposal = { raw: "", summary: "Change files", patches: [{ path: "a.txt", patch: "a" }, { path: "new.txt", patch: "b" }] };
  const reviewed = await reviewCopilotPatches("repo", proposal, preview);
  expect(reviewed.patches).toMatchObject([{ base_sha256: "hash-a.txt", expected_exists: true }, { base_sha256: "hash-new.txt", expected_exists: false }]);
  const apply = vi.fn(async () => {});
  await applyCopilotPatches("repo", reviewed.patches, { apply, read: async () => "" });
  expect(apply).toHaveBeenNthCalledWith(2, { repositoryId: "repo", relativePath: "new.txt", patch: "b", baseSha256: "hash-new.txt", expectedExists: false });
});

it("rejects duplicate targets before preview", async () => {
  const preview = vi.fn();
  await expect(reviewCopilotPatches("repo", { raw: "", summary: "", patches: [{ path: "a", patch: "x" }, { path: "a", patch: "y" }] }, preview)).rejects.toThrow("duplicate");
  expect(preview).not.toHaveBeenCalled();
});


it.each(["dir/./file.txt", "dir//file.txt", "dir\\file.txt", "./a.txt", "dir/../a.txt", "/a.txt"])(
  "rejects an alias or invalid path before preview or application: %s", async (path) => {
    const preview = vi.fn();
    await expect(reviewCopilotPatches("repo", { raw: "", summary: "", patches: [{ path, patch: "x" }] }, preview)).rejects.toThrow("invalid");
    expect(preview).not.toHaveBeenCalled();
    const apply = vi.fn();
    await expect(applyCopilotPatches("repo", [{ path, patch: "x", base_sha256: "hash", expected_exists: true }], { apply, read: async () => "" })).rejects.toThrow("invalid path");
    expect(apply).not.toHaveBeenCalled();
  },
);
