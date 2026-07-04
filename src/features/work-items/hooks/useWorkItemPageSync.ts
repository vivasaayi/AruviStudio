import { useEffect, type Dispatch, type RefObject, type SetStateAction } from "react";

import type { WorkItem } from "../../../lib/types";
import type { WorkItemEditDraftState } from "../components/WorkItemFormModals";

type WorkItemWorkspaceTab = "backlog" | "detail" | "external_cli" | "review";

type WorkItemPageSyncInput = {
  productsLoading: boolean;
  activeProductId: string | null;
  selectedProductId: string | null;
  setActiveProduct: (productId: string | null) => void;
  selectedProductScopeId: string | null;
  activeNodeId: string | null;
  activeNodeType: string | null;
  statusFilter: string;
  setWorkItemPageIndex: Dispatch<SetStateAction<number>>;
  setSelectedBacklogItemIds: Dispatch<SetStateAction<string[]>>;
  workItemPageIndex: number;
  backlogViewportRef: RefObject<HTMLDivElement | null>;
  setBacklogScrollTop: Dispatch<SetStateAction<number>>;
  setBacklogViewportHeight: Dispatch<SetStateAction<number>>;
  workItemWorkspaceTab: WorkItemWorkspaceTab;
  selectedWorkItemId: string | null;
  activeWorkItemId: string | null;
  setActiveWorkItem: (workItemId: string | null) => void;
  activeProductAreaId: string | null;
  activeCapabilityId: string | null;
  setActionError: Dispatch<SetStateAction<string | null>>;
  setActionInfo: Dispatch<SetStateAction<string | null>>;
  setActiveWorkflowRunId: Dispatch<SetStateAction<string | null>>;
  setSelectedArtifactStage: Dispatch<SetStateAction<string | null>>;
  setOpenOverflowWorkItemId: Dispatch<SetStateAction<string | null>>;
  selectedWorkItem: WorkItem | undefined;
  setWorkItemDraft: Dispatch<SetStateAction<WorkItemEditDraftState>>;
  filteredWorkItems: WorkItem[];
  setWorkItemOrderIds: Dispatch<SetStateAction<string[]>>;
  showCreateForm: boolean;
  workItemCreateDialogOpen: boolean;
  setFormError: Dispatch<SetStateAction<string | null>>;
  setSelectedExternalCliRunId: Dispatch<SetStateAction<string | null>>;
};

export function useWorkItemPageSync({
  productsLoading,
  activeProductId,
  selectedProductId,
  setActiveProduct,
  selectedProductScopeId,
  activeNodeId,
  activeNodeType,
  statusFilter,
  setWorkItemPageIndex,
  setSelectedBacklogItemIds,
  workItemPageIndex,
  backlogViewportRef,
  setBacklogScrollTop,
  setBacklogViewportHeight,
  workItemWorkspaceTab,
  selectedWorkItemId,
  activeWorkItemId,
  setActiveWorkItem,
  activeProductAreaId,
  activeCapabilityId,
  setActionError,
  setActionInfo,
  setActiveWorkflowRunId,
  setSelectedArtifactStage,
  setOpenOverflowWorkItemId,
  selectedWorkItem,
  setWorkItemDraft,
  filteredWorkItems,
  setWorkItemOrderIds,
  showCreateForm,
  workItemCreateDialogOpen,
  setFormError,
  setSelectedExternalCliRunId,
}: WorkItemPageSyncInput) {
  useEffect(() => {
    if (productsLoading) {
      return;
    }
    if (activeProductId !== selectedProductId) {
      setActiveProduct(selectedProductId);
    }
  }, [activeProductId, productsLoading, selectedProductId, setActiveProduct]);

  useEffect(() => {
    setWorkItemPageIndex(0);
    setSelectedBacklogItemIds([]);
  }, [selectedProductScopeId, activeNodeId, activeNodeType, statusFilter, setSelectedBacklogItemIds, setWorkItemPageIndex]);

  useEffect(() => {
    setBacklogScrollTop(0);
    backlogViewportRef.current?.scrollTo({ top: 0 });
  }, [
    selectedProductScopeId,
    activeNodeId,
    activeNodeType,
    statusFilter,
    workItemPageIndex,
    backlogViewportRef,
    setBacklogScrollTop,
  ]);

  useEffect(() => {
    if (workItemWorkspaceTab !== "backlog") {
      return;
    }
    const updateViewportHeight = () => {
      const nextHeight = backlogViewportRef.current?.clientHeight;
      if (nextHeight && nextHeight > 0) {
        setBacklogViewportHeight(nextHeight);
      }
    };
    updateViewportHeight();
    window.addEventListener("resize", updateViewportHeight);
    return () => window.removeEventListener("resize", updateViewportHeight);
  }, [
    workItemWorkspaceTab,
    selectedProductScopeId,
    activeNodeId,
    activeNodeType,
    statusFilter,
    workItemPageIndex,
    backlogViewportRef,
    setBacklogViewportHeight,
  ]);

  useEffect(() => {
    if (selectedWorkItemId !== activeWorkItemId) {
      setActiveWorkItem(selectedWorkItemId);
    }
  }, [activeWorkItemId, selectedWorkItemId, setActiveWorkItem]);

  useEffect(() => {
    setActionError(null);
    setActionInfo(null);
    setActiveWorkflowRunId(null);
    setSelectedArtifactStage(null);
    setOpenOverflowWorkItemId(null);
  }, [
    activeProductId,
    activeProductAreaId,
    activeCapabilityId,
    setActionError,
    setActionInfo,
    setActiveWorkflowRunId,
    setSelectedArtifactStage,
    setOpenOverflowWorkItemId,
  ]);

  useEffect(() => {
    if (selectedWorkItem) {
      setWorkItemDraft({
        title: selectedWorkItem.title,
        description: selectedWorkItem.description,
        status: selectedWorkItem.status,
        problemStatement: selectedWorkItem.problem_statement,
        acceptanceCriteria: selectedWorkItem.acceptance_criteria,
        constraints: selectedWorkItem.constraints,
      });
    }
  }, [selectedWorkItem, setWorkItemDraft]);

  useEffect(() => {
    setWorkItemOrderIds(filteredWorkItems.map((workItem) => workItem.id));
  }, [filteredWorkItems, setWorkItemOrderIds]);

  useEffect(() => {
    if (showCreateForm || workItemCreateDialogOpen) {
      setFormError(null);
    }
  }, [showCreateForm, workItemCreateDialogOpen, setFormError]);

  useEffect(() => {
    setActionError(null);
    setActionInfo(null);
  }, [selectedWorkItemId, setActionError, setActionInfo]);

  useEffect(() => {
    setOpenOverflowWorkItemId(null);
  }, [selectedWorkItemId, workItemWorkspaceTab, setOpenOverflowWorkItemId]);

  useEffect(() => {
    setActiveWorkflowRunId(null);
  }, [selectedWorkItemId, setActiveWorkflowRunId]);

  useEffect(() => {
    setSelectedExternalCliRunId(null);
  }, [selectedWorkItemId, setSelectedExternalCliRunId]);

  useEffect(() => {
    setSelectedArtifactStage(null);
  }, [selectedWorkItemId, setSelectedArtifactStage]);
}
