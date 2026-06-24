import type { Dispatch, SetStateAction } from "react";
import { useMutation } from "@tanstack/react-query";

import {
  createWorkItem,
  deleteWorkItem,
  getSubWorkItems,
  updateWorkItem,
} from "../../../lib/tauri";
import type { HierarchyTreeNode, WorkItem } from "../../../lib/types";
import {
  emptyWorkItemDraft,
  SUB_WORK_ITEM_PAGE_SIZE,
  type WorkItemDraftState,
} from "../lib/productListPageState";

type WorkItemDialogMode = "closed" | "create" | "edit";
type DeleteWorkItemCandidate = {
  workItem: WorkItem;
  kind: "story" | "task";
};

type ProductManagementWorkItemMutationsInput = {
  selectedProductId: string | null;
  selectedManagementFeatureNode: HierarchyTreeNode | null;
  selectedManagementStory: WorkItem | null;
  setSelectedManagementStoryId: Dispatch<SetStateAction<string | null>>;
  setActiveWorkItem: (workItemId: string | null) => void;
  storyDraft: WorkItemDraftState;
  setStoryDraft: Dispatch<SetStateAction<WorkItemDraftState>>;
  taskDraft: WorkItemDraftState;
  setTaskDraft: Dispatch<SetStateAction<WorkItemDraftState>>;
  editingStory: WorkItem | null;
  setEditingStory: Dispatch<SetStateAction<WorkItem | null>>;
  editingTask: WorkItem | null;
  setEditingTask: Dispatch<SetStateAction<WorkItem | null>>;
  setStoryDialogMode: Dispatch<SetStateAction<WorkItemDialogMode>>;
  setTaskDialogMode: Dispatch<SetStateAction<WorkItemDialogMode>>;
  setDeleteWorkItemCandidate: Dispatch<SetStateAction<DeleteWorkItemCandidate | null>>;
  setDeleteWorkItemConfirmName: Dispatch<SetStateAction<string>>;
  setDeleteWorkItemConfirmChecked: Dispatch<SetStateAction<boolean>>;
  setFormError: Dispatch<SetStateAction<string | null>>;
  invalidateTasks: () => Promise<void>;
};

