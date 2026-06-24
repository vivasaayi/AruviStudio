import type { Dispatch, SetStateAction } from "react";
import { useMutation, type QueryClient } from "@tanstack/react-query";

import {
  createWorkItem,
  deleteWorkItem,
  updateWorkItem,
} from "../../../lib/tauri";
import type { WorkItem, WorkItemPage } from "../../../lib/types";
import {
  WORK_ITEM_PAGE_SIZE,
} from "../lib/workItemListPageHelpers";
import type {
  WorkItemCreateFormState,
  WorkItemEditDraftState,
} from "../components/WorkItemFormModals";

type WorkItemCrudMutationsInput = {
  queryClient: QueryClient;
  activeProductId: string | null;
  activeProductAreaId: string | null;
  activeCapabilityId: string | null;
  activeNodeId: string | null;
  activeNodeType: "product_area" | "capability" | null;
  selectedWorkItemId: string | null;
  createForm: WorkItemCreateFormState;
  setCreateForm: Dispatch<SetStateAction<WorkItemCreateFormState>>;
  workItemDraft: WorkItemEditDraftState;
  setWorkItemOrderIds: Dispatch<SetStateAction<string[]>>;
  setActiveWorkItem: (workItemId: string | null) => void;
  setShowCreateForm: Dispatch<SetStateAction<boolean>>;
  closeWorkItemCreateDialog: () => void;
  setWorkItemWorkspaceTab: (tab: "backlog" | "detail" | "external_cli" | "review") => void;
  setIsEditingWorkItem: Dispatch<SetStateAction<boolean>>;
  setFormError: Dispatch<SetStateAction<string | null>>;
  workItemsQueryKey: readonly unknown[];
  activeProductSidebarQueryKey: readonly unknown[];
  workItemPageIndex: number;
  invalidateTasks: () => Promise<void>;
};

const emptyCreateForm: WorkItemCreateFormState = {
  title: "",
  problemStatement: "",
  description: "",
  acceptanceCriteria: "",
  constraints: "",
  workItemType: "story",
  priority: "medium",
  complexity: "medium",
  parentWorkItemId: null,
};

export function useWorkItemCrudMutations({
  queryClient,
  activeProductId,
  activeProductAreaId,
  activeCapabilityId,
  activeNodeId,
  activeNodeType,
  selectedWorkItemId,
  createForm,
  setCreateForm,
  workItemDraft,
  setWorkItemOrderIds,
  setActiveWorkItem,
  setShowCreateForm,
  closeWorkItemCreateDialog,
  setWorkItemWorkspaceTab,
  setIsEditingWorkItem,
  setFormError,
  workItemsQueryKey,
  activeProductSidebarQueryKey,
  workItemPageIndex,
  invalidateTasks,
}: WorkItemCrudMutationsInput) {
  const createMutation = useMutation({
    mutationFn: () =>
      createWorkItem({
        productId: activeProductId || "",
        productAreaId: activeProductAreaId ?? undefined,
        capabilityId: activeCapabilityId ?? undefined,
        sourceNodeId: activeNodeId ?? undefined,
        sourceNodeType: activeNodeType ?? undefined,
        title: createForm.title,
        problemStatement: createForm.problemStatement,
        description: createForm.description,
        acceptanceCriteria: createForm.acceptanceCriteria,
        constraints: createForm.constraints,
        workItemType: createForm.workItemType,
        priority: createForm.priority,
        complexity: createForm.complexity,
        parentWorkItemId: createForm.parentWorkItemId ?? undefined,
      }),
    onSuccess: async (createdWorkItem) => {
      queryClient.setQueryData<WorkItemPage | undefined>(workItemsQueryKey, (current) =>
        current
          ? {
              ...current,
              items: current.items.length < current.limit ? [...current.items, createdWorkItem] : current.items,
              has_more: current.has_more || current.items.length >= current.limit,
            }
          : {
              items: [createdWorkItem],
              limit: WORK_ITEM_PAGE_SIZE,
              offset: workItemPageIndex * WORK_ITEM_PAGE_SIZE,
              has_more: false,
            },
      );
      queryClient.setQueryData<WorkItem[] | undefined>(activeProductSidebarQueryKey, (current) =>
        current ? [...current, createdWorkItem] : [createdWorkItem],
      );
      setWorkItemOrderIds((current) => (current.includes(createdWorkItem.id) ? current : [...current, createdWorkItem.id]));
      setActiveWorkItem(createdWorkItem.id);
      await invalidateTasks();
      setCreateForm(emptyCreateForm);
      setShowCreateForm(false);
      closeWorkItemCreateDialog();
      setWorkItemWorkspaceTab("detail");
    },
    onError: (error) => setFormError(String(error)),
  });

  const updateWorkItemMutation = useMutation({
    mutationFn: () =>
      updateWorkItem({
        id: selectedWorkItemId!,
        title: workItemDraft.title,
        description: workItemDraft.description,
        status: workItemDraft.status,
        problemStatement: workItemDraft.problemStatement,
        acceptanceCriteria: workItemDraft.acceptanceCriteria,
        constraints: workItemDraft.constraints,
      }),
    onSuccess: async () => {
      await invalidateTasks();
      setIsEditingWorkItem(false);
    },
    onError: (error) => setFormError(String(error)),
  });

  const deleteMutation = useMutation({
    mutationFn: (id: string) => deleteWorkItem(id),
    onSuccess: async (_, deletedId) => {
      await invalidateTasks();
      if (selectedWorkItemId === deletedId) {
        setActiveWorkItem(null);
      }
    },
  });

  return {
    createMutation,
    updateWorkItemMutation,
    deleteMutation,
  };
}
