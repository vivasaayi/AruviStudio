import { useMemo, useState } from "react";

import type { Repository, WorkItem } from "../../../lib/types";
import {
  workItemBranchName,
  type WorkspaceBranchMode,
} from "../lib/workItemListPageHelpers";

type WorkItemWorkspaceEditorInput = {
  repositories: Repository[];
  selectedWorkItem: WorkItem | null;
  resolvedRepository: Repository | null;
};

export function useWorkItemWorkspaceEditor({
  repositories,
  selectedWorkItem,
  resolvedRepository,
}: WorkItemWorkspaceEditorInput) {
  const [isEditingWorkspace, setIsEditingWorkspace] = useState(false);
  const [workspaceRepositoryId, setWorkspaceRepositoryId] = useState("");
  const [workspaceBranchMode, setWorkspaceBranchMode] = useState<WorkspaceBranchMode>("default");
  const [workspaceBranchName, setWorkspaceBranchName] = useState("");

  const selectedWorkspaceRepository = useMemo(
    () => repositories.find((repository) => repository.id === workspaceRepositoryId) ?? null,
    [repositories, workspaceRepositoryId],
  );

  const branchPreview = useMemo(() => {
    if (workspaceBranchMode === "default") {
      return selectedWorkspaceRepository?.default_branch ?? "";
    }
    if (workspaceBranchMode === "work_item") {
      return selectedWorkItem ? workItemBranchName(selectedWorkItem.title) : "";
    }
    return workspaceBranchName.trim();
  }, [selectedWorkItem, selectedWorkspaceRepository?.default_branch, workspaceBranchMode, workspaceBranchName]);

  const openWorkspaceEditor = () => {
    const currentRepositoryId =
      resolvedRepository?.id ??
      selectedWorkItem?.repo_override_id ??
      selectedWorkItem?.active_repo_id ??
      repositories[0]?.id ??
      "";
    const currentRepository = repositories.find((repository) => repository.id === currentRepositoryId) ?? null;
    const currentBranch = selectedWorkItem?.branch_name ?? currentRepository?.default_branch ?? "";
    setWorkspaceRepositoryId(currentRepositoryId);
    setWorkspaceBranchName(currentBranch);
    setWorkspaceBranchMode(currentRepository && currentBranch === currentRepository.default_branch ? "default" : "custom");
    setIsEditingWorkspace(true);
  };

  const selectWorkspaceRepository = (repositoryId: string) => {
    const nextRepository = repositories.find((repository) => repository.id === repositoryId) ?? null;
    setWorkspaceRepositoryId(repositoryId);
    if (workspaceBranchMode === "default") {
      setWorkspaceBranchName(nextRepository?.default_branch ?? "");
    }
  };

  return {
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
  };
}
