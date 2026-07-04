import type { QueryClient } from "@tanstack/react-query";

import type { Repository, WorkItem } from "../../../lib/types";
import { WorkItemWorkspaceAssignmentPanel } from "../components/WorkItemWorkspaceAssignmentPanel";
import { useWorkItemWorkspaceEditor } from "./useWorkItemWorkspaceEditor";
import { useWorkItemWorkspaceMutations } from "./useWorkItemWorkspaceMutations";

type WorkItemWorkspaceAssignmentInput = {
  queryClient: QueryClient;
  activeProductId: string | null;
  activeProductAreaId: string | null;
  selectedWorkItemId: string | null;
  selectedWorkItem: WorkItem | null;
  repositories: Repository[];
  resolvedRepository: Repository | null;
  workItemsScopeQueryKey: readonly unknown[];
  workItemsQueryKey: readonly unknown[];
  setActionError: (error: string | null) => void;
  setActionInfo: (info: string | null) => void;
  setActiveView: (view: "ide") => void;
  invalidateTasks: () => Promise<void>;
};

export function useWorkItemWorkspaceAssignment({
  queryClient,
  activeProductId,
  activeProductAreaId,
  selectedWorkItemId,
  selectedWorkItem,
  repositories,
  resolvedRepository,
  workItemsScopeQueryKey,
  workItemsQueryKey,
  setActionError,
  setActionInfo,
  setActiveView,
  invalidateTasks,
}: WorkItemWorkspaceAssignmentInput) {
  const {
    isEditingWorkspace,
    setIsEditingWorkspace,
    workspaceRepositoryId,
    workspaceBranchMode,
    setWorkspaceBranchMode,
    workspaceBranchName,
    setWorkspaceBranchName,
    branchPreview,
    openWorkspaceEditor,
    selectWorkspaceRepository,
  } = useWorkItemWorkspaceEditor({
    repositories,
    selectedWorkItem,
    resolvedRepository,
  });

  const {
    createWorkspaceMutation,
    assignWorkspaceMutation,
    clearWorkspaceOverrideMutation,
  } = useWorkItemWorkspaceMutations({
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
  });

  const workspaceAssignmentPanel = (
    <WorkItemWorkspaceAssignmentPanel
      isEditing={isEditingWorkspace}
      repositories={repositories}
      workspaceRepositoryId={workspaceRepositoryId}
      workspaceBranchMode={workspaceBranchMode}
      workspaceBranchName={workspaceBranchName}
      currentBranch={selectedWorkItem?.branch_name || resolvedRepository?.default_branch || "not set"}
      branchPreview={branchPreview}
      hasWorkspaceOverride={!!selectedWorkItem?.repo_override_id}
      isAssignPending={assignWorkspaceMutation.isPending}
      isClearPending={clearWorkspaceOverrideMutation.isPending}
      onOpenEditor={() => {
        setActionError(null);
        openWorkspaceEditor();
      }}
      onClearOverride={() => clearWorkspaceOverrideMutation.mutate()}
      onRepositoryIdChange={selectWorkspaceRepository}
      onBranchModeChange={setWorkspaceBranchMode}
      onBranchNameChange={setWorkspaceBranchName}
      onSave={() => assignWorkspaceMutation.mutate()}
      onCancel={() => setIsEditingWorkspace(false)}
    />
  );

  return {
    createWorkspace: () => createWorkspaceMutation.mutate(),
    isCreateWorkspacePending: createWorkspaceMutation.isPending,
    workspaceAssignmentPanel,
  };
}
