import { useEditorStore } from "../../../state/editorStore";
import type { CopilotPatchProposal, ReviewedCopilotPatchProposal, ReviewedPatch } from "./idePageModel";

type PatchOperations = {
  apply: (input: { repositoryId: string; relativePath: string; patch: string; baseSha256: string; expectedExists: boolean }) => Promise<unknown>;
  read: (input: { repositoryId: string; relativePath: string }) => Promise<string>;
};

export async function reviewCopilotPatches(repositoryId: string, proposal: CopilotPatchProposal, preview: (input: { repositoryId: string; relativePath: string; patch: string; baseSha256?: string }) => Promise<{ exists: boolean; base_sha256: string }>): Promise<ReviewedCopilotPatchProposal> {
  if (!repositoryId) throw new Error("Select a workspace before reviewing a patch proposal.");
  if (proposal.patches.length > 20 || new Set(proposal.patches.map((patch) => patch.path)).size !== proposal.patches.length) throw new Error("Patch proposal has too many or duplicate paths.");
  const patches: ReviewedPatch[] = [];
  for (const patch of proposal.patches) {
    if (patch.patch.length > 200_000 || patch.path.length > 4096) throw new Error("Patch proposal exceeds its size limit.");
    const result = await preview({ repositoryId, relativePath: patch.path, patch: patch.patch, baseSha256: patch.base_sha256 ?? undefined });
    patches.push({ ...patch, base_sha256: result.base_sha256, expected_exists: result.exists });
  }
  return { ...proposal, repositoryId, patches };
}

export async function applyCopilotPatches(repositoryId: string, patches: ReviewedPatch[], operations: PatchOperations) {
  const dirty = (path: string) => useEditorStore.getState().openFiles.find((file) => file.id === `${repositoryId}:${path}`)?.isDirty;
  const existingDirty = patches.find((patch) => dirty(patch.path));
  if (existingDirty) throw new Error(`Save or close unsaved changes in ${existingDirty.path} before applying the proposal.`);
  for (const patch of patches) {
    if (dirty(patch.path)) throw new Error(`Stopped before ${patch.path}: it now has unsaved changes. Earlier patches may already be applied.`);
    await operations.apply({ repositoryId, relativePath: patch.path, patch: patch.patch, baseSha256: patch.base_sha256, expectedExists: patch.expected_exists });
    const id = `${repositoryId}:${patch.path}`;
    if (useEditorStore.getState().openFiles.some((file) => file.id === id)) {
      const content = await operations.read({ repositoryId, relativePath: patch.path });
      const state = useEditorStore.getState();
      const file = state.openFiles.find((entry) => entry.id === id);
      if (file?.isDirty) throw new Error(`Patch applied on disk to ${patch.path}; edits made during application remain unsaved in the editor. Reconcile them before saving.`);
      if (file) state.replaceFileContent(id, content);
    }
  }
}
