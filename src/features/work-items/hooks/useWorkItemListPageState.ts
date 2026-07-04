import { useRef, useState } from "react";
import type { Artifact } from "../../../lib/types";
import type {
  WorkItemCreateFormState,
  WorkItemEditDraftState,
} from "../components/WorkItemFormModals";

export function useWorkItemListPageState() {
  const [statusFilter, setStatusFilter] = useState("");
  const [workItemPageIndex, setWorkItemPageIndex] = useState(0);
  const backlogViewportRef = useRef<HTMLDivElement | null>(null);
  const [backlogScrollTop, setBacklogScrollTop] = useState(0);
  const [backlogViewportHeight, setBacklogViewportHeight] = useState(520);
  const [showCreateForm, setShowCreateForm] = useState(false);
  const [isEditingWorkItem, setIsEditingWorkItem] = useState(false);
  const [draggedWorkItemId, setDraggedWorkItemId] = useState<string | null>(null);
  const [workItemOrderIds, setWorkItemOrderIds] = useState<string[]>([]);
  const [formError, setFormError] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [actionInfo, setActionInfo] = useState<string | null>(null);
  const [activeWorkflowRunId, setActiveWorkflowRunId] = useState<string | null>(null);
  const [selectedExternalCliRunId, setSelectedExternalCliRunId] = useState<string | null>(null);
  const [selectedArtifactStage, setSelectedArtifactStage] = useState<string | null>(null);
  const [artifactModalArtifact, setArtifactModalArtifact] = useState<Artifact | null>(null);
  const [openOverflowWorkItemId, setOpenOverflowWorkItemId] = useState<string | null>(null);
  const [selectedBacklogItemIds, setSelectedBacklogItemIds] = useState<string[]>([]);
  const [pendingRowActionIds, setPendingRowActionIds] = useState<string[]>([]);
  const [bulkActionInFlight, setBulkActionInFlight] = useState<"approve" | "reject" | null>(null);
  const [createForm, setCreateForm] = useState<WorkItemCreateFormState>({
    title: "",
    problemStatement: "",
    description: "",
    acceptanceCriteria: "",
    constraints: "",
    workItemType: "story",
    priority: "medium",
    complexity: "medium",
    parentWorkItemId: null,
  });
  const [workItemDraft, setWorkItemDraft] = useState<WorkItemEditDraftState>({
    title: "",
    description: "",
    status: "draft",
    problemStatement: "",
    acceptanceCriteria: "",
    constraints: "",
  });

  return {
    statusFilter, setStatusFilter, workItemPageIndex, setWorkItemPageIndex,
    backlogViewportRef, backlogScrollTop, setBacklogScrollTop,
    backlogViewportHeight, setBacklogViewportHeight, showCreateForm, setShowCreateForm,
    isEditingWorkItem, setIsEditingWorkItem, draggedWorkItemId, setDraggedWorkItemId,
    workItemOrderIds, setWorkItemOrderIds, formError, setFormError,
    actionError, setActionError, actionInfo, setActionInfo,
    activeWorkflowRunId, setActiveWorkflowRunId, selectedExternalCliRunId,
    setSelectedExternalCliRunId, selectedArtifactStage, setSelectedArtifactStage,
    artifactModalArtifact, setArtifactModalArtifact, openOverflowWorkItemId,
    setOpenOverflowWorkItemId, selectedBacklogItemIds, setSelectedBacklogItemIds,
    pendingRowActionIds, setPendingRowActionIds, bulkActionInFlight,
    setBulkActionInFlight, createForm, setCreateForm, workItemDraft, setWorkItemDraft,
  };
}
