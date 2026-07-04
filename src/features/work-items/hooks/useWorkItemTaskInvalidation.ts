import type { QueryClient, QueryKey } from "@tanstack/react-query";

type WorkItemTaskInvalidationInput = {
  queryClient: QueryClient;
  activeProductId: string | null;
  selectedWorkItemId: string | null;
  workflowRunId: string | null;
  workItemsScopeQueryKey: QueryKey;
  workItemsQueryKey: QueryKey;
};

export function useWorkItemTaskInvalidation({
  queryClient,
  activeProductId,
  selectedWorkItemId,
  workflowRunId,
  workItemsScopeQueryKey,
  workItemsQueryKey,
}: WorkItemTaskInvalidationInput) {
  return async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: workItemsScopeQueryKey, refetchType: "none" }),
      queryClient.invalidateQueries({ queryKey: ["sidebarWorkItems", activeProductId] }),
      queryClient.invalidateQueries({ queryKey: ["productWorkItemSummary"] }),
      queryClient.invalidateQueries({ queryKey: ["workItem", selectedWorkItemId] }),
      queryClient.invalidateQueries({ queryKey: ["latestWorkflowRun", selectedWorkItemId] }),
      queryClient.invalidateQueries({ queryKey: ["workflowHistory", workflowRunId] }),
      queryClient.invalidateQueries({ queryKey: ["agentRunsForWorkflow", workflowRunId] }),
      queryClient.invalidateQueries({ queryKey: ["agentModelCallsForWorkflow", workflowRunId] }),
      queryClient.invalidateQueries({ queryKey: ["externalCliRunsForWorkItem", selectedWorkItemId] }),
      queryClient.invalidateQueries({ queryKey: ["externalCliRunEvents"] }),
      queryClient.invalidateQueries({ queryKey: ["artifacts", selectedWorkItemId] }),
      queryClient.invalidateQueries({ queryKey: ["findings", selectedWorkItemId] }),
      queryClient.refetchQueries({ queryKey: workItemsQueryKey, type: "active" }),
      queryClient.refetchQueries({ queryKey: ["sidebarWorkItems", activeProductId], type: "active" }),
    ]);
  };
}
