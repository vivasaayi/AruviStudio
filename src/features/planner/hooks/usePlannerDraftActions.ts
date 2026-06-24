import type { Dispatch, RefObject, SetStateAction } from "react";
import type { UseMutationResult } from "@tanstack/react-query";

import {
  collectTreeNodeIds,
  formatDraftChildTypeLabel,
  type DraftEditOperation,
  type PlannerMutationResult,
  type PlannerTreeNode,
} from "../lib/plannerPageModel";
import type { PlannerDraftChildType } from "../../../lib/types";

type PlannerDraftActionsInput = {
  draftTreeNodes: PlannerTreeNode[];
  setExpandedDraftNodeIds: Dispatch<SetStateAction<string[]>>;
  setDraft: Dispatch<SetStateAction<string>>;
  composerRef: RefObject<HTMLTextAreaElement | null>;
  selectedDraftNode: PlannerTreeNode | null;
  renameDraftName: string;
  draftChildType: PlannerDraftChildType;
  draftChildName: string;
  setDraftChildName: Dispatch<SetStateAction<string>>;
  draftChildSummary: string;
  setDraftChildSummary: Dispatch<SetStateAction<string>>;
  allowedDraftChildTypes: PlannerDraftChildType[];
  isPlannerBusy: boolean;
  setDraftEditError: Dispatch<SetStateAction<string | null>>;
  setDraftEditMessage: Dispatch<SetStateAction<string | null>>;
  draftEditMutation: UseMutationResult<PlannerMutationResult, Error, DraftEditOperation>;
};

export function usePlannerDraftActions({
  draftTreeNodes,
  setExpandedDraftNodeIds,
  setDraft,
  composerRef,
  selectedDraftNode,
  renameDraftName,
  draftChildType,
  draftChildName,
  setDraftChildName,
  draftChildSummary,
  setDraftChildSummary,
  allowedDraftChildTypes,
  isPlannerBusy,
  setDraftEditError,
  setDraftEditMessage,
  draftEditMutation,
}: PlannerDraftActionsInput) {
  const toggleDraftNodeExpanded = (nodeId: string) => {
    setExpandedDraftNodeIds((current) =>
      current.includes(nodeId) ? current.filter((value) => value !== nodeId) : [...current, nodeId],
    );
  };

  const expandAllDraftNodes = () => {
    setExpandedDraftNodeIds(collectTreeNodeIds(draftTreeNodes));
  };

  const collapseAllDraftNodes = () => {
    setExpandedDraftNodeIds([]);
  };

  const applyPromptSuggestion = (prompt: string) => {
    setDraft(prompt);
    composerRef.current?.focus();
  };

  const renameSelectedDraftNode = async () => {
    if (!selectedDraftNode || !renameDraftName.trim() || isPlannerBusy) {
      return;
    }
    setDraftEditError(null);
    setDraftEditMessage(null);
    try {
      await draftEditMutation.mutateAsync({
        kind: "rename",
        nodeId: selectedDraftNode.id,
        name: renameDraftName.trim(),
      });
      setDraftEditMessage(`Renamed to "${renameDraftName.trim()}".`);
    } catch {
      // Error state is handled by the mutation.
    }
  };

  const addChildToSelectedDraftNode = async () => {
    if (!selectedDraftNode || !draftChildName.trim() || allowedDraftChildTypes.length === 0 || isPlannerBusy) {
      return;
    }
    setDraftEditError(null);
    setDraftEditMessage(null);
    try {
      await draftEditMutation.mutateAsync({
        kind: "add_child",
        parentNodeId: selectedDraftNode.id,
        childType: draftChildType,
        name: draftChildName.trim(),
        summary: draftChildSummary.trim() || undefined,
      });
      setDraftChildName("");
      setDraftChildSummary("");
      setDraftEditMessage(`Added ${formatDraftChildTypeLabel(draftChildType).toLowerCase()} "${draftChildName.trim()}".`);
    } catch {
      // Error state is handled by the mutation.
    }
  };

  const deleteSelectedDraftNode = async () => {
    if (!selectedDraftNode || isPlannerBusy) {
      return;
    }
    setDraftEditError(null);
    setDraftEditMessage(null);
    const deletedLabel = selectedDraftNode.label;
    try {
      await draftEditMutation.mutateAsync({
        kind: "delete",
        nodeId: selectedDraftNode.id,
      });
      setDraftEditMessage(`Removed "${deletedLabel}" from the design.`);
    } catch {
      // Error state is handled by the mutation.
    }
  };

  return {
    toggleDraftNodeExpanded,
    expandAllDraftNodes,
    collapseAllDraftNodes,
    applyPromptSuggestion,
    renameSelectedDraftNode,
    addChildToSelectedDraftNode,
    deleteSelectedDraftNode,
  };
}
