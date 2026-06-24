import React, { useEffect, useMemo, useRef, useState } from "react";
import { useMutation, useQueries, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  assignWorkItemWorkspace,
  approveWorkItem,
  approveWorkItemPlan,
  approveWorkItemTestReview,
  createWorkItem,
  createLocalWorkspace,
  deleteWorkItem,
  getLatestWorkflowRunForWorkItem,
  getWorkflowHistory,
  handleWorkflowUserAction,
  getSubWorkItems,
  getWorkItem,
  getWorkItemApprovals,
  listWorkItemArtifacts,
  listWorkItemFindings,
  listWorkItemsPage,
  listAgentDefinitions,
  listAgentModelCallsForWorkflow,
  listAgentRunsForWorkflow,
  listCapabilities,
  invokeExternalCliForWorkItem,
  listExternalCliRunEvents,
  listExternalCliRunsForWorkItem,
  listAgentModelBindings,
  listAgentTeams,
  listModelDefinitions,
  listProducts,
  listProductAreas,
  listProviders,
  listRepositories,
  listTeamAssignments,
  listTeamMemberships,
  listWorkflowStagePolicies,
  rejectWorkItemPlan,
  readArtifactContent,
  resolveRepositoryForWorkItem,
  reorderWorkItems,
  markWorkflowRunFailed,
  rejectWorkItem,
  restartWorkflowRun,
  startWorkItemWorkflow,
  updateWorkItem,
} from "../../../lib/tauri";
import { getHierarchyNodeKindLabel } from "../../../lib/hierarchyLabels";
import { useWorkspaceStore } from "../../../state/workspaceStore";
import { useUIStore } from "../../../state/uiStore";
import { WorkItemArtifactModal } from "../components/WorkItemArtifactModal";
import { WorkItemBacklogTab } from "../components/WorkItemBacklogTab";
import { WorkItemDetailTab } from "../components/WorkItemDetailTab";
import { WorkItemExternalCliTab } from "../components/WorkItemExternalCliTab";
import {
  WorkItemCreateModal,
  WorkItemEditModal,
  type WorkItemCreateFormState,
  type WorkItemEditDraftState,
} from "../components/WorkItemFormModals";
import { WorkItemReviewSummaryCards } from "../components/WorkItemReviewSummaryCards";
import { WorkItemReviewWorkflowCard } from "../components/WorkItemReviewWorkflowCard";
import { WorkItemWorkspaceReadinessCard } from "../components/WorkItemWorkspaceReadinessCard";
import { WorkItemWorkspaceAssignmentPanel } from "../components/WorkItemWorkspaceAssignmentPanel";
import { styles } from "../lib/workItemListPageStyles";
import {
  BACKLOG_OVERSCAN_ROWS,
  BACKLOG_ROW_ESTIMATED_HEIGHT,
  EXTERNAL_CLI_TRACE_LIMIT,
  SUB_WORK_ITEM_PAGE_SIZE,
  WORKFLOW_DAG_LANES,
  WORKFLOW_DAG_NODES,
  WORK_ITEM_PAGE_SIZE,
  buildWorkflowLaneStatusById,
  buildCapabilityPath,
  filterArtifactsForWorkflowStages,
  filterWorkflowHistoryForStages,
  findLatestAgentRunForStage,
  formatExternalCliTerminal,
  formatWorkflowElapsedLabel,
  getRunningAgentRunStartMs,
  groupArtifactsByAgentRunId,
  groupModelCallsByAgentRunId,
  isWorkflowRunStale,
  orderWorkItemsByIds,
  summarizeModelUsage,
  workItemBranchName,
  type ExternalCliProvider,
  type WorkspaceBranchMode,
} from "../lib/workItemListPageHelpers";
import { buildWorkItemWorkflowReadiness } from "../lib/workItemWorkflowReadiness";
import type {
  ExternalCliRunEvent,
  Approval,
  Artifact,
  Finding,
  WorkItem,
  WorkItemPage,
  WorkflowRun,
  Product,
  ProductArea,
  Capability,
  Repository,
} from "../../../lib/types";





