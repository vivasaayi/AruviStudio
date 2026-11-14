import { useEditorStore } from "../../../state/editorStore";
import type { PatchProposalItem } from "./idePageModel";

type PatchOperations = {
  apply: (input: { repositoryId: string; relativePath: string; patch: string; baseSha256?: string }) => Promise<unknown>;
  read: (input: { repositoryId: string; relativePath: string }) => Promise<string>;
};

export async function applyCopilotPatches(repositoryId: string, patches: PatchProposalItem[], operations: PatchOperations) {
  const dirty = (path: string) => useEditorStore.getState().openFiles.find((file) => file.id === `${repositoryId}:${path}`)?.isDirty;
  const existingDirty = patches.find((patch) => dirty(patch.path));
  if (existingDirty) throw new Error(`Save or close unsaved changes in ${existingDirty.path} before applying the proposal.`);
  for (const patch of patches) {
    if (dirty(patch.path)) throw new Error(`Stopped before ${patch.path}: it now has unsaved changes. Earlier patches may already be applied.`);
    await operations.apply({ repositoryId, relativePath: patch.path, patch: patch.patch, baseSha256: patch.base_sha256 ?? undefined });
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
