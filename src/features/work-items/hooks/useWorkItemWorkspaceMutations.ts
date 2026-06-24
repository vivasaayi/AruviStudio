import { useMutation, type QueryClient } from "@tanstack/react-query";

import {
  assignWorkItemWorkspace,
  createLocalWorkspace,
} from "../../../lib/tauri";
import type { Repository, WorkItem } from "../../../lib/types";

type WorkItemWorkspaceMutationsInput = {
  queryClient: QueryClient;
  activeProductId: string | null;
  activeProductAreaId: string | null;
  selectedWorkItemId: string | null;
  selectedWorkItem: WorkItem | null;
  repositories: Repository[];
  workspaceRepositoryId: string;
  branchPreview: string;
  workItemsScopeQueryKey: readonly unknown[];
  workItemsQueryKey: readonly unknown[];
  setActionError: (error: string | null) => void;
  setActionInfo: (info: string | null) => void;
  setIsEditingWorkspace: (isEditing: boolean) => void;
  setActiveView: (view: "ide") => void;
  invalidateTasks: () => Promise<void>;
};

export function useWorkItemWorkspaceMutations({
  queryClient,
  activeProductId,
  activeProductAreaId,
  selectedWorkItemId,
  selectedWorkItem,
  repositories,
  workspaceRepositoryId,
  branchPreview,
  workItemsScopeQueryKey,
  workItemsQueryKey,
  setActionError,
  setActionInfo,
  setIsEditingWorkspace,
  setActiveView,
  invalidateTasks,
}: WorkItemWorkspaceMutationsInput) {
  const createWorkspaceMutation = useMutation({
    mutationFn: async () => {
      if (!selectedWorkItem) {
        throw new Error("No story selected.");
      }
      return createLocalWorkspace({
        productId: selectedWorkItem.product_id ?? activeProductId,
        productAreaId: selectedWorkItem.product_area_id ?? activeProductAreaId,
        workItemId: selectedWorkItem.id,
      });
    },
    onSuccess: async () => {
      setActionError(null);
      setActionInfo("Workspace created and attached. Opening IDE.");
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["repositories"] }),
        queryClient.invalidateQueries({ queryKey: ["resolvedRepositoryForWorkItem", selectedWorkItemId] }),
        queryClient.invalidateQueries({ queryKey: ["ideScopeRepo"] }),
        invalidateTasks(),
      ]);
      setActiveView("ide");
    },
    onError: (error) => setActionError(String(error)),
  });

  const assignWorkspaceMutation = useMutation({
    mutationFn: async () => {
      if (!selectedWorkItem) {
        throw new Error("No story selected.");
      }
      if (!workspaceRepositoryId) {
        throw new Error("Select a workspace.");
      }
      const branchName = branchPreview;
      if (!branchName) {
        throw new Error("Branch name is required.");
      }
      return assignWorkItemWorkspace({
        id: selectedWorkItem.id,
        repositoryId: workspaceRepositoryId,
        branchName,
      });
    },
    onSuccess: async (updatedWorkItem) => {
      setActionError(null);
      setActionInfo("Workspace and branch updated for this story.");
      setIsEditingWorkspace(false);
      queryClient.setQueryData(["workItem", selectedWorkItemId], updatedWorkItem);
      const updatedRepositoryId = updatedWorkItem.repo_override_id ?? updatedWorkItem.active_repo_id;
      const updatedRepository = repositories.find((repository: Repository) => repository.id === updatedRepositoryId) ?? null;
      if (updatedRepository) {
        queryClient.setQueryData(["resolvedRepositoryForWorkItem", selectedWorkItemId], updatedRepository);
      }
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["workItem", selectedWorkItemId] }),
        queryClient.invalidateQueries({ queryKey: workItemsScopeQueryKey, refetchType: "none" }),
        queryClient.refetchQueries({ queryKey: workItemsQueryKey, type: "active" }),
        queryClient.invalidateQueries({ queryKey: ["resolvedRepositoryForWorkItem", selectedWorkItemId] }),
        queryClient.invalidateQueries({ queryKey: ["ideScopeRepo"] }),
      ]);
    },
    onError: (error) => setActionError(String(error)),
  });

  const clearWorkspaceOverrideMutation = useMutation({
    mutationFn: async () => {
      if (!selectedWorkItem) {
        throw new Error("No story selected.");
      }
      return assignWorkItemWorkspace({
        id: selectedWorkItem.id,
        repositoryId: null,
        branchName: null,
      });
    },
    onSuccess: async (updatedWorkItem) => {
      setActionError(null);
      setActionInfo("Work item workspace override cleared. Scope defaults will be used.");
      setIsEditingWorkspace(false);
      queryClient.setQueryData(["workItem", selectedWorkItemId], updatedWorkItem);
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["workItem", selectedWorkItemId] }),
        queryClient.invalidateQueries({ queryKey: workItemsScopeQueryKey, refetchType: "none" }),
        queryClient.refetchQueries({ queryKey: workItemsQueryKey, type: "active" }),
        queryClient.invalidateQueries({ queryKey: ["resolvedRepositoryForWorkItem", selectedWorkItemId] }),
        queryClient.invalidateQueries({ queryKey: ["ideScopeRepo"] }),
      ]);
    },
    onError: (error) => setActionError(String(error)),
  });

  return {
    createWorkspaceMutation,
    assignWorkspaceMutation,
    clearWorkspaceOverrideMutation,
  };
}