export function WorkItemListPage() {
  const queryClient = useQueryClient();
  const {
    activeProductId,
    activeProductAreaId,
    activeCapabilityId,
    activeNodeId,
    activeNodeType,
    activeWorkItemId,
    setActiveProduct,
    setActiveWorkItem,
  } = useWorkspaceStore();
  const { workItemWorkspaceTab, setWorkItemWorkspaceTab, workItemCreateDialogOpen, openWorkItemCreateDialog, closeWorkItemCreateDialog, setActiveView } = useUIStore();

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
  const [isEditingWorkspace, setIsEditingWorkspace] = useState(false);
  const [workspaceRepositoryId, setWorkspaceRepositoryId] = useState("");
  const [workspaceBranchMode, setWorkspaceBranchMode] = useState<WorkspaceBranchMode>("default");
  const [workspaceBranchName, setWorkspaceBranchName] = useState("");
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
    parentWorkItemId: null as string | null,
  });
  const [workItemDraft, setWorkItemDraft] = useState<WorkItemEditDraftState>({
    title: "",
    description: "",
    status: "draft",
    problemStatement: "",
    acceptanceCriteria: "",
    constraints: "",
  });

  const { data: products, isLoading: productsLoading } = useQuery({
    queryKey: ["products"],
    queryFn: () => listProducts(),
  });
  const selectedProductId = (products ?? []).some((product) => product.id === activeProductId)
    ? activeProductId
    : ((products ?? [])[0]?.id ?? null);

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
  }, [selectedProductId, activeNodeId, activeNodeType, statusFilter]);

  useEffect(() => {
    setBacklogScrollTop(0);
    backlogViewportRef.current?.scrollTo({ top: 0 });
  }, [selectedProductId, activeNodeId, activeNodeType, statusFilter, workItemPageIndex]);

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
  }, [workItemWorkspaceTab, selectedProductId, activeNodeId, activeNodeType, statusFilter, workItemPageIndex]);

  const workItemsScopeQueryKey = ["workItems", selectedProductId, activeNodeId, activeNodeType, statusFilter] as const;
  const workItemsQueryKey = [...workItemsScopeQueryKey, workItemPageIndex] as const;
  const { data: workItemPage, isLoading } = useQuery({
    queryKey: workItemsQueryKey,
    queryFn: () =>
      listWorkItemsPage({
        productId: selectedProductId ?? undefined,
        sourceNodeId: activeNodeId ?? undefined,
        sourceNodeType: activeNodeType ?? undefined,
        status: statusFilter || undefined,
        limit: WORK_ITEM_PAGE_SIZE,
        offset: workItemPageIndex * WORK_ITEM_PAGE_SIZE,
      }),
    enabled: !!selectedProductId,
  });
  const workItems = workItemPage?.items ?? [];
  const { data: activeProductAreas = [] } = useQuery<ProductArea[]>({
    queryKey: ["workItemProductAreas", selectedProductId],
    queryFn: () => listProductAreas(selectedProductId!),
    enabled: !!selectedProductId,
  });
  const productAreaById = useMemo(
    () => new Map(activeProductAreas.map((productArea) => [productArea.id, productArea])),
    [activeProductAreas],
  );
  const productAreaIdsForCapabilityLookup = useMemo(() => {
    const ids = new Set<string>();
    if (activeProductAreaId && productAreaById.has(activeProductAreaId)) {
      ids.add(activeProductAreaId);
    }
    workItems.forEach((workItem) => {
      if (workItem.product_area_id && productAreaById.has(workItem.product_area_id)) {
        ids.add(workItem.product_area_id);
      }
    });
    return Array.from(ids).sort();
  }, [activeProductAreaId, productAreaById, workItems]);
  const productAreaCapabilityQueries = useQueries({
    queries: productAreaIdsForCapabilityLookup.map((productAreaId) => ({
      queryKey: ["workItemProductAreaCapabilities", productAreaId],
      queryFn: () => listCapabilities(productAreaId),
      enabled: !!selectedProductId,
    })),
  });
  const activeCapabilities = useMemo<Capability[]>(
    () => productAreaCapabilityQueries.flatMap((query) => query.data ?? []),
    [productAreaCapabilityQueries],
  );
  const capabilityById = useMemo(
    () => new Map(activeCapabilities.map((capability) => [capability.id, capability])),
    [activeCapabilities],
  );
  const { data: repositories = [] } = useQuery({ queryKey: ["repositories"], queryFn: listRepositories });
  const filteredWorkItems = useMemo(() => {
    if (!selectedProductId) {
      return [];
    }
    return workItems.filter((workItem) => workItem.product_id === selectedProductId);
  }, [selectedProductId, workItems]);

  const selectedWorkItemId = useMemo(() => {
    const activeIdInScope = activeWorkItemId && filteredWorkItems.some((workItem) => workItem.id === activeWorkItemId)
      ? activeWorkItemId
      : null;
    return activeIdInScope ?? filteredWorkItems[0]?.id ?? null;
  }, [activeWorkItemId, filteredWorkItems]);
  const { data: selectedWorkItem } = useQuery({
    queryKey: ["workItem", selectedWorkItemId],
    queryFn: () => getWorkItem(selectedWorkItemId!),
    enabled: !!selectedWorkItemId,
  });
  const { data: latestWorkflowRun } = useQuery({
    queryKey: ["latestWorkflowRun", selectedWorkItemId],
    queryFn: () => getLatestWorkflowRunForWorkItem(selectedWorkItemId!),
    enabled: !!selectedWorkItemId,
    refetchInterval: 4000,
  });
  const workflowRunId = activeWorkflowRunId ?? latestWorkflowRun?.id ?? null;
  const { data: subWorkItems } = useQuery({
    queryKey: ["subWorkItems", selectedWorkItemId, SUB_WORK_ITEM_PAGE_SIZE],
    queryFn: () =>
      getSubWorkItems(selectedWorkItemId!, {
        limit: SUB_WORK_ITEM_PAGE_SIZE,
        offset: 0,
      }),
    enabled: !!selectedWorkItemId,
  });
  const { data: approvals } = useQuery({ queryKey: ["approvals", selectedWorkItemId], queryFn: () => getWorkItemApprovals(selectedWorkItemId!), enabled: !!selectedWorkItemId });
  const { data: artifacts } = useQuery({
    queryKey: ["artifacts", selectedWorkItemId],
    queryFn: () => listWorkItemArtifacts(selectedWorkItemId!),
    enabled: !!selectedWorkItemId,
    refetchInterval: 4000,
  });
  const { data: resolvedRepositoryFromQuery } = useQuery({
    queryKey: ["resolvedRepositoryForWorkItem", selectedWorkItemId],
    queryFn: () => resolveRepositoryForWorkItem(selectedWorkItemId!),
    enabled: !!selectedWorkItemId,
    refetchInterval: 4000,
  });
  const { data: artifactModalContent } = useQuery({
    queryKey: ["artifactContent", artifactModalArtifact?.id],
    queryFn: () => readArtifactContent(artifactModalArtifact!.id),
    enabled: !!artifactModalArtifact?.id,
  });
  const { data: findings } = useQuery({ queryKey: ["findings", selectedWorkItemId], queryFn: () => listWorkItemFindings(selectedWorkItemId!), enabled: !!selectedWorkItemId });
  const { data: teamAssignments } = useQuery({ queryKey: ["teamAssignments"], queryFn: () => listTeamAssignments() });
  const { data: agentTeams } = useQuery({ queryKey: ["agentTeams"], queryFn: () => listAgentTeams() });
  const { data: teamMemberships } = useQuery({ queryKey: ["teamMemberships"], queryFn: () => listTeamMemberships() });
  const { data: agentDefinitions } = useQuery({ queryKey: ["agentDefinitions"], queryFn: () => listAgentDefinitions() });
  const { data: workflowPolicies } = useQuery({ queryKey: ["workflowPolicies"], queryFn: () => listWorkflowStagePolicies() });
  const { data: modelBindings } = useQuery({ queryKey: ["agentModelBindings"], queryFn: () => listAgentModelBindings() });
  const { data: modelDefinitions } = useQuery({ queryKey: ["modelDefinitions"], queryFn: () => listModelDefinitions() });
  const { data: providers } = useQuery({ queryKey: ["modelProviders"], queryFn: () => listProviders() });

  const { data: workflowHistory } = useQuery({
    queryKey: ["workflowHistory", workflowRunId],
    queryFn: () => getWorkflowHistory(workflowRunId!),
    enabled: !!workflowRunId,
    refetchInterval: 4000,
  });
  const activeWorkflowStage = latestWorkflowRun?.current_stage ?? null;
  const selectedDagNodeId = selectedArtifactStage ?? WORKFLOW_DAG_NODES.find((node) => node.actualStageIds.includes(activeWorkflowStage ?? ""))?.id ?? "draft";
  const { data: agentRuns } = useQuery({
    queryKey: ["agentRunsForWorkflow", workflowRunId],
    queryFn: () => listAgentRunsForWorkflow(workflowRunId!),
    enabled: !!workflowRunId,
    refetchInterval: 4000,
  });
  const { data: agentModelCalls } = useQuery({
    queryKey: ["agentModelCallsForWorkflow", workflowRunId],
    queryFn: () => listAgentModelCallsForWorkflow(workflowRunId!),
    enabled: !!workflowRunId,
    refetchInterval: 4000,
  });
  const { data: externalCliRuns } = useQuery({
    queryKey: ["externalCliRunsForWorkItem", selectedWorkItemId],
    queryFn: () => listExternalCliRunsForWorkItem(selectedWorkItemId!),
    enabled: !!selectedWorkItemId,
    refetchInterval: 2000,
  });

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
  }, [activeProductId, activeProductAreaId, activeCapabilityId]);

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
  }, [selectedWorkItem]);

  useEffect(() => {
    setWorkItemOrderIds(filteredWorkItems.map((workItem) => workItem.id));
  }, [filteredWorkItems]);

  useEffect(() => {
    if (showCreateForm || workItemCreateDialogOpen) {
      setFormError(null);
    }
  }, [showCreateForm, workItemCreateDialogOpen]);

  useEffect(() => {
    setActionError(null);
    setActionInfo(null);
  }, [selectedWorkItemId]);

  useEffect(() => {
    setOpenOverflowWorkItemId(null);
  }, [selectedWorkItemId, workItemWorkspaceTab]);

  useEffect(() => {
    setActiveWorkflowRunId(null);
  }, [selectedWorkItemId]);

  useEffect(() => {
    setSelectedExternalCliRunId(null);
  }, [selectedWorkItemId]);

  useEffect(() => {
    setSelectedArtifactStage(null);
  }, [selectedWorkItemId]);

  const latestAgentRunForActiveStage = useMemo(
    () => findLatestAgentRunForStage(agentRuns, activeWorkflowStage),
    [agentRuns, activeWorkflowStage],
  );

  const runningSinceMs = useMemo(
    () => getRunningAgentRunStartMs(latestAgentRunForActiveStage),
    [latestAgentRunForActiveStage],
  );

  const workflowElapsedLabel = useMemo(
    () => formatWorkflowElapsedLabel(runningSinceMs),
    [runningSinceMs],
  );

  const isStaleRun = useMemo(
    () => isWorkflowRunStale(runningSinceMs, latestWorkflowRun?.status),
    [runningSinceMs, latestWorkflowRun?.status],
  );

  const selectedDagNode = useMemo(
    () => WORKFLOW_DAG_NODES.find((node) => node.id === selectedDagNodeId) ?? WORKFLOW_DAG_NODES[0],
    [selectedDagNodeId],
  );
  const focusedStageNames = useMemo(
    () => selectedDagNode.actualStageIds,
    [selectedDagNode],
  );
  const stageRuns = useMemo(
    () => (agentRuns ?? []).filter((run) => focusedStageNames.includes(run.stage)),
    [agentRuns, focusedStageNames],
  );
  const stageArtifactsForFocusedStage = useMemo(
    () =>
      filterArtifactsForWorkflowStages(artifacts, focusedStageNames, workflowRunId),
    [artifacts, workflowRunId, focusedStageNames],
  );
  const stageHistoryForFocusedStage = useMemo(
    () =>
      filterWorkflowHistoryForStages(workflowHistory, focusedStageNames),
    [workflowHistory, focusedStageNames],
  );
  const artifactsByAgentRunId = useMemo(
    () => groupArtifactsByAgentRunId(stageArtifactsForFocusedStage),
    [stageArtifactsForFocusedStage],
  );
  const modelCallsByAgentRunId = useMemo(
    () => groupModelCallsByAgentRunId(agentModelCalls),
    [agentModelCalls],
  );
  const workflowModelUsage = useMemo(
    () => summarizeModelUsage(agentModelCalls ?? [], agentRuns ?? []),
    [agentModelCalls, agentRuns],
  );

  const invalidateTasks = async () => {
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
      queryClient.setQueryData<WorkItem[] | undefined>(["sidebarWorkItems", activeProductId], (current) =>
        current ? [...current, createdWorkItem] : [createdWorkItem],
      );
      setWorkItemOrderIds((current) => (current.includes(createdWorkItem.id) ? current : [...current, createdWorkItem.id]));
      setActiveWorkItem(createdWorkItem.id);
      await invalidateTasks();
      setCreateForm({
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
    onSuccess: async (workflowRunId) => {
      setActiveWorkflowRunId(workflowRunId);
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
  const createWorkspaceMutation = useMutation({
    mutationFn: async () => {
      if (!selectedWorkItemSummary) {
        throw new Error("No story selected.");
      }
      return createLocalWorkspace({
        productId: selectedWorkItemSummary.product_id ?? activeProductId,
        productAreaId: selectedWorkItemSummary.product_area_id ?? activeProductAreaId,
        workItemId: selectedWorkItemSummary.id,
      });
    },
    onSuccess: async () => {
      setActionError(null);
      setActionInfo("Workspace created and attached. Opening IDE.");
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["repositories"] }),
        queryClient.invalidateQueries({ queryKey: ["resolvedRepositoryForWorkItem", selectedWorkItemId] }),
        queryClient.invalidateQueries({ queryKey: ["ideScopeRepo"] }),
        invalidateTasks(),
      ]);
      setActiveView("ide");
    },
    onError: (error) => setActionError(String(error)),
  });

  const selectedWorkspaceRepository = repositories.find((repository: Repository) => repository.id === workspaceRepositoryId) ?? null;

  const resolveWorkspaceBranchName = () => {
    if (workspaceBranchMode === "default") {
      return selectedWorkspaceRepository?.default_branch ?? "";
    }
    if (workspaceBranchMode === "work_item") {
      return selectedWorkItemSummary ? workItemBranchName(selectedWorkItemSummary.title) : "";
    }
    return workspaceBranchName.trim();
  };

  const openWorkspaceEditor = () => {
    const currentRepositoryId =
      resolvedRepository?.id ??
      selectedWorkItemSummary?.repo_override_id ??
      selectedWorkItemSummary?.active_repo_id ??
      repositories[0]?.id ??
      "";
    const currentRepository = repositories.find((repository: Repository) => repository.id === currentRepositoryId) ?? null;
    const currentBranch = selectedWorkItemSummary?.branch_name ?? currentRepository?.default_branch ?? "";
    setWorkspaceRepositoryId(currentRepositoryId);
    setWorkspaceBranchName(currentBranch);
    setWorkspaceBranchMode(currentRepository && currentBranch === currentRepository.default_branch ? "default" : "custom");
    setActionError(null);
    setIsEditingWorkspace(true);
  };

  const assignWorkspaceMutation = useMutation({
    mutationFn: async () => {
      if (!selectedWorkItemSummary) {
        throw new Error("No story selected.");
      }
      if (!workspaceRepositoryId) {
        throw new Error("Select a workspace.");
      }
      const branchName = resolveWorkspaceBranchName();
      if (!branchName) {
        throw new Error("Branch name is required.");
      }
      return assignWorkItemWorkspace({
        id: selectedWorkItemSummary.id,
        repositoryId: workspaceRepositoryId,
        branchName,
      });
    },
    onSuccess: async (updatedWorkItem) => {
      setActionError(null);
      setActionInfo("Workspace and branch updated for this story.");
      setIsEditingWorkspace(false);
      queryClient.setQueryData(["workItem", selectedWorkItemId], updatedWorkItem);
      const updatedRepositoryId = updatedWorkItem.repo_override_id ?? updatedWorkItem.active_repo_id;
      const updatedRepository = repositories.find((repository: Repository) => repository.id === updatedRepositoryId) ?? null;
      if (updatedRepository) {
        queryClient.setQueryData(["resolvedRepositoryForWorkItem", selectedWorkItemId], updatedRepository);
      }
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["workItem", selectedWorkItemId] }),
        queryClient.invalidateQueries({ queryKey: workItemsScopeQueryKey, refetchType: "none" }),
        queryClient.refetchQueries({ queryKey: workItemsQueryKey, type: "active" }),
        queryClient.invalidateQueries({ queryKey: ["resolvedRepositoryForWorkItem", selectedWorkItemId] }),
        queryClient.invalidateQueries({ queryKey: ["ideScopeRepo"] }),
      ]);
    },
    onError: (error) => setActionError(String(error)),
  });

  const clearWorkspaceOverrideMutation = useMutation({
    mutationFn: async () => {
      if (!selectedWorkItemSummary) {
        throw new Error("No story selected.");
      }
      return assignWorkItemWorkspace({
        id: selectedWorkItemSummary.id,
        repositoryId: null,
        branchName: null,
      });
    },
    onSuccess: async (updatedWorkItem) => {
      setActionError(null);
      setActionInfo("Work item workspace override cleared. Scope defaults will be used.");
      setIsEditingWorkspace(false);
      queryClient.setQueryData(["workItem", selectedWorkItemId], updatedWorkItem);
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["workItem", selectedWorkItemId] }),
        queryClient.invalidateQueries({ queryKey: workItemsScopeQueryKey, refetchType: "none" }),
        queryClient.refetchQueries({ queryKey: workItemsQueryKey, type: "active" }),
        queryClient.invalidateQueries({ queryKey: ["resolvedRepositoryForWorkItem", selectedWorkItemId] }),
        queryClient.invalidateQueries({ queryKey: ["ideScopeRepo"] }),
      ]);
    },
    onError: (error) => setActionError(String(error)),
  });

  const selectedWorkItemSummary = useMemo(
    () => selectedWorkItem ?? filteredWorkItems.find((workItem) => workItem.id === selectedWorkItemId) ?? null,
    [filteredWorkItems, selectedWorkItem, selectedWorkItemId],
  );
  const repositoryFromWorkItem = useMemo(() => {
    const repositoryId = selectedWorkItemSummary?.repo_override_id ?? selectedWorkItemSummary?.active_repo_id;
    if (!repositoryId) {
      return null;
    }
    return repositories.find((repository: Repository) => repository.id === repositoryId) ?? null;
  }, [repositories, selectedWorkItemSummary?.active_repo_id, selectedWorkItemSummary?.repo_override_id]);
  const resolvedRepository = resolvedRepositoryFromQuery ?? repositoryFromWorkItem ?? null;
  const activeProduct = useMemo(
    () => (products ?? []).find((product: Product) => product.id === activeProductId) ?? null,
    [activeProductId, products],
  );
  const activeProductArea = useMemo(
    () => activeProductAreaId ? productAreaById.get(activeProductAreaId) ?? null : null,
    [activeProductAreaId, productAreaById],
  );
  const activeCapability = useMemo(() => {
    return activeCapabilityId ? capabilityById.get(activeCapabilityId) ?? null : null;
  }, [activeCapabilityId, capabilityById]);
  const scopeDescriptor = useMemo(() => {
    const parts: string[] = [];
    if (activeProduct?.name) {
      parts.push(activeProduct.name);
    }
    if (activeProductArea?.name) {
      parts.push(activeProductArea.name);
    }
    if (activeCapability?.name) {
      parts.push(activeCapability.name);
    }
    return parts.length > 0 ? parts.join(" / ") : "None selected";
  }, [activeCapability?.name, activeProductArea?.name, activeProduct?.name]);
  const createWorkItemScopeLabel = createForm.parentWorkItemId
    ? "Current story"
    : activeCapability
    ? `Current ${getHierarchyNodeKindLabel(activeCapability.node_kind, { lowercase: true })}`
    : activeCapabilityId
      ? "Current node"
      : activeProductAreaId
        ? "Current product area"
        : activeProductId
          ? "Current product"
          : "No product selected";
  const workItemOwnerMap = useMemo(() => {
    const map = new Map<string, { badge: string; path: string; isRoot: boolean }>();
    if (!activeProduct) {
      return map;
    }

    filteredWorkItems.forEach((workItem) => {
      const ownerId = workItem.source_node_id ?? workItem.capability_id ?? workItem.product_area_id;
      const ownerType = workItem.source_node_type ?? (workItem.capability_id ? "capability" : workItem.product_area_id ? "product_area" : null);

      if (ownerId && ownerType === "product_area") {
        const productArea = productAreaById.get(ownerId);
        if (productArea) {
          map.set(workItem.id, {
            badge: getHierarchyNodeKindLabel(productArea.node_kind),
            path: [activeProduct.name, productArea.name].join(" / "),
            isRoot: false,
          });
          return;
        }
      }

      if (ownerId && ownerType === "capability") {
        const capability = capabilityById.get(ownerId);
        if (capability) {
          const ownerPath = buildCapabilityPath(capability, productAreaById, capabilityById);
          map.set(workItem.id, {
            badge: getHierarchyNodeKindLabel(capability.node_kind),
            path: [activeProduct.name, ...ownerPath].join(" / "),
            isRoot: false,
          });
          return;
        }
      }

      if (workItem.capability_id) {
        const capability = capabilityById.get(workItem.capability_id);
        if (capability) {
          const ownerPath = buildCapabilityPath(capability, productAreaById, capabilityById);
          map.set(workItem.id, {
            badge: getHierarchyNodeKindLabel(capability.node_kind),
            path: [activeProduct.name, ...ownerPath].join(" / "),
            isRoot: false,
          });
          return;
        }
      }

      if (workItem.product_area_id) {
        const productArea = productAreaById.get(workItem.product_area_id);
        if (productArea) {
          map.set(workItem.id, {
            badge: getHierarchyNodeKindLabel(productArea.node_kind),
            path: [activeProduct.name, productArea.name].join(" / "),
            isRoot: false,
          });
          return;
        }
      }

      if (workItem.product_area_id || workItem.capability_id || workItem.source_node_id) {
        map.set(workItem.id, {
          badge: "Unknown Owner",
          path: activeProduct.name,
          isRoot: false,
        });
        return;
      }

      map.set(workItem.id, {
        badge: "Product",
        path: activeProduct.name,
        isRoot: true,
      });
    });

    return map;
  }, [activeProduct, capabilityById, filteredWorkItems, productAreaById]);
  const orderedWorkItems = useMemo(() => orderWorkItemsByIds(filteredWorkItems, workItemOrderIds), [filteredWorkItems, workItemOrderIds]);
  const backlogWindow = useMemo(() => {
    const start = Math.max(0, Math.floor(backlogScrollTop / BACKLOG_ROW_ESTIMATED_HEIGHT) - BACKLOG_OVERSCAN_ROWS);
    const visibleRows = Math.ceil(backlogViewportHeight / BACKLOG_ROW_ESTIMATED_HEIGHT) + BACKLOG_OVERSCAN_ROWS * 2;
    const end = Math.min(orderedWorkItems.length, start + Math.max(visibleRows, BACKLOG_OVERSCAN_ROWS * 2));
    return {
      start,
      end,
      topPadding: start * BACKLOG_ROW_ESTIMATED_HEIGHT,
      bottomPadding: Math.max(0, orderedWorkItems.length - end) * BACKLOG_ROW_ESTIMATED_HEIGHT,
      items: orderedWorkItems.slice(start, end),
    };
  }, [backlogScrollTop, backlogViewportHeight, orderedWorkItems]);
  const hasNextWorkItemPage = workItemPage?.has_more ?? false;
  const workItemPageStart = workItemPageIndex * WORK_ITEM_PAGE_SIZE + (orderedWorkItems.length > 0 ? 1 : 0);
  const workItemPageEnd = workItemPageIndex * WORK_ITEM_PAGE_SIZE + orderedWorkItems.length;
  const backlogRenderedRangeLabel =
    orderedWorkItems.length > 0 ? `${backlogWindow.start + 1}-${backlogWindow.end}` : "0-0";
  const selectedBacklogItems = useMemo(
    () => orderedWorkItems.filter((workItem) => selectedBacklogItemIds.includes(workItem.id)),
    [orderedWorkItems, selectedBacklogItemIds],
  );
  const isRowActionPending = (workItemId: string) => pendingRowActionIds.includes(workItemId);
  const runRowApprovalAction = async (workItemId: string, action: "approve" | "reject") => {
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
  const runBulkApprovalAction = async (action: "approve" | "reject") => {
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
  const backlogWorkflowRunQueries = useQueries({
    queries: backlogWindow.items.map((workItem) => ({
      queryKey: ["latestWorkflowRun", workItem.id],
      queryFn: () => getLatestWorkflowRunForWorkItem(workItem.id),
      enabled: workItemWorkspaceTab === "backlog",
      refetchInterval: 4000,
    })),
  });
  const latestWorkflowRunByWorkItemId = useMemo(() => {
    const map = new Map<string, WorkflowRun | null>();
    backlogWindow.items.forEach((workItem, index) => {
      const run = backlogWorkflowRunQueries[index]?.data ?? null;
      map.set(workItem.id, run);
    });
    return map;
  }, [backlogWorkflowRunQueries, backlogWindow.items]);
  const stageLabel = activeWorkflowStage ? activeWorkflowStage.replace(/_/g, " ") : null;
  const completedStages = useMemo(
    () => new Set((workflowHistory ?? []).map((entry) => entry.to_stage)),
    [workflowHistory],
  );
  const dagNodeById = useMemo(
    () => new Map(WORKFLOW_DAG_NODES.map((node) => [node.id, node])),
    [],
  );
  const laneStatusById = useMemo(
    () => buildWorkflowLaneStatusById({
      lanes: WORKFLOW_DAG_LANES,
      nodes: WORKFLOW_DAG_NODES,
      completedStages,
      activeWorkflowStage,
      workflowStatus: latestWorkflowRun?.status,
    }),
    [activeWorkflowStage, completedStages, latestWorkflowRun?.status],
  );
  const latestApproval = useMemo(
    () => (approvals ?? []).slice().sort((a, b) => b.created_at.localeCompare(a.created_at))[0] ?? null,
    [approvals],
  );
  const latestArtifact = useMemo(
    () => (artifacts ?? []).slice().sort((a, b) => b.created_at.localeCompare(a.created_at))[0] ?? null,
    [artifacts],
  );
  const latestExternalCliRun = useMemo(
    () => (externalCliRuns ?? []).slice().sort((a, b) => b.started_at.localeCompare(a.started_at))[0] ?? null,
    [externalCliRuns],
  );
  const activeExternalCliRunId = selectedExternalCliRunId ?? latestExternalCliRun?.id ?? null;
  const activeExternalCliRun = useMemo(
    () => (externalCliRuns ?? []).find((run) => run.id === activeExternalCliRunId) ?? latestExternalCliRun,
    [activeExternalCliRunId, externalCliRuns, latestExternalCliRun],
  );
  const { data: externalCliRunEvents } = useQuery({
    queryKey: ["externalCliRunEvents", activeExternalCliRunId, EXTERNAL_CLI_TRACE_LIMIT],
    queryFn: () => listExternalCliRunEvents(activeExternalCliRunId!, EXTERNAL_CLI_TRACE_LIMIT),
    enabled: !!activeExternalCliRunId,
    refetchInterval: activeExternalCliRun?.status === "running" ? 1000 : 4000,
  });
  const latestExternalCliEvent = useMemo(
    () => (externalCliRunEvents ?? []).slice().sort((a, b) => a.sequence - b.sequence).slice(-1)[0] ?? null,
    [externalCliRunEvents],
  );
  const externalCliTerminalOutput = useMemo(
    () => formatExternalCliTerminal(externalCliRunEvents ?? []),
    [externalCliRunEvents],
  );
  const externalCliProviderInFlight = externalCliMutation.isPending ? externalCliMutation.variables : null;
  const findingSeverityCounts = useMemo(() => {
    const counts = new Map<string, number>();
    for (const finding of findings ?? []) {
      counts.set(finding.severity, (counts.get(finding.severity) ?? 0) + 1);
    }
    return counts;
  }, [findings]);
  const topArtifactTypes = useMemo(() => {
    const counts = new Map<string, number>();
    for (const artifact of artifacts ?? []) {
      counts.set(artifact.artifact_type, (counts.get(artifact.artifact_type) ?? 0) + 1);
    }
    return Array.from(counts.entries())
      .sort((a, b) => b[1] - a[1])
      .slice(0, 3);
  }, [artifacts]);
  const workflowReadiness = useMemo(() => buildWorkItemWorkflowReadiness({
    selectedWorkItem: selectedWorkItemSummary,
    teamAssignments,
    agentTeams,
    teamMemberships,
    agentDefinitions,
    workflowPolicies,
    modelBindings,
    modelDefinitions,
    providers,
    resolvedRepository,
  }), [
    selectedWorkItemSummary,
    teamAssignments,
    agentTeams,
    teamMemberships,
    agentDefinitions,
    workflowPolicies,
    modelBindings,
    modelDefinitions,
    providers,
    resolvedRepository,
  ]);

  const renderWorkspaceAssignmentPanel = () => (
    <WorkItemWorkspaceAssignmentPanel
      isEditing={isEditingWorkspace}
      repositories={repositories}
      workspaceRepositoryId={workspaceRepositoryId}
      workspaceBranchMode={workspaceBranchMode}
      workspaceBranchName={workspaceBranchName}
      currentBranch={selectedWorkItemSummary?.branch_name || resolvedRepository?.default_branch || "not set"}
      branchPreview={resolveWorkspaceBranchName()}
      hasWorkspaceOverride={!!selectedWorkItemSummary?.repo_override_id}
      isAssignPending={assignWorkspaceMutation.isPending}
      isClearPending={clearWorkspaceOverrideMutation.isPending}
      onOpenEditor={openWorkspaceEditor}
      onClearOverride={() => clearWorkspaceOverrideMutation.mutate()}
      onRepositoryIdChange={(repositoryId) => {
        const nextRepository = repositories.find((repository: Repository) => repository.id === repositoryId) ?? null;
        setWorkspaceRepositoryId(repositoryId);
        if (workspaceBranchMode === "default") {
          setWorkspaceBranchName(nextRepository?.default_branch ?? "");
        }
      }}
      onBranchModeChange={setWorkspaceBranchMode}
      onBranchNameChange={setWorkspaceBranchName}
      onSave={() => assignWorkspaceMutation.mutate()}
      onCancel={() => setIsEditingWorkspace(false)}
    />
  );

  return (
    <div style={styles.page}>
      <div style={styles.header}>
        <div style={styles.titleBlock}>
          <h1 style={styles.title}>Delivery / Builder</h1>
          <div style={styles.subtitle}>Execute product delivery as stories and tasks, inspect evidence, and keep implementation scoped to the selected product management node.</div>
        </div>
      </div>

      <div style={styles.panel}>
        <div style={styles.panelInner}>
          <div style={styles.tabBar}>
            <button style={workItemWorkspaceTab === "backlog" ? styles.tabActive : styles.tab} onClick={() => setWorkItemWorkspaceTab("backlog")}>Backlog</button>
            <button style={workItemWorkspaceTab === "detail" ? styles.tabActive : styles.tab} onClick={() => setWorkItemWorkspaceTab("detail")}>Story Detail</button>
            <button style={workItemWorkspaceTab === "external_cli" ? styles.tabActive : styles.tab} onClick={() => setWorkItemWorkspaceTab("external_cli")}>External CLI</button>
            <button style={workItemWorkspaceTab === "review" ? styles.tabActive : styles.tab} onClick={() => setWorkItemWorkspaceTab("review")}>Review</button>
          </div>

          {workItemWorkspaceTab === "backlog" && (
            <WorkItemBacklogTab
              activeProduct={activeProduct}
              activeProductArea={activeProductArea}
              activeCapability={activeCapability}
              selectedBacklogItems={selectedBacklogItems}
              bulkActionInFlight={bulkActionInFlight}
              onRunBulkApprovalAction={(action) => void runBulkApprovalAction(action)}
              onOpenCreateStory={() => {
                setCreateForm((current) => ({ ...current, parentWorkItemId: null }));
                openWorkItemCreateDialog();
              }}
              activeProductId={activeProductId}
              actionError={actionError}
              statusFilter={statusFilter}
              onStatusFilterChange={setStatusFilter}
              scopeDescriptor={scopeDescriptor}
              workItemPageIndex={workItemPageIndex}
              workItemPageStart={workItemPageStart}
              workItemPageEnd={workItemPageEnd}
              backlogRenderedRangeLabel={backlogRenderedRangeLabel}
              onWorkItemPageIndexChange={setWorkItemPageIndex}
              hasNextWorkItemPage={hasNextWorkItemPage}
              isLoading={isLoading}
              orderedWorkItems={orderedWorkItems}
              backlogViewportRef={backlogViewportRef}
              onBacklogScrollTopChange={setBacklogScrollTop}
              backlogWindow={backlogWindow}
              latestWorkflowRunByWorkItemId={latestWorkflowRunByWorkItemId}
              workItemOwnerMap={workItemOwnerMap}
              selectedWorkItemId={selectedWorkItemId}
              draggedWorkItemId={draggedWorkItemId}
              onDraggedWorkItemIdChange={setDraggedWorkItemId}
              workItemOrderIds={workItemOrderIds}
              onWorkItemOrderIdsChange={setWorkItemOrderIds}
              onReorderWorkItems={(orderedIds) => reorderWorkItemsMutation.mutate(orderedIds)}
              onSelectWorkItem={(workItemId) => {
                setActiveWorkItem(workItemId);
                setWorkItemWorkspaceTab("detail");
              }}
              selectedBacklogItemIds={selectedBacklogItemIds}
              onSelectedBacklogItemIdsChange={setSelectedBacklogItemIds}
              onRunRowApprovalAction={(workItemId, action) => void runRowApprovalAction(workItemId, action)}
              isRowActionPending={isRowActionPending}
              openOverflowWorkItemId={openOverflowWorkItemId}
              onOpenOverflowWorkItemIdChange={setOpenOverflowWorkItemId}
              onDeleteWorkItem={(workItemId) => deleteMutation.mutate(workItemId)}
            />
          )}

          {workItemWorkspaceTab === "detail" && (
            <WorkItemDetailTab
              workItem={selectedWorkItemSummary}
              workflowReadiness={workflowReadiness}
              isWorkflowPending={workflowMutation.isPending}
              onStartWorkflow={() => workflowMutation.mutate()}
              onApprove={() => approveMutation.mutate()}
              onReject={() => rejectMutation.mutate()}
              workflowRunId={workflowRunId}
              isRestartWorkflowPending={restartWorkflowMutation.isPending}
              onRestartWorkflow={() => restartWorkflowMutation.mutate()}
              onEditStory={() => setIsEditingWorkItem(true)}
              onCreateTask={() => {
                if (!selectedWorkItemSummary) {
                  return;
                }
                setCreateForm((current) => ({ ...current, parentWorkItemId: selectedWorkItemSummary.id }));
                openWorkItemCreateDialog();
              }}
              resolvedRepository={resolvedRepository}
              isExternalCliPending={externalCliMutation.isPending}
              externalCliProviderInFlight={externalCliProviderInFlight}
              onRunExternalCli={(provider) => externalCliMutation.mutate(provider)}
              latestExternalCliRun={latestExternalCliRun}
              onOpenExternalCliRun={(runId) => {
                setSelectedExternalCliRunId(runId);
                setWorkItemWorkspaceTab("external_cli");
              }}
              actionError={actionError}
              actionInfo={actionInfo}
              workspaceAssignmentPanel={renderWorkspaceAssignmentPanel()}
              isCreateWorkspacePending={createWorkspaceMutation.isPending}
              onCreateWorkspace={() => createWorkspaceMutation.mutate()}
              latestWorkflowRun={latestWorkflowRun}
            />
          )}

          {workItemWorkspaceTab === "external_cli" && (
            <WorkItemExternalCliTab
              selectedWorkItemId={selectedWorkItemId}
              resolvedRepository={resolvedRepository}
              isRunPending={externalCliMutation.isPending}
              providerInFlight={externalCliProviderInFlight}
              onRunProvider={(provider) => externalCliMutation.mutate(provider)}
              actionError={actionError}
              actionInfo={actionInfo}
              activeRun={activeExternalCliRun}
              activeRunId={activeExternalCliRunId}
              latestEvent={latestExternalCliEvent}
              events={externalCliRunEvents}
              terminalOutput={externalCliTerminalOutput}
              runs={externalCliRuns}
              artifacts={artifacts}
              onOpenArtifact={setArtifactModalArtifact}
              onSelectRun={setSelectedExternalCliRunId}
            />
          )}

          {workItemWorkspaceTab === "review" && (
            <>
              <WorkItemWorkspaceReadinessCard
                resolvedRepository={resolvedRepository}
                selectedWorkItem={selectedWorkItemSummary}
                workspaceAssignmentPanel={renderWorkspaceAssignmentPanel()}
                isCreateWorkspacePending={createWorkspaceMutation.isPending}
                onCreateWorkspace={() => createWorkspaceMutation.mutate()}
              />
              <div style={styles.sectionTitle}>Review Signals</div>
              <WorkItemReviewWorkflowCard
                workflowRunId={workflowRunId}
                latestWorkflowRun={latestWorkflowRun}
                activeWorkflowStage={activeWorkflowStage}
                stageLabel={stageLabel}
                workflowElapsedLabel={workflowElapsedLabel}
                isStaleRun={isStaleRun}
                workflowModelUsage={workflowModelUsage}
                laneStatusById={laneStatusById}
                dagNodeById={dagNodeById}
                completedStages={completedStages}
                selectedDagNodeId={selectedDagNodeId}
                selectedDagNode={selectedDagNode}
                stageRuns={stageRuns}
                artifactsByAgentRunId={artifactsByAgentRunId}
                modelCallsByAgentRunId={modelCallsByAgentRunId}
                stageHistoryForFocusedStage={stageHistoryForFocusedStage}
                isFailWorkflowRunPending={failWorkflowRunMutation.isPending}
                isRestartWorkflowPending={restartWorkflowMutation.isPending}
                isPlanApprovalPending={planApprovalMutation.isPending}
                isPlanRejectPending={planRejectMutation.isPending}
                isTestReviewApprovePending={testReviewApproveMutation.isPending}
                isTestReviewRejectPending={testReviewRejectMutation.isPending}
                onFailWorkflowRun={() => failWorkflowRunMutation.mutate()}
                onRestartWorkflow={() => restartWorkflowMutation.mutate()}
                onApprovePlan={() => planApprovalMutation.mutate()}
                onRejectPlan={() => planRejectMutation.mutate()}
                onApproveTestReview={() => testReviewApproveMutation.mutate()}
                onRejectTestReview={() => testReviewRejectMutation.mutate()}
                onSelectArtifactStage={setSelectedArtifactStage}
                onOpenArtifact={setArtifactModalArtifact}
              />
              <WorkItemReviewSummaryCards
                subWorkItems={subWorkItems}
                approvals={approvals}
                artifacts={artifacts}
                findings={findings}
                latestApproval={latestApproval}
                latestArtifact={latestArtifact}
                topArtifactTypes={topArtifactTypes}
                findingSeverityCounts={findingSeverityCounts}
              />
            </>
          )}
        </div>
      </div>

      {artifactModalArtifact && (
        <WorkItemArtifactModal
          artifact={artifactModalArtifact}
          content={artifactModalContent}
          onClose={() => setArtifactModalArtifact(null)}
        />
      )}

      {(showCreateForm || workItemCreateDialogOpen) && (
        <WorkItemCreateModal
          createForm={createForm}
          setCreateForm={setCreateForm}
          creationScopeLabel={createWorkItemScopeLabel}
          hasActiveProduct={!!activeProductId}
          formError={formError}
          isPending={createMutation.isPending}
          onClose={() => { setShowCreateForm(false); closeWorkItemCreateDialog(); }}
          onSubmit={() => createMutation.mutate()}
        />
      )}

      {isEditingWorkItem && selectedWorkItemSummary && (
        <WorkItemEditModal
          draft={workItemDraft}
          setDraft={setWorkItemDraft}
          formError={formError}
          isPending={updateWorkItemMutation.isPending}
          onClose={() => setIsEditingWorkItem(false)}
          onSubmit={() => updateWorkItemMutation.mutate()}
        />
      )}
    </div>
  );
}
