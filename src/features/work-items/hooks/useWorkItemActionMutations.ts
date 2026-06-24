import { useMutation, type QueryClient } from "@tanstack/react-query";

import {
  approveWorkItem,
  approveWorkItemPlan,
  approveWorkItemTestReview,
  handleWorkflowUserAction,
  invokeExternalCliForWorkItem,
  markWorkflowRunFailed,
  rejectWorkItem,
  rejectWorkItemPlan,
  reorderWorkItems,
  restartWorkflowRun,
  startWorkItemWorkflow,
} from "../../../lib/tauri";
import type { ExternalCliProvider } from "../lib/workItemListPageHelpers";

type WorkItemWorkspaceTab = "backlog" | "detail" | "external_cli" | "review";

type WorkItemActionMutationsInput = {
  queryClient: QueryClient;
  selectedWorkItemId: string | null;
  workflowRunId: string | null;
  setActiveWorkflowRunId: (workflowRunId: string | null) => void;
  setSelectedExternalCliRunId: (runId: string | null) => void;
  setActionError: (error: string | null) => void;
  setActionInfo: (info: string | null) => void;
  setWorkItemWorkspaceTab: (tab: WorkItemWorkspaceTab) => void;
  invalidateTasks: () => Promise<void>;
};

export function useWorkItemActionMutations({
  queryClient,
  selectedWorkItemId,
  workflowRunId,
  setActiveWorkflowRunId,
  setSelectedExternalCliRunId,
  setActionError,
  setActionInfo,
  setWorkItemWorkspaceTab,
  invalidateTasks,
}: WorkItemActionMutationsInput) {
  const approveMutation = useMutation({
    mutationFn: () => approveWorkItem(selectedWorkItemId!, "Approved from story workspace"),
    onSuccess: async () => {
      setActionError(null);
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["approvals", selectedWorkItemId] }),
        invalidateTasks(),
      ]);
    },
    onError: (error) => setActionError(String(error)),
  });

  const rejectMutation = useMutation({
    mutationFn: () => rejectWorkItem(selectedWorkItemId!, "Rejected from story workspace"),
    onSuccess: async () => {
      setActionError(null);
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["approvals", selectedWorkItemId] }),
        invalidateTasks(),
      ]);
    },
    onError: (error) => setActionError(String(error)),
  });

  const workflowMutation = useMutation({
    mutationFn: () => startWorkItemWorkflow(selectedWorkItemId!),
    onSuccess: async (newWorkflowRunId) => {
      setActiveWorkflowRunId(newWorkflowRunId);
      setActionError(null);
      await invalidateTasks();
      setWorkItemWorkspaceTab("review");
    },
    onError: (error) => setActionError(String(error)),
  });

  const externalCliMutation = useMutation({
    mutationFn: (provider: ExternalCliProvider) => {
      if (!selectedWorkItemId) {
        throw new Error("No story selected.");
      }
      return invokeExternalCliForWorkItem({ workItemId: selectedWorkItemId, provider });
    },
    onSuccess: async (run) => {
      setActionError(null);
      setSelectedExternalCliRunId(run.id);
      setActionInfo(`${run.label} started. Follow progress in External CLI.`);
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["externalCliRunsForWorkItem", selectedWorkItemId] }),
        queryClient.invalidateQueries({ queryKey: ["externalCliRunEvents", run.id] }),
        queryClient.invalidateQueries({ queryKey: ["artifacts", selectedWorkItemId] }),
      ]);
      setWorkItemWorkspaceTab("external_cli");
    },
    onError: (error) => setActionError(String(error)),
  });

  const planApprovalMutation = useMutation({
    mutationFn: async () => {
      if (!selectedWorkItemId || !workflowRunId) {
        throw new Error("No workflow run available for plan approval.");
      }
      await approveWorkItemPlan(selectedWorkItemId, "Plan approved from story workspace");
      await handleWorkflowUserAction({
        workflowRunId,
        action: "approve",
        notes: "Plan approved from story workspace",
      });
    },
    onSuccess: async () => {
      setActionError(null);
      await invalidateTasks();
    },
    onError: (error) => setActionError(String(error)),
  });

  const planRejectMutation = useMutation({
    mutationFn: async () => {
      if (!selectedWorkItemId || !workflowRunId) {
        throw new Error("No workflow run available for plan rejection.");
      }
      await rejectWorkItemPlan(selectedWorkItemId, "Plan rejected from story workspace");
      await handleWorkflowUserAction({
        workflowRunId,
        action: "reject",
        notes: "Plan rejected from story workspace",
      });
    },
    onSuccess: async () => {
      setActionError(null);
      await invalidateTasks();
    },
    onError: (error) => setActionError(String(error)),
  });

  const testReviewApproveMutation = useMutation({
    mutationFn: async () => {
      if (!selectedWorkItemId || !workflowRunId) {
        throw new Error("No workflow run available for test review.");
      }
      await approveWorkItemTestReview(selectedWorkItemId, "Test review approved from story workspace");
      await handleWorkflowUserAction({
        workflowRunId,
        action: "approve",
        notes: "Test review approved from story workspace",
      });
    },
    onSuccess: async () => {
      setActionError(null);
      await invalidateTasks();
    },
    onError: (error) => setActionError(String(error)),
  });

  const testReviewRejectMutation = useMutation({
    mutationFn: async () => {
      if (!workflowRunId) {
        throw new Error("No workflow run available for test review rejection.");
      }
      await handleWorkflowUserAction({
        workflowRunId,
        action: "reject",
        notes: "Test review rejected from story workspace",
      });
    },
    onSuccess: async () => {
      setActionError(null);
      await invalidateTasks();
    },
    onError: (error) => setActionError(String(error)),
  });

  const failWorkflowRunMutation = useMutation({
    mutationFn: async () => {
      if (!workflowRunId) {
        throw new Error("No workflow run available.");
      }
      await markWorkflowRunFailed(
        workflowRunId,
        "Marked failed from story review due to stale execution",
      );
    },
    onSuccess: async () => {
      setActionError(null);
      await invalidateTasks();
    },
    onError: (error) => setActionError(String(error)),
  });

  const restartWorkflowMutation = useMutation({
    mutationFn: async () => {
      if (!workflowRunId) {
        throw new Error("No workflow run available.");
      }
      return restartWorkflowRun(workflowRunId);
    },
    onSuccess: async (newWorkflowRunId) => {
      setActiveWorkflowRunId(newWorkflowRunId);
      setActionError(null);
      await invalidateTasks();
    },
    onError: (error) => setActionError(String(error)),
  });

  const reorderWorkItemsMutation = useMutation({
    mutationFn: (orderedIds: string[]) => reorderWorkItems(orderedIds),
    onSuccess: async () => invalidateTasks(),
  });

  return {
    approveMutation,
    rejectMutation,
    workflowMutation,
    externalCliMutation,
    planApprovalMutation,
    planRejectMutation,
    testReviewApproveMutation,
    testReviewRejectMutation,
    failWorkflowRunMutation,
    restartWorkflowMutation,
    reorderWorkItemsMutation,
  };
}
