import type { Dispatch, SetStateAction } from "react";

import { approveWorkItem, rejectWorkItem } from "../../../lib/tauri";

type BacklogApprovalAction = "approve" | "reject";

type WorkItemBacklogApprovalActionsInput = {
  selectedBacklogItemIds: string[];
  setSelectedBacklogItemIds: Dispatch<SetStateAction<string[]>>;
  pendingRowActionIds: string[];
  setPendingRowActionIds: Dispatch<SetStateAction<string[]>>;
  bulkActionInFlight: BacklogApprovalAction | null;
  setBulkActionInFlight: Dispatch<SetStateAction<BacklogApprovalAction | null>>;
  setActionError: Dispatch<SetStateAction<string | null>>;
  invalidateTasks: () => Promise<void>;
};

export function useWorkItemBacklogApprovalActions({
  selectedBacklogItemIds,
  setSelectedBacklogItemIds,
  pendingRowActionIds,
  setPendingRowActionIds,
  bulkActionInFlight,
  setBulkActionInFlight,
  setActionError,
  invalidateTasks,
}: WorkItemBacklogApprovalActionsInput) {
  const isRowActionPending = (workItemId: string) => pendingRowActionIds.includes(workItemId);

  const runRowApprovalAction = async (workItemId: string, action: BacklogApprovalAction) => {
    if (isRowActionPending(workItemId)) {
      return;
    }
    setPendingRowActionIds((current) => [...current, workItemId]);
    setActionError(null);
    try {
      if (action === "approve") {
        await approveWorkItem(workItemId, "Approved from backlog row");
      } else {
        await rejectWorkItem(workItemId, "Rejected from backlog row");
      }
      await invalidateTasks();
    } catch (error) {
      setActionError(String(error));
    } finally {
      setPendingRowActionIds((current) => current.filter((id) => id !== workItemId));
    }
  };

  const runBulkApprovalAction = async (action: BacklogApprovalAction) => {
    if (selectedBacklogItemIds.length === 0 || bulkActionInFlight) {
      return;
    }
    setBulkActionInFlight(action);
    setActionError(null);
    try {
      for (const workItemId of selectedBacklogItemIds) {
        if (action === "approve") {
          await approveWorkItem(workItemId, "Approved from backlog");
        } else {
          await rejectWorkItem(workItemId, "Rejected from backlog");
        }
      }
      setSelectedBacklogItemIds([]);
      await invalidateTasks();
    } catch (error) {
      setActionError(String(error));
    } finally {
      setBulkActionInFlight(null);
    }
  };

  return {
    isRowActionPending,
    runRowApprovalAction,
    runBulkApprovalAction,
  };
}