export function useProductManagementWorkItemMutations({
  selectedProductId,
  selectedManagementFeatureNode,
  selectedManagementStory,
  setSelectedManagementStoryId,
  setActiveWorkItem,
  storyDraft,
  setStoryDraft,
  taskDraft,
  setTaskDraft,
  editingStory,
  setEditingStory,
  editingTask,
  setEditingTask,
  setStoryDialogMode,
  setTaskDialogMode,
  setDeleteWorkItemCandidate,
  setDeleteWorkItemConfirmName,
  setDeleteWorkItemConfirmChecked,
  setFormError,
  invalidateTasks,
}: ProductManagementWorkItemMutationsInput) {
  const createManagementStoryMutation = useMutation({
    mutationFn: () => {
      if (!selectedProductId || !selectedManagementFeatureNode) {
        throw new Error("Select a feature before adding a story.");
      }
      return createWorkItem({
        productId: selectedProductId,
        productAreaId: selectedManagementFeatureNode.product_area_id ?? undefined,
        capabilityId: selectedManagementFeatureNode.capability_id ?? undefined,
        sourceNodeId: selectedManagementFeatureNode.id,
        sourceNodeType: selectedManagementFeatureNode.node_type,
        title: storyDraft.title.trim(),
        problemStatement: storyDraft.problemStatement.trim(),
        description: storyDraft.description.trim(),
        acceptanceCriteria: storyDraft.acceptanceCriteria.trim(),
        constraints: storyDraft.constraints.trim(),
        workItemType: "story",
        priority: storyDraft.priority,
        complexity: storyDraft.complexity,
      });
    },
    onSuccess: async (createdStory) => {
      await invalidateTasks();
      setSelectedManagementStoryId(createdStory.id);
      setActiveWorkItem(createdStory.id);
      setStoryDraft(emptyWorkItemDraft);
      setStoryDialogMode("closed");
      setFormError(null);
    },
    onError: (error) => setFormError(String(error)),
  });

  const updateManagementStoryMutation = useMutation({
    mutationFn: () => {
      if (!editingStory) {
        throw new Error("Select a story before editing.");
      }
      return updateWorkItem({
        id: editingStory.id,
        title: storyDraft.title.trim(),
        status: storyDraft.status,
        problemStatement: storyDraft.problemStatement.trim(),
        description: storyDraft.description.trim(),
        acceptanceCriteria: storyDraft.acceptanceCriteria.trim(),
        constraints: storyDraft.constraints.trim(),
      });
    },
    onSuccess: async (updatedStory) => {
      await invalidateTasks();
      setSelectedManagementStoryId(updatedStory.id);
      setActiveWorkItem(updatedStory.id);
      setEditingStory(null);
      setStoryDraft(emptyWorkItemDraft);
      setStoryDialogMode("closed");
      setFormError(null);
    },
    onError: (error) => setFormError(String(error)),
  });

  const createManagementTaskMutation = useMutation({
    mutationFn: () => {
      if (!selectedProductId || !selectedManagementFeatureNode || !selectedManagementStory) {
        throw new Error("Select a story before adding a task.");
      }
      return createWorkItem({
        productId: selectedProductId,
        productAreaId: selectedManagementFeatureNode.product_area_id ?? undefined,
        capabilityId: selectedManagementFeatureNode.capability_id ?? undefined,
        sourceNodeId: selectedManagementFeatureNode.id,
        sourceNodeType: selectedManagementFeatureNode.node_type,
        parentWorkItemId: selectedManagementStory.id,
        title: taskDraft.title.trim(),
        problemStatement: taskDraft.problemStatement.trim(),
        description: taskDraft.description.trim(),
        acceptanceCriteria: taskDraft.acceptanceCriteria.trim(),
        constraints: taskDraft.constraints.trim(),
        workItemType: "task",
        priority: taskDraft.priority,
        complexity: taskDraft.complexity,
      });
    },
    onSuccess: async () => {
      await invalidateTasks();
      setTaskDraft(emptyWorkItemDraft);
      setTaskDialogMode("closed");
      setFormError(null);
    },
    onError: (error) => setFormError(String(error)),
  });

  const updateManagementTaskMutation = useMutation({
    mutationFn: () => {
      if (!editingTask) {
        throw new Error("Select a task before editing.");
      }
      return updateWorkItem({
        id: editingTask.id,
        title: taskDraft.title.trim(),
        status: taskDraft.status,
        problemStatement: taskDraft.problemStatement.trim(),
        description: taskDraft.description.trim(),
        acceptanceCriteria: taskDraft.acceptanceCriteria.trim(),
        constraints: taskDraft.constraints.trim(),
      });
    },
    onSuccess: async () => {
      await invalidateTasks();
      setEditingTask(null);
      setTaskDraft(emptyWorkItemDraft);
      setTaskDialogMode("closed");
      setFormError(null);
    },
    onError: (error) => setFormError(String(error)),
  });

  const deleteManagementWorkItemMutation = useMutation({
    mutationFn: async (candidate: DeleteWorkItemCandidate) => {
      if (candidate.kind === "story") {
        for (;;) {
          const childTasks = await getSubWorkItems(candidate.workItem.id, {
            limit: SUB_WORK_ITEM_PAGE_SIZE,
            offset: 0,
          });
          if (childTasks.length === 0) {
            break;
          }
          await Promise.all(childTasks.map((workItem) => deleteWorkItem(workItem.id)));
          if (childTasks.length < SUB_WORK_ITEM_PAGE_SIZE) {
            break;
          }
        }
      }
      await deleteWorkItem(candidate.workItem.id);
    },
    onSuccess: async () => {
      await invalidateTasks();
      setDeleteWorkItemCandidate(null);
      setDeleteWorkItemConfirmName("");
      setDeleteWorkItemConfirmChecked(false);
      setSelectedManagementStoryId(null);
      setFormError(null);
    },
    onError: (error) => setFormError(String(error)),
  });

  return {
    createManagementStoryMutation,
    updateManagementStoryMutation,
    createManagementTaskMutation,
    updateManagementTaskMutation,
    deleteManagementWorkItemMutation,
  };
}
