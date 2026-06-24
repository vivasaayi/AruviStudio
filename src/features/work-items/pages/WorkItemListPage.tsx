import React, { useMemo, useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  approveWorkItem,
  approveWorkItemPlan,
  approveWorkItemTestReview,
  getLatestWorkflowRunForWorkItem,
  getWorkflowHistory,
  handleWorkflowUserAction,
  getSubWorkItems,
  getWorkItemApprovals,
  listWorkItemArtifacts,
  listWorkItemFindings,
  listAgentDefinitions,
  listAgentModelCallsForWorkflow,
  listAgentRunsForWorkflow,
  invokeExternalCliForWorkItem,
  listExternalCliRunsForWorkItem,
  listAgentModelBindings,
  listAgentTeams,
  listModelDefinitions,
  listProviders,
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
} from "../../../lib/tauri";
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
import { useWorkItemBacklogApprovalActions } from "../hooks/useWorkItemBacklogApprovalActions";
import { useWorkItemBacklogView } from "../hooks/useWorkItemBacklogView";
import { useWorkItemCrudMutations } from "../hooks/useWorkItemCrudMutations";
import { useWorkItemPageSync } from "../hooks/useWorkItemPageSync";
import { useWorkItemReviewSignals } from "../hooks/useWorkItemReviewSignals";
import { useWorkItemScopeData } from "../hooks/useWorkItemScopeData";
import { useWorkItemScopeDisplay } from "../hooks/useWorkItemScopeDisplay";
import { useWorkItemWorkspaceEditor } from "../hooks/useWorkItemWorkspaceEditor";
import { useWorkItemWorkspaceMutations } from "../hooks/useWorkItemWorkspaceMutations";
import { styles } from "../lib/workItemListPageStyles";
import {
  SUB_WORK_ITEM_PAGE_SIZE,
  type ExternalCliProvider,
} from "../lib/workItemListPageHelpers";
import type {
  Approval,
  Artifact,
  Finding,
  WorkItem,
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

  const {
    products,
    productsLoading,
    selectedProductId,
    workItemsScopeQueryKey,
    workItemsQueryKey,
    workItemPage,
    isLoading,
    productAreaById,
    capabilityById,
    repositories,
    filteredWorkItems,
    selectedWorkItemId,
    selectedWorkItem,
  } = useWorkItemScopeData({
    activeProductId,
    activeProductAreaId,
    activeNodeId,
    activeNodeType,
    activeWorkItemId,
    statusFilter,
    workItemPageIndex,
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

  useWorkItemPageSync({
    productsLoading,
    activeProductId,
    selectedProductId,
    setActiveProduct,
    selectedProductScopeId: selectedProductId,
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
  });

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

  const {
    createMutation,
    updateWorkItemMutation,
    deleteMutation,
  } = useWorkItemCrudMutations({
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
    activeProductSidebarQueryKey: ["sidebarWorkItems", activeProductId],
    workItemPageIndex,
    invalidateTasks,
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
  const {
    isEditingWorkspace,
    setIsEditingWorkspace,
    workspaceRepositoryId,
    workspaceBranchMode,
    setWorkspaceBranchMode,
    workspaceBranchName,
    setWorkspaceBranchName,
    branchPreview,
    openWorkspaceEditor,
    selectWorkspaceRepository,
  } = useWorkItemWorkspaceEditor({
    repositories,
    selectedWorkItem: selectedWorkItemSummary,
    resolvedRepository,
  });
  const {
    createWorkspaceMutation,
    assignWorkspaceMutation,
    clearWorkspaceOverrideMutation,
  } = useWorkItemWorkspaceMutations({
    queryClient,
    activeProductId,
    activeProductAreaId,
    selectedWorkItemId,
    selectedWorkItem: selectedWorkItemSummary,
    repositories,
    workspaceRepositoryId,
    branchPreview,
    workItemsScopeQueryKey,
    workItemsQueryKey,
    setActionError,
    setActionInfo,
    setIsEditingWorkspace,
    setActiveView,
    invalidateTasks,
  });
  const {
    selectedDagNodeId,
    workflowElapsedLabel,
    isStaleRun,
    selectedDagNode,
    stageRuns,
    artifactsByAgentRunId,
    modelCallsByAgentRunId,
    workflowModelUsage,
    completedStages,
    dagNodeById,
    laneStatusById,
    latestApproval,
    latestArtifact,
    latestExternalCliRun,
    activeExternalCliRunId,
    activeExternalCliRun,
    externalCliRunEvents,
    latestExternalCliEvent,
    externalCliTerminalOutput,
    findingSeverityCounts,
    topArtifactTypes,
    workflowReadiness,
    stageHistoryForFocusedStage,
  } = useWorkItemReviewSignals({
    selectedArtifactStage,
    activeWorkflowStage,
    latestWorkflowRun,
    workflowRunId,
    workflowHistory,
    agentRuns,
    agentModelCalls,
    artifacts,
    approvals,
    findings,
    externalCliRuns,
    selectedExternalCliRunId,
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
  });
  const {
    activeProduct,
    activeProductArea,
    activeCapability,
    scopeDescriptor,
    createWorkItemScopeLabel,
    workItemOwnerMap,
  } = useWorkItemScopeDisplay({
    products,
    activeProductId,
    activeProductAreaId,
    activeCapabilityId,
    productAreaById,
    capabilityById,
    filteredWorkItems,
    createParentWorkItemId: createForm.parentWorkItemId,
  });
  const {
    orderedWorkItems,
    backlogWindow,
    hasNextWorkItemPage,
    workItemPageStart,
    workItemPageEnd,
    backlogRenderedRangeLabel,
    selectedBacklogItems,
    latestWorkflowRunByWorkItemId,
  } = useWorkItemBacklogView({
    filteredWorkItems,
    workItemOrderIds,
    backlogScrollTop,
    backlogViewportHeight,
    workItemPage,
    workItemPageIndex,
    selectedBacklogItemIds,
    isBacklogActive: workItemWorkspaceTab === "backlog",
  });
  const {
    isRowActionPending,
    runRowApprovalAction,
    runBulkApprovalAction,
  } = useWorkItemBacklogApprovalActions({
    selectedBacklogItemIds,
    setSelectedBacklogItemIds,
    pendingRowActionIds,
    setPendingRowActionIds,
    bulkActionInFlight,
    setBulkActionInFlight,
    setActionError,
    invalidateTasks,
  });
  const stageLabel = activeWorkflowStage ? activeWorkflowStage.replace(/_/g, " ") : null;
  const externalCliProviderInFlight = externalCliMutation.isPending ? externalCliMutation.variables : null;

  const renderWorkspaceAssignmentPanel = () => (
    <WorkItemWorkspaceAssignmentPanel
      isEditing={isEditingWorkspace}
      repositories={repositories}
      workspaceRepositoryId={workspaceRepositoryId}
      workspaceBranchMode={workspaceBranchMode}
      workspaceBranchName={workspaceBranchName}
      currentBranch={selectedWorkItemSummary?.branch_name || resolvedRepository?.default_branch || "not set"}
      branchPreview={branchPreview}
      hasWorkspaceOverride={!!selectedWorkItemSummary?.repo_override_id}
      isAssignPending={assignWorkspaceMutation.isPending}
      isClearPending={clearWorkspaceOverrideMutation.isPending}
      onOpenEditor={() => {
        setActionError(null);
        openWorkspaceEditor();
      }}
      onClearOverride={() => clearWorkspaceOverrideMutation.mutate()}
      onRepositoryIdChange={selectWorkspaceRepository}
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
