import { useEffect, useState } from "react";

import type { PlannerDraftChildType } from "../../../lib/types";
import type { PlannerTreeNode } from "../lib/plannerPageModel";

type PlannerDraftEditorStateInput = {
  selectedDraftNode: PlannerTreeNode | null;
  selectedDraftNodeId: string | null;
  allowedDraftChildTypes: PlannerDraftChildType[];
};

export function usePlannerDraftEditorState({
  selectedDraftNode,
  selectedDraftNodeId,
  allowedDraftChildTypes,
}: PlannerDraftEditorStateInput) {
  const [renameDraftName, setRenameDraftName] = useState("");
  const [draftChildType, setDraftChildType] = useState<PlannerDraftChildType>("product_area");
  const [draftChildName, setDraftChildName] = useState("");
  const [draftChildSummary, setDraftChildSummary] = useState("");
  const [draftEditError, setDraftEditError] = useState<string | null>(null);
  const [draftEditMessage, setDraftEditMessage] = useState<string | null>(null);

  useEffect(() => {
    setRenameDraftName(selectedDraftNode?.label ?? "");
    setDraftEditError(null);
    setDraftEditMessage(null);
  }, [selectedDraftNodeId, selectedDraftNode?.label]);

  useEffect(() => {
    if (allowedDraftChildTypes.length === 0) {
      return;
    }
    if (!allowedDraftChildTypes.includes(draftChildType)) {
      setDraftChildType(allowedDraftChildTypes[0]);
    }
  }, [allowedDraftChildTypes, draftChildType]);

  return {
    renameDraftName,
    setRenameDraftName,
    draftChildType,
    setDraftChildType,
    draftChildName,
    setDraftChildName,
    draftChildSummary,
    setDraftChildSummary,
    draftEditError,
    setDraftEditError,
    draftEditMessage,
    setDraftEditMessage,
  };
}
