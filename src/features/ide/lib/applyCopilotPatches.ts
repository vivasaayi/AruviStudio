import { useEditorStore } from "../../../state/editorStore";
import type { CopilotPatchProposal, ReviewedCopilotPatchProposal, ReviewedPatch } from "./idePageModel";

type PatchOperations = {
  apply: (input: { repositoryId: string; relativePath: string; patch: string; baseSha256: string; expectedExists: boolean }) => Promise<unknown>;
  match: (input: { repositoryId: string; relativePath: string; openPaths: string[] }) => Promise<string[]>;
  read: (input: { repositoryId: string; relativePath: string }) => Promise<string>;
};

// Match the repository tree's slash-separated paths so an alias cannot bypass
// the open-editor dirty check or cause two patches to target the same path.
function validPatchPath(path: string): boolean {
  return path.length > 0 && path.length <= 4096 && !path.startsWith("/") &&
    !path.includes("\\") && !path.split("/").some((part) => !part || part === "." || part === "..");
}

export async function reviewCopilotPatches(repositoryId: string, proposal: CopilotPatchProposal, preview: (input: { repositoryId: string; relativePath: string; patch: string; baseSha256?: string }) => Promise<{ exists: boolean; base_sha256: string }>): Promise<ReviewedCopilotPatchProposal> {
  if (!repositoryId) throw new Error("Select a workspace before reviewing a patch proposal.");
  if (proposal.patches.length > 20 || proposal.patches.some((patch) => !validPatchPath(patch.path)) || new Set(proposal.patches.map((patch) => patch.path)).size !== proposal.patches.length) throw new Error("Patch proposal has too many, invalid, or duplicate paths.");
  const patches: ReviewedPatch[] = [];
  for (const patch of proposal.patches) {
    if (patch.patch.length > 200_000 || patch.path.length > 4096) throw new Error("Patch proposal exceeds its size limit.");
    const result = await preview({ repositoryId, relativePath: patch.path, patch: patch.patch, baseSha256: patch.base_sha256 ?? undefined });
    patches.push({ ...patch, base_sha256: result.base_sha256, expected_exists: result.exists });
  }
  return { ...proposal, repositoryId, patches };
}

export async function applyCopilotPatches(repositoryId: string, patches: ReviewedPatch[], operations: PatchOperations) {
  if (patches.some((patch) => !validPatchPath(patch.path))) throw new Error("Patch proposal contains an invalid path.");
  const openFiles = () => useEditorStore.getState().openFiles.filter((file) => file.id === `${repositoryId}:${file.path}`);
  // Repeat if the user opened or closed files during the asynchronous identity check.
  const matching = async (path: string) => {
    for (let attempt = 0; attempt < 5; attempt++) {
      const paths = openFiles().map((file) => file.path).sort();
      const matched = await operations.match({ repositoryId, relativePath: path, openPaths: paths });
      if (JSON.stringify(paths) === JSON.stringify(openFiles().map((file) => file.path).sort())) return matched;
    }
    throw new Error("Open files changed during patch review; try again.");
  };
  const rejectDirty = (paths: string[], message: string) => {
    const dirty = openFiles().find((file) => paths.includes(file.path) && file.isDirty);
    if (dirty) throw new Error(`${message} ${dirty.path}`);
  };
  // Check all targets before the first write, then recheck between writes.
  for (const patch of patches) {
    if (!patch.expected_exists) rejectDirty(openFiles().map((file) => file.path), "Save or close unsaved changes before creating a file:");
    rejectDirty(await matching(patch.path), "Save or close unsaved changes in");
  }
  for (const patch of patches) {
    if (!patch.expected_exists) rejectDirty(openFiles().map((file) => file.path), "Stopped before creating a file with unsaved edits open. Earlier patches may already be applied:");
    rejectDirty(await matching(patch.path), "Stopped before a file with unsaved changes. Earlier patches may already be applied:");
    if (!patch.expected_exists) rejectDirty(openFiles().map((file) => file.path), "Stopped before creating a file with unsaved edits open. Earlier patches may already be applied:");
    await operations.apply({ repositoryId, relativePath: patch.path, patch: patch.patch, baseSha256: patch.base_sha256, expectedExists: patch.expected_exists });
    const paths = await matching(patch.path);
    rejectDirty(paths, "Patch applied on disk; edits made during application remain unsaved. Reconcile:");
    for (const path of paths) {
      const content = await operations.read({ repositoryId, relativePath: path });
      const state = useEditorStore.getState();
      const file = state.openFiles.find((entry) => entry.id === `${repositoryId}:${path}`);
      if (file?.isDirty) throw new Error(`Patch applied on disk to ${path}; edits made during application remain unsaved in the editor. Reconcile them before saving.`);
      if (file) state.replaceFileContent(file.id, content);
    }
  }
}
