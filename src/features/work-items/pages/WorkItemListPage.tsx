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
import {
  WorkItemCreateModal,
  WorkItemEditModal,
  type WorkItemCreateFormState,
  type WorkItemEditDraftState,
} from "../components/WorkItemFormModals";
import { WorkItemWorkspaceAssignmentPanel } from "../components/WorkItemWorkspaceAssignmentPanel";
import { styles } from "../lib/workItemListPageStyles";
import {
  BACKLOG_OVERSCAN_ROWS,
  BACKLOG_ROW_ESTIMATED_HEIGHT,
  EXTERNAL_CLI_PROVIDERS,
  EXTERNAL_CLI_TRACE_LIMIT,
  SUB_WORK_ITEM_PAGE_SIZE,
  WORKFLOW_DAG_LANES,
  WORKFLOW_DAG_LINKS,
  WORKFLOW_DAG_NODES,
  WORK_ITEM_PAGE_SIZE,
  buildCapabilityPath,
  describeWorkItemRuntime,
  formatDurationMs,
  formatExternalCliCommand,
  formatExternalCliTerminal,
  formatExternalCliTerminalEvent,
  formatInteger,
  getArtifactFileName,
  getWorkItemExecutionSteps,
  orderWorkItemsByIds,
  parseSqliteUtcTimestamp,
  summarizeModelUsage,
  workItemBranchName,
  type ExternalCliProvider,
  type WorkspaceBranchMode,
} from "../lib/workItemListPageHelpers";
import type {
  AgentDefinition,
  AgentModelBinding,
  ExternalCliRun,
  ExternalCliRunEvent,
  ModelCall,
  AgentRun,
  AgentTeam,
  AgentTeamMembership,
  Approval,
  Artifact,
  Finding,
  ModelDefinition,
  ModelProvider,
  TeamAssignment,
  WorkItem,
  WorkItemPage,
  WorkflowRun,
  WorkflowStageHistory,
  WorkflowStagePolicy,
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

  const latestAgentRunForActiveStage = useMemo(() => {
    if (!activeWorkflowStage || !agentRuns?.length) return null;
    return [...agentRuns]
      .reverse()
      .find((run) => run.stage === activeWorkflowStage) ?? null;
  }, [agentRuns, activeWorkflowStage]);

  const runningSinceMs = useMemo(() => {
    if (!latestAgentRunForActiveStage || latestAgentRunForActiveStage.status !== "running") return null;
    return parseSqliteUtcTimestamp(latestAgentRunForActiveStage.started_at);
  }, [latestAgentRunForActiveStage]);

  const workflowElapsedLabel = useMemo(() => {
    if (!runningSinceMs) return null;
    const elapsedMs = Date.now() - runningSinceMs;
    if (elapsedMs < 0) return null;
    const mins = Math.floor(elapsedMs / 60000);
    const secs = Math.floor((elapsedMs % 60000) / 1000);
    return `${mins}m ${secs}s`;
  }, [runningSinceMs]);

  const isStaleRun = useMemo(() => {
    if (!runningSinceMs || latestWorkflowRun?.status !== "running") return false;
    return Date.now() - runningSinceMs > 7 * 60 * 1000;
  }, [runningSinceMs, latestWorkflowRun?.status]);

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
      (artifacts ?? []).filter((artifact) => {
        if (workflowRunId && artifact.workflow_run_id !== workflowRunId) {
          return false;
        }
        if (focusedStageNames.some((stageName) => artifact.artifact_type.startsWith(`${stageName}_`))) return true;
        if (focusedStageNames.includes("coding")) {
          return artifact.artifact_type === "coding_tool_trace" || artifact.artifact_type === "coding_applied_files";
        }
        return false;
      }),
    [artifacts, workflowRunId, focusedStageNames],
  );
  const stageHistoryForFocusedStage = useMemo(
    () =>
      (workflowHistory ?? []).filter(
        (entry) =>
          focusedStageNames.includes(entry.from_stage) || focusedStageNames.includes(entry.to_stage),
      ),
    [workflowHistory, focusedStageNames],
  );
  const artifactsByAgentRunId = useMemo(() => {
    const map = new Map<string, Artifact[]>();
    for (const artifact of stageArtifactsForFocusedStage) {
      if (!artifact.agent_run_id) continue;
      const list = map.get(artifact.agent_run_id) ?? [];
      list.push(artifact);
      map.set(artifact.agent_run_id, list);
    }
    return map;
  }, [stageArtifactsForFocusedStage]);
  const modelCallsByAgentRunId = useMemo(() => {
    const map = new Map<string, ModelCall[]>();
    for (const call of agentModelCalls ?? []) {
      if (!call.agent_run_id) continue;
      const list = map.get(call.agent_run_id) ?? [];
      list.push(call);
      map.set(call.agent_run_id, list);
    }
    for (const calls of map.values()) {
      calls.sort((a, b) => a.call_index - b.call_index || a.created_at.localeCompare(b.created_at));
    }
    return map;
  }, [agentModelCalls]);
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
  const laneStatusById = useMemo(() => {
    const map = new Map<string, { done: number; active: number; pending: number; failed: number }>();
    for (const lane of WORKFLOW_DAG_LANES) {
      let done = 0;
      let active = 0;
      let pending = 0;
      let failed = 0;
      for (const nodeId of lane.nodeIds) {
        const node = dagNodeById.get(nodeId);
        if (!node) continue;
        if (node.actualStageIds.length === 0) {
          pending += 1;
          continue;
        }
        const hasFailed = node.actualStageIds.some((stageId) => stageId === "failed" || (latestWorkflowRun?.status === "failed" && activeWorkflowStage === stageId));
        const isActive = node.actualStageIds.includes(activeWorkflowStage ?? "");
        const isDone = node.actualStageIds.every((stageId) => completedStages.has(stageId) || stageId === "done");
        if (hasFailed) {
          failed += 1;
        } else if (isActive) {
          active += 1;
        } else if (isDone) {
          done += 1;
        } else {
          pending += 1;
        }
      }
      map.set(lane.id, { done, active, pending, failed });
    }
    return map;
  }, [activeWorkflowStage, completedStages, dagNodeById, latestWorkflowRun?.status]);
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
  const workflowReadiness = useMemo(() => {
    const blockers: string[] = [];
    const warnings: string[] = [];
    const checks: string[] = [];

    if (!selectedWorkItemSummary) {
      return { blockers: ["Select a story to evaluate readiness."], warnings, checks };
    }

    if (selectedWorkItemSummary.status !== "approved") {
      blockers.push("Work item status must be Approved before starting workflow.");
    } else {
      checks.push("Work item is approved.");
    }

    if (!resolvedRepository) {
      blockers.push("No workspace is attached to this story scope. Create a local workspace before starting delivery stages.");
    } else {
      checks.push(`Workspace resolved: ${resolvedRepository.name}.`);
      checks.push(`Branch resolved: ${selectedWorkItemSummary.branch_name || resolvedRepository.default_branch}.`);
      if (!resolvedRepository.remote_url) {
        warnings.push("Workspace has no remote configured. Local-only delivery is fine, but push stages will remain local until a remote is added.");
      }
    }

    const assignmentMatch = (teamAssignments ?? []).find((assignment: TeamAssignment) => {
      if (assignment.scope_type === "capability" && selectedWorkItemSummary.capability_id) {
        return assignment.scope_id === selectedWorkItemSummary.capability_id;
      }
      if (assignment.scope_type === "product_area" && selectedWorkItemSummary.product_area_id) {
        return assignment.scope_id === selectedWorkItemSummary.product_area_id;
      }
      if (assignment.scope_type === "product") {
        return assignment.scope_id === selectedWorkItemSummary.product_id;
      }
      return false;
    });

    const matchedTeam = assignmentMatch
      ? (agentTeams ?? []).find((team: AgentTeam) => team.id === assignmentMatch.team_id)
      : null;

    if (!matchedTeam) {
      warnings.push("No team assignment found for capability/product_area/product scope. Fallback global agents will be used.");
    } else {
      checks.push(`Team assignment resolved: ${matchedTeam.name}.`);
      if (!matchedTeam.enabled) {
        blockers.push(`Assigned team "${matchedTeam.name}" is disabled.`);
      }
    }

    const activeAgents = (agentDefinitions ?? []).filter((agent: AgentDefinition) => agent.enabled && agent.employment_status === "active");
    if (activeAgents.length === 0) {
      blockers.push("No active agents are available.");
    } else {
      checks.push(`${activeAgents.length} active agents available.`);
    }

    const stagePolicy = (workflowPolicies ?? []).find((policy: WorkflowStagePolicy) => policy.stage_name === "requirement_analysis");
    const requiredRoles = stagePolicy
      ? [...stagePolicy.primary_roles, ...stagePolicy.fallback_roles]
      : ["manager", "architect", "analyst", "requirement_analysis"];
    const stageAgent = activeAgents.find((agent) =>
      requiredRoles.some((role) => role.toLowerCase() === agent.role.toLowerCase()),
    );

    if (!stageAgent) {
      blockers.push("No active agent matches requirement-analysis roles.");
    } else {
      checks.push(`Requirement-analysis agent ready: ${stageAgent.name} (${stageAgent.role}).`);
    }

    const stageAgentBinding = stageAgent
      ? (modelBindings ?? []).find((binding: AgentModelBinding) => binding.agent_id === stageAgent.id)
      : null;
    const boundModel = stageAgentBinding
      ? (modelDefinitions ?? []).find((model: ModelDefinition) => model.id === stageAgentBinding.model_id)
      : null;
    const boundProvider = boundModel
      ? (providers ?? []).find((provider: ModelProvider) => provider.id === boundModel.provider_id)
      : null;

    if (!stageAgentBinding || !boundModel) {
      blockers.push("Requirement-analysis agent has no model binding.");
    } else {
      checks.push(`Model binding resolved: ${boundModel.name}.`);
      if (!boundModel.enabled) {
        blockers.push(`Bound model "${boundModel.name}" is disabled.`);
      }
      if (!boundProvider) {
        blockers.push("Bound model provider is missing.");
      } else if (!boundProvider.enabled) {
        blockers.push(`Model provider "${boundProvider.name}" is disabled.`);
      } else {
        checks.push(`Provider ready: ${boundProvider.name}.`);
      }
    }

    const coordinatorRequired = stagePolicy ? stagePolicy.coordinator_required : true;
    if (coordinatorRequired) {
      if (!matchedTeam) {
        warnings.push("Coordinator review is enabled, but no team is assigned. Workflow will bypass coordinator stage.");
      } else {
        const teamMembers = (teamMemberships ?? []).filter((membership: AgentTeamMembership) => membership.team_id === matchedTeam.id);
        const hasCoordinator = teamMembers.some((membership) => {
          const memberAgent = (agentDefinitions ?? []).find((agent: AgentDefinition) => agent.id === membership.agent_id);
          if (!memberAgent || !memberAgent.enabled || memberAgent.employment_status !== "active") {
            return false;
          }
          const normalizedRole = memberAgent.role.toLowerCase();
          return membership.is_lead || normalizedRole === "manager" || normalizedRole === "team_lead";
        });
        if (!hasCoordinator) {
          warnings.push(`Coordinator review is enabled, but team "${matchedTeam.name}" has no active lead/manager.`);
        } else {
          checks.push("Coordinator available for review gates.");
        }
      }
    }

    return { blockers, warnings, checks };
  }, [
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
            selectedWorkItemSummary ? (
              <>
                <div style={styles.detailTitle}>{selectedWorkItemSummary.title}</div>
                <div style={{ display: "flex", gap: 8, marginBottom: 12, flexWrap: "wrap" }}>
                  <button
                    style={styles.btn}
                    disabled={workflowReadiness.blockers.length > 0 || workflowMutation.isPending}
                    onClick={() => workflowMutation.mutate()}
                  >
                    {workflowMutation.isPending ? "Starting..." : "Start Workflow"}
                  </button>
                  <button style={{ ...styles.btn, backgroundColor: "#2d6a3f" }} onClick={() => approveMutation.mutate()}>Approve</button>
                  <button style={styles.btnDanger} onClick={() => rejectMutation.mutate()}>Reject</button>
                  {workflowRunId && (
                    <button
                      style={styles.btn}
                      onClick={() => restartWorkflowMutation.mutate()}
                      disabled={restartWorkflowMutation.isPending}
                    >
                      {restartWorkflowMutation.isPending ? "Restarting..." : "Restart Workflow"}
                    </button>
                  )}
                  <button style={styles.ghostBtn} onClick={() => setIsEditingWorkItem(true)}>
                    Edit Story
                  </button>
                  <button
                    style={styles.ghostBtn}
                    onClick={() => {
                      setCreateForm((current) => ({ ...current, parentWorkItemId: selectedWorkItemSummary.id }));
                      openWorkItemCreateDialog();
                    }}
                  >
                    + New Task
                  </button>
                  {EXTERNAL_CLI_PROVIDERS.map((entry) => (
                    <button
                      key={entry.provider}
                      style={styles.ghostBtn}
                      onClick={() => externalCliMutation.mutate(entry.provider)}
                      disabled={!resolvedRepository || externalCliMutation.isPending}
                      title={!resolvedRepository ? "Attach a workspace before launching an external CLI." : `Run ${entry.label}`}
                    >
                      {externalCliProviderInFlight === entry.provider ? "Running..." : `Run ${entry.label}`}
                    </button>
                  ))}
                  {latestExternalCliRun ? (
                    <button
                      style={styles.ghostBtn}
                      onClick={() => {
                        setSelectedExternalCliRunId(latestExternalCliRun.id);
                        setWorkItemWorkspaceTab("external_cli");
                      }}
                    >
                      CLI: {latestExternalCliRun.status}
                    </button>
                  ) : null}
                </div>
                {actionError && <div style={styles.errorText}>{actionError}</div>}
                {actionInfo && <div style={{ ...styles.smallText, color: "#4ec9b0", marginBottom: 10 }}>{actionInfo}</div>}
                <div style={styles.readinessCard}>
                  <div style={styles.readinessHeading}>Workflow Readiness Check</div>
                  {workflowReadiness.blockers.length === 0 && workflowReadiness.warnings.length === 0 ? (
                    <div style={{ ...styles.readinessItem, ...styles.readinessOk }}>Ready to start.</div>
                  ) : null}
                  {workflowReadiness.blockers.map((item) => (
                    <div key={`blocker-${item}`} style={styles.readinessItem}>
                      <span style={styles.readinessBlocker}>Blocker:</span> {item}
                    </div>
                  ))}
                  {workflowReadiness.warnings.map((item) => (
                    <div key={`warn-${item}`} style={styles.readinessItem}>
                      <span style={styles.readinessWarn}>Warning:</span> {item}
                    </div>
                  ))}
                  {workflowReadiness.checks.map((item) => (
                    <div key={`ok-${item}`} style={styles.readinessItem}>
                      <span style={styles.readinessOk}>OK:</span> {item}
                    </div>
                  ))}
                </div>
                <div style={styles.detailCard}>
                  <div style={styles.detailLabel}>Workspace Readiness</div>
                  {resolvedRepository ? (
                    <>
                      <div style={styles.detailValue}>{resolvedRepository.name}</div>
                      <div style={styles.smallText}>{resolvedRepository.local_path}</div>
                      <div style={styles.smallText}>
                        {resolvedRepository.remote_url
                          ? `Remote configured: ${resolvedRepository.remote_url}`
                          : "Remote: not configured"}
                      </div>
                      <div style={styles.smallText}>Branch: {selectedWorkItemSummary.branch_name || resolvedRepository.default_branch}</div>
                      <div style={styles.smallText}>
                        Source: {selectedWorkItemSummary.repo_override_id ? "story override" : "scope default"}
                      </div>
                      <div style={styles.smallText}>Version history: enabled</div>
                      {renderWorkspaceAssignmentPanel()}
                    </>
                  ) : (
                    <>
                      <div style={styles.warning}>
                        No workspace is attached to the current story scope.
                      </div>
                      <div style={styles.smallText}>
                        Create the workspace here and AruviStudio will enable version history and attach it automatically.
                      </div>
                      <div style={{ marginTop: 10 }}>
                        <button
                          style={styles.btn}
                          onClick={() => createWorkspaceMutation.mutate()}
                          disabled={createWorkspaceMutation.isPending}
                        >
                          {createWorkspaceMutation.isPending ? "Creating Workspace..." : "Create Workspace"}
                        </button>
                      </div>
                      {renderWorkspaceAssignmentPanel()}
                    </>
                  )}
                </div>

                <>
                  <div style={styles.detailCard}>
                    <div style={styles.detailLabel}>Description</div>
                    <div style={styles.detailValue}>{selectedWorkItemSummary.description || "No description yet."}</div>
                  </div>
                  <div style={styles.detailCard}>
                    <div style={styles.detailLabel}>Execution Steps</div>
                    <div style={styles.list}>
                      {getWorkItemExecutionSteps(selectedWorkItemSummary, resolvedRepository?.name ?? null).map((step, index) => (
                        <div key={`${selectedWorkItemSummary.id}-step-${index}`} style={styles.listItem}>
                          <div style={styles.detailValue}>{index + 1}. {step}</div>
                        </div>
                      ))}
                    </div>
                  </div>
                  <div style={styles.row}>
                    <div style={styles.detailCard}><div style={styles.detailLabel}>Story Status</div><div style={styles.detailValue}>{selectedWorkItemSummary.status.replace(/_/g, " ")}</div></div>
                    <div style={styles.detailCard}><div style={styles.detailLabel}>Workflow Status</div><div style={styles.detailValue}>{describeWorkItemRuntime(selectedWorkItemSummary, latestWorkflowRun ?? null).detail}</div></div>
                  </div>
                  <div style={styles.row}>
                    <div style={styles.detailCard}><div style={styles.detailLabel}>Priority</div><div style={styles.detailValue}>{selectedWorkItemSummary.priority}</div></div>
                    <div style={styles.detailCard}><div style={styles.detailLabel}>Type</div><div style={styles.detailValue}>{selectedWorkItemSummary.work_item_type}</div></div>
                  </div>
                  <div style={styles.detailCard}><div style={styles.detailLabel}>Complexity</div><div style={styles.detailValue}>{selectedWorkItemSummary.complexity}</div></div>
                  {selectedWorkItemSummary.problem_statement && <div style={styles.detailCard}><div style={styles.detailLabel}>Problem Statement</div><div style={styles.detailValue}>{selectedWorkItemSummary.problem_statement}</div></div>}
                  {selectedWorkItemSummary.acceptance_criteria && <div style={styles.detailCard}><div style={styles.detailLabel}>Acceptance Criteria</div><div style={styles.detailValue}>{selectedWorkItemSummary.acceptance_criteria}</div></div>}
                  {selectedWorkItemSummary.constraints && <div style={styles.detailCard}><div style={styles.detailLabel}>Constraints</div><div style={styles.detailValue}>{selectedWorkItemSummary.constraints}</div></div>}
                </>
              </>
            ) : (
              <div style={styles.empty}>Select a story from the queue to refine it.</div>
            )
          )}

          {workItemWorkspaceTab === "external_cli" && (
            <>
              <div style={styles.detailTitle}>External CLI</div>
              <div style={{ display: "flex", gap: 8, marginBottom: 12, flexWrap: "wrap" }}>
                {EXTERNAL_CLI_PROVIDERS.map((entry) => (
                  <button
                    key={entry.provider}
                    style={styles.btn}
                    onClick={() => externalCliMutation.mutate(entry.provider)}
                    disabled={!selectedWorkItemId || !resolvedRepository || externalCliMutation.isPending}
                    title={!resolvedRepository ? "Attach a workspace before launching an external CLI." : `Run ${entry.label}`}
                  >
                    {externalCliProviderInFlight === entry.provider ? "Running..." : `Run ${entry.label}`}
                  </button>
                ))}
              </div>
              {actionError && <div style={styles.errorText}>{actionError}</div>}
              {actionInfo && <div style={{ ...styles.smallText, color: "#4ec9b0", marginBottom: 10 }}>{actionInfo}</div>}

              <div style={styles.detailCard}>
                <div style={styles.detailLabel}>Active Run</div>
                {!resolvedRepository ? (
                  <div style={styles.warning}>Attach a workspace before launching an external CLI.</div>
                ) : activeExternalCliRun ? (
                  <>
                    <div style={styles.taskTitle}>{activeExternalCliRun.label} · {activeExternalCliRun.status}</div>
                    <div style={styles.smallText}>Run: <code>{activeExternalCliRun.id}</code></div>
                    <div style={styles.smallText}>Command: {formatExternalCliCommand(activeExternalCliRun)}</div>
                    <div style={styles.smallText}>CWD: {activeExternalCliRun.cwd}</div>
                    {activeExternalCliRun.session_log_path ? (
                      <div style={styles.smallText}>Session file: <code>{activeExternalCliRun.session_log_path}</code></div>
                    ) : null}
                    <div style={styles.smallText}>
                      Started: {activeExternalCliRun.started_at}{activeExternalCliRun.ended_at ? ` · Ended: ${activeExternalCliRun.ended_at}` : ""}
                    </div>
                    <div style={styles.smallText}>
                      Exit {activeExternalCliRun.exit_code ?? "n/a"} · Duration {formatDurationMs(activeExternalCliRun.duration_ms)} · stdout {formatInteger(activeExternalCliRun.stdout_chars)} chars · stderr {formatInteger(activeExternalCliRun.stderr_chars)} chars
                    </div>
                    {activeExternalCliRun.error_message && <div style={styles.warning}>{activeExternalCliRun.error_message}</div>}
                    {(() => {
                      const outputArtifact = (artifacts ?? []).find((artifact) => artifact.id === activeExternalCliRun.output_artifact_id) ?? null;
                      return outputArtifact ? (
                        <button
                          style={{ ...styles.ghostBtn, marginTop: 8 }}
                          onClick={() => setArtifactModalArtifact(outputArtifact)}
                        >
                          Open captured output
                        </button>
                      ) : (
                        <div style={styles.smallText}>Captured artifact pending.</div>
                      );
                    })()}
                  </>
                ) : (
                  <div style={styles.detailValue}>No external CLI run has been launched for this story.</div>
                )}
              </div>

              <div style={styles.detailCard}>
                <div style={styles.detailLabel}>Latest Log</div>
                {latestExternalCliEvent ? (
                  <pre style={{ ...styles.terminalOutput, maxHeight: 120 }}>{formatExternalCliTerminalEvent(latestExternalCliEvent)}</pre>
                ) : activeExternalCliRun?.status === "running" ? (
                  <div style={styles.detailValue}>Waiting for the first CLI event...</div>
                ) : (
                  <div style={styles.detailValue}>No log events recorded yet.</div>
                )}
              </div>

              <div style={styles.detailCard}>
                <div style={styles.detailLabel}>Console Output</div>
                <div style={styles.smallText}>Showing latest {formatInteger(EXTERNAL_CLI_TRACE_LIMIT)} events for the selected run as a combined terminal transcript.</div>
                {externalCliRunEvents && externalCliRunEvents.length > 0 ? (
                  <pre style={styles.terminalOutput}>{externalCliTerminalOutput}</pre>
                ) : activeExternalCliRun ? (
                  <div style={styles.detailValue}>Trace events are loading...</div>
                ) : (
                  <div style={styles.detailValue}>No trace is available until a CLI run starts.</div>
                )}
              </div>

              <div style={styles.detailCard}>
                <div style={styles.detailLabel}>Run History</div>
                {externalCliRuns && externalCliRuns.length > 0 ? (
                  <div style={styles.list}>
                    {externalCliRuns.map((run: ExternalCliRun) => {
                      const outputArtifact = (artifacts ?? []).find((artifact) => artifact.id === run.output_artifact_id) ?? null;
                      return (
                        <div key={run.id} style={run.id === activeExternalCliRunId ? { ...styles.listItem, borderColor: "#0e639c" } : styles.listItem}>
                          <div style={styles.taskTitle}>{run.label} · {run.status}</div>
                          <div style={styles.smallText}>Run: {run.id}</div>
                          <div style={styles.smallText}>Command: {formatExternalCliCommand(run)}</div>
                          <div style={styles.smallText}>CWD: {run.cwd}</div>
                          {run.session_log_path ? <div style={styles.smallText}>Session file: {run.session_log_path}</div> : null}
                          <div style={styles.smallText}>
                            Started: {run.started_at}{run.ended_at ? ` · Ended: ${run.ended_at}` : ""}
                          </div>
                          <div style={styles.smallText}>
                            Exit {run.exit_code ?? "n/a"} · Duration {formatDurationMs(run.duration_ms)} · stdout {formatInteger(run.stdout_chars)} chars · stderr {formatInteger(run.stderr_chars)} chars
                          </div>
                          {run.error_message && <div style={styles.warning}>{run.error_message}</div>}
                          <div style={{ display: "flex", gap: 8, marginTop: 8, flexWrap: "wrap" }}>
                            <button style={styles.ghostBtn} onClick={() => setSelectedExternalCliRunId(run.id)}>
                              View trace
                            </button>
                            {outputArtifact ? (
                              <button
                                style={styles.ghostBtn}
                                onClick={() => setArtifactModalArtifact(outputArtifact)}
                              >
                                Open captured output
                              </button>
                            ) : null}
                          </div>
                        </div>
                      );
                    })}
                  </div>
                ) : (
                  <div style={styles.detailValue}>No external CLI runs yet.</div>
                )}
              </div>
            </>
          )}

          {workItemWorkspaceTab === "review" && (
            <>
              <div style={styles.detailCard}>
                <div style={styles.detailLabel}>Workspace Readiness</div>
                {resolvedRepository ? (
                  <>
                    <div style={styles.detailValue}>{resolvedRepository.name}</div>
                    <div style={styles.smallText}>{resolvedRepository.local_path}</div>
                    <div style={styles.smallText}>
                      {resolvedRepository.remote_url
                        ? `Remote configured: ${resolvedRepository.remote_url}`
                        : "Remote: not configured"}
                    </div>
                    <div style={styles.smallText}>Branch: {selectedWorkItemSummary?.branch_name || resolvedRepository.default_branch}</div>
                    <div style={styles.smallText}>
                      Source: {selectedWorkItemSummary?.repo_override_id ? "story override" : "scope default"}
                    </div>
                    <div style={styles.smallText}>Version history: enabled</div>
                    {renderWorkspaceAssignmentPanel()}
                  </>
                ) : (
                  <>
                    <div style={styles.warning}>
                      No workspace is attached to the current story scope.
                    </div>
                    <div style={styles.smallText}>
                      Delivery stages will be blocked until a workspace exists for this scope.
                    </div>
                    <div style={{ marginTop: 10 }}>
                      <button
                        style={styles.btn}
                        onClick={() => createWorkspaceMutation.mutate()}
                        disabled={createWorkspaceMutation.isPending}
                      >
                        {createWorkspaceMutation.isPending ? "Creating Workspace..." : "Create Workspace"}
                      </button>
                    </div>
                    {renderWorkspaceAssignmentPanel()}
                  </>
                )}
              </div>
              <div style={styles.sectionTitle}>Review Signals</div>
              <div style={styles.detailCard}>
                <div style={styles.detailLabel}>Workflow</div>
                {workflowRunId ? (
                  <>
                    <div style={styles.detailValue}>
                      Run: <code>{workflowRunId}</code>
                    </div>
                    <div style={styles.smallText}>
                      Stage: {stageLabel ?? "unknown"} · Status: {latestWorkflowRun?.status ?? "unknown"}
                    </div>
                    {workflowElapsedLabel && (
                      <div style={styles.smallText}>
                        Active stage elapsed: {workflowElapsedLabel}
                      </div>
                    )}
                    {isStaleRun && (
                      <div style={styles.infoCard}>
                        <div style={styles.detailValue}>
                          This run appears stale. No completion/error has been recorded for the active stage.
                        </div>
                        <div style={{ display: "flex", gap: 8, marginTop: 10, flexWrap: "wrap" }}>
                          <button
                            style={styles.btnDanger}
                            onClick={() => failWorkflowRunMutation.mutate()}
                          >
                            {failWorkflowRunMutation.isPending ? "Failing..." : "Mark Run Failed"}
                          </button>
                          <button
                            style={styles.btn}
                            onClick={() => restartWorkflowMutation.mutate()}
                          >
                            {restartWorkflowMutation.isPending ? "Restarting..." : "Restart Workflow"}
                          </button>
                        </div>
                      </div>
                    )}
                    {activeWorkflowStage === "pending_plan_approval" && (
                      <div style={{ display: "flex", gap: 8, marginTop: 10, flexWrap: "wrap" }}>
                        <button style={{ ...styles.btn, backgroundColor: "#2d6a3f" }} onClick={() => planApprovalMutation.mutate()}>
                          {planApprovalMutation.isPending ? "Approving Plan..." : "Approve Plan"}
                        </button>
                        <button style={styles.btnDanger} onClick={() => planRejectMutation.mutate()}>
                          {planRejectMutation.isPending ? "Rejecting..." : "Reject Plan"}
                        </button>
                      </div>
                    )}
                    <div style={{ marginTop: 12, paddingTop: 10, borderTop: "1px solid #32353d" }}>
                      <div style={styles.detailLabel}>Cost Visibility</div>
                      <div style={{ display: "flex", flexWrap: "wrap", gap: 12 }}>
                        <span style={styles.smallText}>Calls: {formatInteger(workflowModelUsage.callCount)}</span>
                        <span style={styles.smallText}>Failed: {formatInteger(workflowModelUsage.failedCallCount)}</span>
                        <span style={styles.smallText}>Input tokens: {formatInteger(workflowModelUsage.inputTokens)}</span>
                        <span style={styles.smallText}>Output tokens: {formatInteger(workflowModelUsage.outputTokens)}</span>
                        <span style={styles.smallText}>Prompt chars: {formatInteger(workflowModelUsage.promptChars)}</span>
                        <span style={styles.smallText}>Response chars: {formatInteger(workflowModelUsage.responseChars)}</span>
                        <span style={styles.smallText}>Model time: {formatDurationMs(workflowModelUsage.durationMs)}</span>
                      </div>
                      {workflowModelUsage.providerLabels.length > 0 && (
                        <div style={styles.smallText}>
                          Providers: {workflowModelUsage.providerLabels.slice(0, 3).join(", ")}
                          {workflowModelUsage.providerLabels.length > 3 ? ` +${workflowModelUsage.providerLabels.length - 3} more` : ""}
                        </div>
                      )}
                      <div style={styles.smallText}>
                        {workflowModelUsage.source === "per_call"
                          ? "Source: per-call telemetry. Dollar estimate is hidden until provider pricing is configured."
                          : workflowModelUsage.source === "agent_run"
                            ? "Source: legacy agent-run token totals. Per-call telemetry starts with new model calls."
                            : "No token or call telemetry has been recorded for this workflow yet."}
                      </div>
                    </div>
                    <div style={{ marginTop: 12 }}>
                      <div style={styles.detailLabel}>Stage Artifacts</div>
                      <div style={styles.dagLegend}>
                        <span style={styles.dagLegendItem}><span style={{ width: 8, height: 8, borderRadius: 99, backgroundColor: "#4ec9b0", display: "inline-block" }} /> done</span>
                        <span style={styles.dagLegendItem}><span style={{ width: 8, height: 8, borderRadius: 99, backgroundColor: "#0e639c", display: "inline-block" }} /> active</span>
                        <span style={styles.dagLegendItem}><span style={{ width: 8, height: 8, borderRadius: 99, backgroundColor: "#3f4a59", display: "inline-block" }} /> pending</span>
                      </div>
                      <div style={styles.dagWrap}>
                        <svg width={2520} height={260} viewBox="0 0 2520 260" role="img" aria-label="Workflow DAG">
                          {WORKFLOW_DAG_LANES.map((lane) => (
                            <g key={lane.id}>
                              <rect
                                x={lane.x}
                                y={lane.y}
                                width={lane.width}
                                height={lane.height}
                                rx={12}
                                fill="#131821"
                                stroke="#273140"
                                strokeWidth={1}
                              />
                              <text
                                x={lane.x + 14}
                                y={lane.y + 22}
                                fill="#8f96a3"
                                fontSize={11}
                                fontWeight={700}
                                letterSpacing={0.8}
                              >
                                {lane.label}
                              </text>
                              {(() => {
                                const summary = laneStatusById.get(lane.id);
                                if (!summary) return null;
                                const parts = [
                                  `done ${summary.done}`,
                                  `active ${summary.active}`,
                                  `pending ${summary.pending}`,
                                ];
                                if (summary.failed > 0) {
                                  parts.push(`failed ${summary.failed}`);
                                }
                                return (
                                  <text
                                    x={lane.x + lane.width - 14}
                                    y={lane.y + 22}
                                    fill={summary.failed > 0 ? "#ff9b9b" : "#6f7b8e"}
                                    fontSize={10}
                                    fontWeight={600}
                                    textAnchor="end"
                                  >
                                    {parts.join(" · ")}
                                  </text>
                                );
                              })()}
                            </g>
                          ))}
                          {WORKFLOW_DAG_LINKS.map(([from, to]) => {
                            const fromNode = dagNodeById.get(from);
                            const toNode = dagNodeById.get(to);
                            if (!fromNode || !toNode) return null;
                            return (
                              <line
                                key={`${from}-${to}`}
                                x1={fromNode.x + (fromNode.kind ? 20 : 52)}
                                y1={fromNode.y}
                                x2={toNode.x - (toNode.kind ? 20 : 52)}
                                y2={toNode.y}
                                stroke="#3c4048"
                                strokeWidth={2}
                              />
                            );
                          })}
                          {WORKFLOW_DAG_NODES.map((node) => {
                            const hasActualStages = node.actualStageIds.length > 0;
                            const isDone = hasActualStages && node.actualStageIds.every((stageId) => completedStages.has(stageId));
                            const isActive = hasActualStages && node.actualStageIds.includes(activeWorkflowStage ?? "");
                            const isSelected = selectedDagNodeId === node.id;
                            const fill = isDone ? "#2d6a3f" : isActive ? "#0e639c" : node.kind ? "#232833" : "#2c3139";
                            const stroke = isSelected ? "#8ecbff" : isDone ? "#4ec9b0" : isActive ? "#57b0e5" : "#3c4048";
                            return (
                              <g key={node.id} onClick={() => setSelectedArtifactStage(node.id)} style={{ cursor: "pointer" }}>
                                {node.kind ? (
                                  <>
                                    <polygon
                                      points={`${node.x},${node.y - 22} ${node.x + 22},${node.y} ${node.x},${node.y + 22} ${node.x - 22},${node.y}`}
                                      fill={fill}
                                      stroke={stroke}
                                      strokeWidth={2}
                                    />
                                    <text x={node.x} y={node.y + 38} textAnchor="middle" fill="#e8edf7" fontSize={10} fontWeight={700}>
                                      {node.label}
                                    </text>
                                  </>
                                ) : (
                                  <>
                                    <rect x={node.x - 52} y={node.y - 20} width={104} height={40} rx={8} fill={fill} stroke={stroke} strokeWidth={2} />
                                    <text x={node.x} y={node.y + 4} textAnchor="middle" fill="#e8edf7" fontSize={10} fontWeight={700}>
                                      {node.label}
                                    </text>
                                  </>
                                )}
                              </g>
                            );
                          })}
                        </svg>
                      </div>
                      <div style={styles.smallText}>
                        Selected node: <strong>{selectedDagNode.label}</strong>
                        {selectedDagNode.actualStageIds.length > 0 ? ` · Runtime stages: ${selectedDagNode.actualStageIds.join(", ")}` : " · Structural split/merge node"}
                      </div>
                    </div>

                    <div style={{ marginTop: 12 }}>
                      <div style={styles.detailLabel}>Selected Stage Details</div>
                      <div style={styles.detailCard}>
                        <div style={styles.detailLabel}>Node</div>
                        <div style={styles.detailValue}>{selectedDagNode.label}</div>
                        <div style={styles.smallText}>
                          {selectedDagNode.actualStageIds.length > 0
                            ? `Backed by runtime stage${selectedDagNode.actualStageIds.length > 1 ? "s" : ""}: ${selectedDagNode.actualStageIds.join(", ")}`
                            : "This is a structural split/merge node used to explain the orchestrated flow."}
                        </div>

                        <div style={{ ...styles.detailLabel, marginTop: 12 }}>Stage Agent Runs</div>
                        {selectedDagNode.actualStageIds.length === 0 ? (
                          <div style={styles.smallText}>No direct agent run is attached to this structural node.</div>
                        ) : stageRuns.length > 0 ? (
                          <div style={styles.list}>
                            {stageRuns.map((run: AgentRun) => {
                              const runArtifacts = (artifactsByAgentRunId.get(run.id) ?? []).sort((a, b) =>
                                a.created_at.localeCompare(b.created_at),
                              );
                              const runCalls = modelCallsByAgentRunId.get(run.id) ?? [];
                              const runUsage = summarizeModelUsage(runCalls, [run]);
                              const visibleRunCalls = runCalls.slice(-8);
                              return (
                                <div key={run.id} style={styles.listItem}>
                                  <div style={styles.taskTitle}>{run.status} · {run.agent_id}</div>
                                  <div style={styles.smallText}>
                                    Run: {run.id}
                                  </div>
                                  <div style={styles.smallText}>Stage: {run.stage}</div>
                                  <div style={styles.smallText}>
                                    Started: {run.started_at}{run.ended_at ? ` · Ended: ${run.ended_at}` : ""}
                                  </div>
                                  <div style={styles.smallText}>
                                    Usage: calls {formatInteger(runUsage.callCount)} · failed {formatInteger(runUsage.failedCallCount)} · input {formatInteger(runUsage.inputTokens)} · output {formatInteger(runUsage.outputTokens)} · model time {formatDurationMs(runUsage.durationMs)}
                                  </div>
                                  {runUsage.source === "agent_run" && (
                                    <div style={styles.smallText}>Per-call rows were not recorded for this older run; showing aggregate run tokens.</div>
                                  )}
                                  {visibleRunCalls.length > 0 && (
                                    <div style={{ marginTop: 8, paddingTop: 8, borderTop: "1px solid #2d3139" }}>
                                      <div style={styles.detailLabel}>Model Calls</div>
                                      <div style={styles.list}>
                                        {visibleRunCalls.map((call) => (
                                          <div key={call.id} style={{ borderLeft: "2px solid #36506f", paddingLeft: 8 }}>
                                            <div style={styles.smallText}>
                                              #{call.call_index} · {call.status} · {call.provider_name || call.provider_id} / {call.model_name}
                                            </div>
                                            <div style={styles.smallText}>
                                              Input {formatInteger(call.token_count_input)} · Output {formatInteger(call.token_count_output)} · Prompt {formatInteger(call.prompt_chars)} chars · Response {formatInteger(call.response_chars)} chars · Max {formatInteger(call.max_tokens)} · {formatDurationMs(call.duration_ms)}
                                            </div>
                                            {call.error_message && <div style={styles.warning}>{call.error_message}</div>}
                                          </div>
                                        ))}
                                      </div>
                                      {runCalls.length > visibleRunCalls.length && (
                                        <div style={styles.smallText}>
                                          Showing latest {visibleRunCalls.length} of {runCalls.length} model calls.
                                        </div>
                                      )}
                                    </div>
                                  )}
                                  {run.error_message && <div style={styles.warning}>{run.error_message}</div>}
                                  <div style={{ ...styles.detailLabel, marginTop: 8 }}>Input / Output / Attachments</div>
                                  {runArtifacts.length > 0 ? (
                                    <div style={styles.list}>
                                      {runArtifacts.map((artifact) => (
                                        <button
                                          key={artifact.id}
                                          style={{ ...styles.ghostBtn, textAlign: "left", width: "100%" }}
                                          onClick={() => setArtifactModalArtifact(artifact)}
                                        >
                                          {getArtifactFileName(artifact)} · {artifact.artifact_type}
                                        </button>
                                      ))}
                                    </div>
                                  ) : (
                                    <div style={styles.warning}>No attachments generated for this run yet.</div>
                                  )}
                                </div>
                              );
                            })}
                          </div>
                        ) : (
                          <div style={styles.warning}>No agent runs recorded for this stage yet.</div>
                        )}

                        <div style={{ ...styles.detailLabel, marginTop: 12 }}>Stage Transition History</div>
                        {selectedDagNode.actualStageIds.length === 0 ? (
                          <div style={styles.smallText}>No direct transition history is attached to this structural node.</div>
                        ) : stageHistoryForFocusedStage.length > 0 ? (
                          <div style={styles.list}>
                            {stageHistoryForFocusedStage.slice(-8).map((entry: WorkflowStageHistory) => (
                              <div key={entry.id} style={styles.listItem}>
                                <div style={styles.taskTitle}>
                                  {entry.from_stage.replace(/_/g, " ")} → {entry.to_stage.replace(/_/g, " ")}
                                </div>
                                <div style={styles.smallText}>{entry.trigger} · {entry.transitioned_at}</div>
                                <div style={styles.smallText}>{entry.notes}</div>
                              </div>
                            ))}
                          </div>
                        ) : (
                          <div style={styles.warning}>No transitions recorded for this stage yet.</div>
                        )}
                      </div>
                    </div>

                    {activeWorkflowStage === "pending_test_review" && (
                      <div style={{ display: "flex", gap: 8, marginTop: 10, flexWrap: "wrap" }}>
                        <button style={{ ...styles.btn, backgroundColor: "#2d6a3f" }} onClick={() => testReviewApproveMutation.mutate()}>
                          {testReviewApproveMutation.isPending ? "Approving Tests..." : "Approve Test Review"}
                        </button>
                        <button style={styles.btnDanger} onClick={() => testReviewRejectMutation.mutate()}>
                          {testReviewRejectMutation.isPending ? "Rejecting..." : "Reject Test Review"}
                        </button>
                      </div>
                    )}
                  </>
                ) : (
                  <div style={styles.detailValue}>No workflow run yet. Start a workflow from the Story Detail tab.</div>
                )}
              </div>
              <div style={styles.detailCard}>
                <div style={styles.detailLabel}>Tasks</div>
              {subWorkItems && subWorkItems.length > 0 ? (
                <div style={styles.list}>
                    {subWorkItems.map((workItem: WorkItem) => (
                      <div key={workItem.id} style={styles.listItem}>
                        <div style={styles.taskTitle}>{workItem.title}</div>
                        <div style={styles.smallText}>{workItem.status.replace(/_/g, " ")} · {workItem.work_item_type}</div>
                      </div>
                    ))}
                </div>
              ) : (
                  <div style={styles.detailValue}>No tasks yet.</div>
              )}
              </div>
              <div style={styles.detailCard}>
                <div style={styles.detailLabel}>Summary</div>
                <div style={styles.list}>
                  <div style={styles.listItem}>
                    <div style={styles.taskTitle}>Approvals</div>
                    <div style={styles.smallText}>{approvals?.length ?? 0} records</div>
                    {latestApproval && (
                      <div style={styles.smallText}>
                        Latest: {latestApproval.approval_type} · {latestApproval.status} · {latestApproval.created_at}
                      </div>
                    )}
                  </div>
                  <div style={styles.listItem}>
                    <div style={styles.taskTitle}>Artifacts</div>
                    <div style={styles.smallText}>{artifacts?.length ?? 0} generated</div>
                    {latestArtifact && (
                      <div style={styles.smallText}>
                        Latest: {latestArtifact.artifact_type} · {latestArtifact.created_at}
                      </div>
                    )}
                    {topArtifactTypes.length > 0 && (
                      <div style={styles.smallText}>
                        Top types: {topArtifactTypes.map(([kind, count]) => `${kind} (${count})`).join(", ")}
                      </div>
                    )}
                  </div>
                  <div style={styles.listItem}>
                    <div style={styles.taskTitle}>Findings</div>
                    <div style={styles.smallText}>{findings?.length ?? 0} logged</div>
                    {(findings?.length ?? 0) > 0 && (
                      <div style={styles.smallText}>
                        Severity: {["critical", "high", "medium", "low", "info"]
                          .filter((severity) => (findingSeverityCounts.get(severity) ?? 0) > 0)
                          .map((severity) => `${severity} (${findingSeverityCounts.get(severity)})`)
                          .join(", ")}
                      </div>
                    )}
                  </div>
                </div>
              </div>
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
