import { useMemo } from "react";
import type { Repository, WorkItem } from "../../../lib/types";

type WorkItemResolvedRepositoryInput = {
  filteredWorkItems: WorkItem[];
  repositories: Repository[];
  resolvedRepositoryFromQuery: Repository | null | undefined;
  selectedWorkItem: WorkItem | null | undefined;
  selectedWorkItemId: string | null | undefined;
};

export function useWorkItemResolvedRepository({
  filteredWorkItems,
  repositories,
  resolvedRepositoryFromQuery,
  selectedWorkItem,
  selectedWorkItemId,
}: WorkItemResolvedRepositoryInput) {
  const selectedWorkItemSummary = useMemo(
    () => selectedWorkItem ?? filteredWorkItems.find((workItem) => workItem.id === selectedWorkItemId) ?? null,
    [filteredWorkItems, selectedWorkItem, selectedWorkItemId],
  );

  const repositoryFromWorkItem = useMemo(() => {
    const repositoryId = selectedWorkItemSummary?.repo_override_id ?? selectedWorkItemSummary?.active_repo_id;
    if (!repositoryId) {
      return null;
    }
    return repositories.find((repository) => repository.id === repositoryId) ?? null;
  }, [repositories, selectedWorkItemSummary?.active_repo_id, selectedWorkItemSummary?.repo_override_id]);

  return {
    selectedWorkItemSummary,
    resolvedRepository: resolvedRepositoryFromQuery ?? repositoryFromWorkItem ?? null,
  };
}
