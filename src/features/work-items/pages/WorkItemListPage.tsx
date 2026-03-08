import React, { useEffect, useMemo, useState } from "react";
import { useMutation, useQueries, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  approveWorkItem,
  approveWorkItemPlan,
  approveWorkItemTestReview,
  createWorkItem,
  deleteWorkItem,
  getLatestWorkflowRunForWorkItem,
  getWorkflowHistory,
  handleWorkflowUserAction,
  getSubWorkItems,
  getWorkItem,
  getWorkItemApprovals,
  listWorkItemArtifacts,
  listWorkItemFindings,
  listWorkItems,
  listAgentDefinitions,
  listAgentRunsForWorkflow,
  listAgentModelBindings,
  listAgentTeams,
  listModelDefinitions,
  listProviders,
  listTeamAssignments,
  listTeamMemberships,
  listWorkflowStagePolicies,
  rejectWorkItemPlan,
  readArtifactContent,
  reorderWorkItems,
  markWorkflowRunFailed,
  rejectWorkItem,
  restartWorkflowRun,
  startWorkItemWorkflow,
  updateWorkItem,
} from "../../../lib/tauri";
import { useWorkspaceStore } from "../../../state/workspaceStore";
import { useUIStore } from "../../../state/uiStore";
import type {
  AgentDefinition,
  AgentModelBinding,
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
  WorkflowRun,
  WorkflowStageHistory,
  WorkflowStagePolicy,
} from "../../../lib/types";

const styles: Record<string, React.CSSProperties> = {
  page: { display: "flex", flexDirection: "column", gap: 12, height: "100%" },
  header: { display: "flex", justifyContent: "space-between", alignItems: "center", gap: 10 },
  titleBlock: { display: "flex", flexDirection: "column", gap: 3 },
  title: { fontSize: 18, fontWeight: 800, color: "#f3f3f3", margin: 0 },
  subtitle: { fontSize: 12, color: "#8f96a3" },
  panel: { backgroundColor: "#212327", border: "1px solid #32353d", borderRadius: 12, minHeight: 0, overflow: "hidden", flex: 1 },
  panelInner: { padding: 14, height: "100%", overflow: "auto" },
  tabBar: { display: "flex", gap: 8, marginBottom: 14, borderBottom: "1px solid #32353d", paddingBottom: 10 },
  tab: { padding: "7px 12px", fontSize: 12, fontWeight: 700, borderRadius: 8, border: "1px solid #3b4049", backgroundColor: "#2c3139", color: "#cfd6e4", cursor: "pointer" },
  tabActive: { padding: "7px 12px", fontSize: 12, fontWeight: 700, borderRadius: 8, border: "1px solid #0e639c", backgroundColor: "#173247", color: "#ffffff", cursor: "pointer" },
  btn: { padding: "7px 12px", fontSize: 12, backgroundColor: "#0e639c", color: "#fff", border: "none", borderRadius: 8, cursor: "pointer" },
  btnDanger: { padding: "6px 10px", fontSize: 12, backgroundColor: "#6c2020", color: "#fff", border: "none", borderRadius: 8, cursor: "pointer" },
  ghostBtn: { padding: "6px 10px", fontSize: 12, backgroundColor: "#2c3139", color: "#e0e0e0", border: "1px solid #3b4049", borderRadius: 8, cursor: "pointer" },
  sectionTitle: { fontSize: 12, fontWeight: 700, letterSpacing: 1, textTransform: "uppercase" as const, color: "#8f96a3", marginBottom: 10, display: "flex", alignItems: "center", justifyContent: "space-between" },
  filterSelect: { padding: "7px 10px", fontSize: 12, backgroundColor: "#181a1f", border: "1px solid #3c4048", borderRadius: 8, color: "#e0e0e0", width: "100%", marginBottom: 12 },
  taskList: { display: "flex", flexDirection: "column", gap: 8 },
  taskCard: { padding: 12, border: "1px solid #32353d", borderRadius: 10, backgroundColor: "#26292f", cursor: "pointer" },
  taskCardActive: { padding: 12, border: "1px solid #0e639c", borderRadius: 10, background: "linear-gradient(135deg, rgba(14,99,156,0.18), rgba(38,41,47,1))", cursor: "pointer" },
  taskRowCard: { display: "grid", gridTemplateColumns: "minmax(0, 1.6fr) minmax(260px, 1fr) auto", gap: 12, alignItems: "center" },
  taskMain: { display: "flex", flexDirection: "column", gap: 4, minWidth: 0 },
  taskTitle: { fontSize: 13, fontWeight: 700, color: "#f3f3f3", margin: 0, whiteSpace: "nowrap" as const, overflow: "hidden", textOverflow: "ellipsis" },
  taskMeta: { fontSize: 11, color: "#8f96a3", display: "flex", gap: 8, flexWrap: "wrap" },
  badge: { fontSize: 11, padding: "2px 8px", borderRadius: 10, display: "inline-block" },
  badgeRow: { display: "flex", gap: 8, flexWrap: "wrap" as const, alignItems: "center" },
  taskStatusLine: { fontSize: 12, color: "#cfd6e4", display: "flex", flexDirection: "column", gap: 4, minWidth: 0 },
  taskStatusSummary: { whiteSpace: "nowrap" as const, overflow: "hidden", textOverflow: "ellipsis" },
  taskActions: { display: "flex", gap: 8, alignItems: "center", justifyContent: "flex-end" },
  overflowWrap: { position: "relative" as const },
  overflowMenu: { position: "absolute" as const, top: "calc(100% + 6px)", right: 0, minWidth: 140, backgroundColor: "#1b1d22", border: "1px solid #32353d", borderRadius: 10, boxShadow: "0 12px 28px rgba(0,0,0,0.35)", padding: 6, zIndex: 10, display: "flex", flexDirection: "column", gap: 4 },
  overflowMenuItem: { padding: "7px 10px", fontSize: 12, backgroundColor: "transparent", color: "#e0e0e0", border: "none", borderRadius: 8, textAlign: "left" as const, cursor: "pointer" },
  overflowMenuItemDanger: { padding: "7px 10px", fontSize: 12, backgroundColor: "transparent", color: "#ff9b9b", border: "none", borderRadius: 8, textAlign: "left" as const, cursor: "pointer" },
  detailTitle: { fontSize: 22, fontWeight: 800, color: "#ffffff", marginBottom: 8 },
  detailCard: { backgroundColor: "#26292f", border: "1px solid #32353d", borderRadius: 12, padding: 14, marginBottom: 12 },
  detailLabel: { fontSize: 11, color: "#8f96a3", textTransform: "uppercase" as const, marginBottom: 4 },
  detailValue: { fontSize: 13, color: "#f3f3f3", lineHeight: 1.5 },
  input: { width: "100%", padding: "9px 12px", backgroundColor: "#181a1f", border: "1px solid #3c4048", borderRadius: 8, color: "#e0e0e0", fontSize: 13, marginBottom: 10, boxSizing: "border-box" as const },
  textarea: { width: "100%", padding: "9px 12px", backgroundColor: "#181a1f", border: "1px solid #3c4048", borderRadius: 8, color: "#e0e0e0", fontSize: 13, marginBottom: 10, minHeight: 76, resize: "vertical" as const, boxSizing: "border-box" as const },
  row: { display: "grid", gridTemplateColumns: "1fr 1fr", gap: 10 },
  warning: { color: "#ff7b72", fontSize: 12, marginBottom: 10 },
  previewBox: { whiteSpace: "pre-wrap" as const, fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace", fontSize: 12, lineHeight: 1.5, color: "#d7deea", backgroundColor: "#181a1f", border: "1px solid #3c4048", borderRadius: 8, padding: 10, maxHeight: 320, overflow: "auto" as const },
  dagWrap: { border: "1px solid #3c4048", borderRadius: 8, backgroundColor: "#181a1f", overflowX: "auto" as const, padding: 8, marginBottom: 10 },
  dagLegend: { display: "flex", gap: 10, flexWrap: "wrap" as const, marginBottom: 8, fontSize: 11, color: "#8f96a3" },
  dagLegendItem: { display: "inline-flex", alignItems: "center", gap: 6 },
  readinessCard: { backgroundColor: "#1f242d", border: "1px solid #323f52", borderRadius: 12, padding: 12, marginBottom: 12 },
  readinessHeading: { fontSize: 12, fontWeight: 700, color: "#d8e6ff", marginBottom: 8, textTransform: "uppercase" as const, letterSpacing: 0.8 },
  readinessItem: { fontSize: 12, color: "#d0d7e4", marginBottom: 5 },
  readinessOk: { color: "#4ec9b0", fontWeight: 700 as const },
  readinessWarn: { color: "#d7ba7d", fontWeight: 700 as const },
  readinessBlocker: { color: "#ff7b72", fontWeight: 700 as const },
  list: { display: "flex", flexDirection: "column", gap: 8 },
  listItem: { backgroundColor: "#1b1d22", border: "1px solid #2d3139", borderRadius: 10, padding: 10 },
  smallText: { fontSize: 11, color: "#8f96a3", marginTop: 4 },
  empty: { textAlign: "center" as const, color: "#666", padding: 40, fontSize: 14 },
  dropTarget: { outline: "1px dashed #0e639c", outlineOffset: 2 },
  dragHandle: { fontSize: 13, color: "#8f96a3", cursor: "grab", userSelect: "none" as const, padding: "2px 4px" },
  modalBackdrop: { position: "fixed", inset: 0, backgroundColor: "rgba(8, 10, 14, 0.72)", display: "flex", alignItems: "center", justifyContent: "center", padding: 24, zIndex: 40 },
  modal: { width: "min(760px, 100%)", maxHeight: "80vh", backgroundColor: "#212327", border: "1px solid #32353d", borderRadius: 14, overflow: "hidden", boxShadow: "0 24px 60px rgba(0,0,0,0.45)" },
  modalHeader: { display: "flex", alignItems: "center", justifyContent: "space-between", gap: 8, padding: "14px 16px", borderBottom: "1px solid #32353d" },
  modalTitle: { fontSize: 14, fontWeight: 800, color: "#f3f3f3" },
  modalBody: { padding: 16, maxHeight: "calc(80vh - 61px)", overflow: "auto" },
  errorText: { fontSize: 12, color: "#ff7b72", marginBottom: 10 },
  infoCard: { backgroundColor: "#1b2330", border: "1px solid #32445e", borderRadius: 10, padding: 10, marginTop: 10 },
};

const statusColors: Record<string, string> = {
  draft: "#444",
  ready_for_review: "#569cd6",
  approved: "#4ec9b0",
  in_planning: "#dcdcaa",
  in_progress: "#ce9178",
  in_validation: "#c586c0",
  waiting_human_review: "#d7ba7d",
  done: "#6a9955",
  blocked: "#f44747",
  failed: "#f44747",
  cancelled: "#666",
};

type WorkflowDagNode = {
  id: string;
  label: string;
  x: number;
  y: number;
  kind?: "stage" | "split" | "merge";
  actualStageIds: string[];
};

type WorkflowDagLane = {
  id: string;
  label: string;
  x: number;
  y: number;
  width: number;
  height: number;
  nodeIds: string[];
};

const WORKFLOW_DAG_NODES: WorkflowDagNode[] = [
  { id: "draft", label: "Draft", x: 90, y: 120, actualStageIds: ["draft"] },
  { id: "requirement_analysis", label: "Requirement Analysis", x: 280, y: 120, actualStageIds: ["requirement_analysis"] },
  { id: "planning_split", label: "Plan Split", x: 470, y: 120, kind: "split", actualStageIds: [] },
  { id: "architecture_plan", label: "Architecture Plan", x: 660, y: 40, actualStageIds: ["planning"] },
  { id: "unit_test_plan", label: "Unit Test Plan", x: 660, y: 100, actualStageIds: ["planning"] },
  { id: "integration_test_plan", label: "Integration Plan", x: 660, y: 160, actualStageIds: ["planning"] },
  { id: "ui_test_plan", label: "UI Test Plan", x: 660, y: 220, actualStageIds: ["planning"] },
  { id: "planning_merge", label: "Lead Merge", x: 860, y: 120, kind: "merge", actualStageIds: ["pending_plan_approval"] },
  { id: "coding", label: "Coding", x: 1060, y: 120, actualStageIds: ["coding"] },
  { id: "verification_split", label: "Verify Split", x: 1260, y: 120, kind: "split", actualStageIds: [] },
  { id: "unit_test_generation", label: "Unit Tests", x: 1460, y: 40, actualStageIds: ["unit_test_generation"] },
  { id: "integration_test_generation", label: "Integration Tests", x: 1460, y: 100, actualStageIds: ["integration_test_generation"] },
  { id: "ui_test_planning", label: "UI Verification", x: 1460, y: 160, actualStageIds: ["ui_test_planning"] },
  { id: "qa_validation", label: "QA Validation", x: 1660, y: 40, actualStageIds: ["qa_validation"] },
  { id: "security_review", label: "Security Review", x: 1660, y: 100, actualStageIds: ["security_review"] },
  { id: "performance_review", label: "Performance Review", x: 1660, y: 160, actualStageIds: ["performance_review"] },
  { id: "verification_merge", label: "Test Review", x: 1860, y: 120, kind: "merge", actualStageIds: ["pending_test_review"] },
  { id: "push_preparation", label: "Push Prep", x: 2060, y: 120, actualStageIds: ["push_preparation"] },
  { id: "git_push", label: "Git Push", x: 2240, y: 120, actualStageIds: ["git_push"] },
  { id: "done", label: "Done", x: 2420, y: 120, actualStageIds: ["done"] },
];

const WORKFLOW_DAG_LINKS: Array<[string, string]> = [
  ["draft", "requirement_analysis"],
  ["requirement_analysis", "planning_split"],
  ["planning_split", "architecture_plan"],
  ["planning_split", "unit_test_plan"],
  ["planning_split", "integration_test_plan"],
  ["planning_split", "ui_test_plan"],
  ["architecture_plan", "planning_merge"],
  ["unit_test_plan", "planning_merge"],
  ["integration_test_plan", "planning_merge"],
  ["ui_test_plan", "planning_merge"],
  ["planning_merge", "coding"],
  ["coding", "verification_split"],
  ["verification_split", "unit_test_generation"],
  ["verification_split", "integration_test_generation"],
  ["verification_split", "ui_test_planning"],
  ["verification_split", "qa_validation"],
  ["verification_split", "security_review"],
  ["verification_split", "performance_review"],
  ["unit_test_generation", "verification_merge"],
  ["integration_test_generation", "verification_merge"],
  ["ui_test_planning", "verification_merge"],
  ["qa_validation", "verification_merge"],
  ["security_review", "verification_merge"],
  ["performance_review", "verification_merge"],
  ["verification_merge", "push_preparation"],
  ["push_preparation", "git_push"],
  ["git_push", "done"],
];

const WORKFLOW_DAG_LANES: WorkflowDagLane[] = [
  { id: "intake", label: "Intake", x: 20, y: 12, width: 360, height: 236, nodeIds: ["draft", "requirement_analysis"] },
  { id: "planning_swarm", label: "Planning Swarm", x: 400, y: 12, width: 540, height: 236, nodeIds: ["planning_split", "architecture_plan", "unit_test_plan", "integration_test_plan", "ui_test_plan", "planning_merge"] },
  { id: "execution", label: "Execution", x: 960, y: 12, width: 260, height: 236, nodeIds: ["coding"] },
  { id: "verification_swarm", label: "Verification Swarm", x: 1240, y: 12, width: 660, height: 236, nodeIds: ["verification_split", "unit_test_generation", "integration_test_generation", "ui_test_planning", "qa_validation", "security_review", "performance_review", "verification_merge"] },
  { id: "delivery", label: "Delivery", x: 1920, y: 12, width: 560, height: 236, nodeIds: ["push_preparation", "git_push", "done"] },
];

function parseSqliteUtcTimestamp(value?: string | null): number | null {
  if (!value) return null;
  const parsed = Date.parse(value.includes("T") ? value : `${value.replace(" ", "T")}Z`);
  return Number.isFinite(parsed) ? parsed : null;
}

function getArtifactFileName(artifact: Artifact): string {
  const segments = artifact.storage_path.split(/[\\/]/).filter(Boolean);
  return segments[segments.length - 1] ?? artifact.artifact_type;
}

export function WorkItemListPage() {
  const queryClient = useQueryClient();
  const { activeProductId, activeModuleId, activeCapabilityId, activeWorkItemId, setActiveWorkItem } = useWorkspaceStore();
  const { workItemWorkspaceTab, setWorkItemWorkspaceTab, workItemCreateDialogOpen, openWorkItemCreateDialog, closeWorkItemCreateDialog } = useUIStore();

  const [statusFilter, setStatusFilter] = useState("");
  const [showCreateForm, setShowCreateForm] = useState(false);
  const [isEditingWorkItem, setIsEditingWorkItem] = useState(false);
  const [draggedWorkItemId, setDraggedWorkItemId] = useState<string | null>(null);
  const [workItemOrderIds, setWorkItemOrderIds] = useState<string[]>([]);
  const [formError, setFormError] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [activeWorkflowRunId, setActiveWorkflowRunId] = useState<string | null>(null);
  const [selectedArtifactStage, setSelectedArtifactStage] = useState<string | null>(null);
  const [artifactModalArtifact, setArtifactModalArtifact] = useState<Artifact | null>(null);
  const [openOverflowWorkItemId, setOpenOverflowWorkItemId] = useState<string | null>(null);
  const [createForm, setCreateForm] = useState({
    title: "",
    problemStatement: "",
    description: "",
    acceptanceCriteria: "",
    constraints: "",
    workItemType: "feature",
    priority: "medium",
    complexity: "medium",
  });
  const [workItemDraft, setWorkItemDraft] = useState({
    title: "",
    description: "",
    status: "draft",
    problemStatement: "",
    acceptanceCriteria: "",
    constraints: "",
  });

  const { data: workItems, isLoading } = useQuery({
    queryKey: ["workItems", activeProductId, activeModuleId, activeCapabilityId, statusFilter],
    queryFn: () =>
      listWorkItems({
        productId: activeProductId ?? undefined,
        moduleId: activeCapabilityId ? undefined : activeModuleId ?? undefined,
        capabilityId: activeCapabilityId ?? undefined,
        status: statusFilter || undefined,
      }),
  });

  const selectedWorkItemId = activeWorkItemId ?? workItems?.[0]?.id ?? null;
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
  const { data: subWorkItems } = useQuery({ queryKey: ["subWorkItems", selectedWorkItemId], queryFn: () => getSubWorkItems(selectedWorkItemId!), enabled: !!selectedWorkItemId });
  const { data: approvals } = useQuery({ queryKey: ["approvals", selectedWorkItemId], queryFn: () => getWorkItemApprovals(selectedWorkItemId!), enabled: !!selectedWorkItemId });
  const { data: artifacts } = useQuery({
    queryKey: ["artifacts", selectedWorkItemId],
    queryFn: () => listWorkItemArtifacts(selectedWorkItemId!),
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

  useEffect(() => {
    if (!activeWorkItemId && workItems?.[0]?.id) {
      setActiveWorkItem(workItems[0].id);
    }
  }, [activeWorkItemId, setActiveWorkItem, workItems]);

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
    setWorkItemOrderIds((workItems ?? []).map((workItem) => workItem.id));
  }, [workItems]);

  useEffect(() => {
    if (showCreateForm || workItemCreateDialogOpen) {
      setFormError(null);
    }
  }, [showCreateForm, workItemCreateDialogOpen]);

  useEffect(() => {
    setActionError(null);
  }, [selectedWorkItemId]);

  useEffect(() => {
    setOpenOverflowWorkItemId(null);
  }, [selectedWorkItemId, workItemWorkspaceTab]);

  useEffect(() => {
    setActiveWorkflowRunId(null);
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

  const invalidateTasks = async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ["workItems", activeProductId, activeModuleId, activeCapabilityId, statusFilter] }),
      queryClient.invalidateQueries({ queryKey: ["sidebarWorkItems", activeProductId] }),
      queryClient.invalidateQueries({ queryKey: ["workItem", selectedWorkItemId] }),
      queryClient.invalidateQueries({ queryKey: ["latestWorkflowRun", selectedWorkItemId] }),
      queryClient.invalidateQueries({ queryKey: ["workflowHistory", workflowRunId] }),
      queryClient.invalidateQueries({ queryKey: ["agentRunsForWorkflow", workflowRunId] }),
      queryClient.invalidateQueries({ queryKey: ["artifacts", selectedWorkItemId] }),
      queryClient.invalidateQueries({ queryKey: ["findings", selectedWorkItemId] }),
      queryClient.refetchQueries({ queryKey: ["workItems", activeProductId, activeModuleId, activeCapabilityId, statusFilter], type: "active" }),
      queryClient.refetchQueries({ queryKey: ["sidebarWorkItems", activeProductId], type: "active" }),
    ]);
  };

  const createMutation = useMutation({
    mutationFn: () =>
      createWorkItem({
        productId: activeProductId || "",
        moduleId: activeModuleId ?? undefined,
        capabilityId: activeCapabilityId ?? undefined,
        title: createForm.title,
        problemStatement: createForm.problemStatement,
        description: createForm.description,
        acceptanceCriteria: createForm.acceptanceCriteria,
        constraints: createForm.constraints,
        workItemType: createForm.workItemType,
        priority: createForm.priority,
        complexity: createForm.complexity,
      }),
    onSuccess: async (createdWorkItem) => {
      queryClient.setQueryData<WorkItem[] | undefined>(["workItems", activeProductId, activeModuleId, activeCapabilityId, statusFilter], (current) =>
        current ? [...current, createdWorkItem] : [createdWorkItem],
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
        workItemType: "feature",
        priority: "medium",
        complexity: "medium",
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
    mutationFn: () => approveWorkItem(selectedWorkItemId!, "Approved from work item workspace"),
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
    mutationFn: () => rejectWorkItem(selectedWorkItemId!, "Rejected from work item workspace"),
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
  const planApprovalMutation = useMutation({
    mutationFn: async () => {
      if (!selectedWorkItemId || !workflowRunId) {
        throw new Error("No workflow run available for plan approval.");
      }
      await approveWorkItemPlan(selectedWorkItemId, "Plan approved from work item workspace");
      await handleWorkflowUserAction({
        workflowRunId,
        action: "approve",
        notes: "Plan approved from work item workspace",
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
      await rejectWorkItemPlan(selectedWorkItemId, "Plan rejected from work item workspace");
      await handleWorkflowUserAction({
        workflowRunId,
        action: "reject",
        notes: "Plan rejected from work item workspace",
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
      await approveWorkItemTestReview(selectedWorkItemId, "Test review approved from work item workspace");
      await handleWorkflowUserAction({
        workflowRunId,
        action: "approve",
        notes: "Test review approved from work item workspace",
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
        notes: "Test review rejected from work item workspace",
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
        "Marked failed from Work Item review due to stale execution",
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
    () => selectedWorkItem ?? workItems?.find((workItem) => workItem.id === selectedWorkItemId) ?? null,
    [selectedWorkItem, workItems, selectedWorkItemId],
  );
  const orderedWorkItems = useMemo(() => orderWorkItemsByIds(workItems ?? [], workItemOrderIds), [workItems, workItemOrderIds]);
  const backlogWorkflowRunQueries = useQueries({
    queries: orderedWorkItems.map((workItem) => ({
      queryKey: ["latestWorkflowRun", workItem.id],
      queryFn: () => getLatestWorkflowRunForWorkItem(workItem.id),
      enabled: workItemWorkspaceTab === "backlog",
      refetchInterval: 4000,
    })),
  });
  const latestWorkflowRunByWorkItemId = useMemo(() => {
    const map = new Map<string, WorkflowRun | null>();
    orderedWorkItems.forEach((workItem, index) => {
      const run = backlogWorkflowRunQueries[index]?.data ?? null;
      map.set(workItem.id, run);
    });
    return map;
  }, [backlogWorkflowRunQueries, orderedWorkItems]);
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
      return { blockers: ["Select a work item to evaluate readiness."], warnings, checks };
    }

    if (selectedWorkItemSummary.status !== "approved") {
      blockers.push("Work item status must be Approved before starting workflow.");
    } else {
      checks.push("Work item is approved.");
    }

    const assignmentMatch = (teamAssignments ?? []).find((assignment: TeamAssignment) => {
      if (assignment.scope_type === "capability" && selectedWorkItemSummary.capability_id) {
        return assignment.scope_id === selectedWorkItemSummary.capability_id;
      }
      if (assignment.scope_type === "module" && selectedWorkItemSummary.module_id) {
        return assignment.scope_id === selectedWorkItemSummary.module_id;
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
      warnings.push("No team assignment found for capability/module/product scope. Fallback global agents will be used.");
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
  ]);

  return (
    <div style={styles.page}>
      <div style={styles.header}>
        <div style={styles.titleBlock}>
          <h1 style={styles.title}>Work Item Workspace</h1>
          <div style={styles.subtitle}>Use the queue for intake, detail for refinement, and review for evidence. The active hierarchy still controls scope.</div>
        </div>
      </div>

      <div style={styles.panel}>
        <div style={styles.panelInner}>
          <div style={styles.tabBar}>
            <button style={workItemWorkspaceTab === "backlog" ? styles.tabActive : styles.tab} onClick={() => setWorkItemWorkspaceTab("backlog")}>Backlog</button>
            <button style={workItemWorkspaceTab === "detail" ? styles.tabActive : styles.tab} onClick={() => setWorkItemWorkspaceTab("detail")}>Work Item Detail</button>
            <button style={workItemWorkspaceTab === "review" ? styles.tabActive : styles.tab} onClick={() => setWorkItemWorkspaceTab("review")}>Review</button>
          </div>

          {workItemWorkspaceTab === "backlog" && (
            <>
              <div style={styles.sectionTitle}>
                <span>Backlog</span>
                <button style={styles.ghostBtn} onClick={openWorkItemCreateDialog}>+ New Work Item</button>
              </div>
              {!activeProductId && <div style={styles.warning}>Select a product to load the backlog.</div>}
              <select style={styles.filterSelect} value={statusFilter} onChange={(e) => setStatusFilter(e.target.value)}>
                <option value="">All Statuses</option>
                {Object.keys(statusColors).map((status) => (
                  <option key={status} value={status}>{status.replace(/_/g, " ")}</option>
                ))}
              </select>
              <div style={styles.smallText}>
                Scope: {activeCapabilityId ? "Outcome" : activeModuleId ? "Module" : activeProductId ? "Product" : "None selected"}
              </div>
              {isLoading ? (
                <div style={styles.empty}>Loading work items...</div>
              ) : orderedWorkItems.length > 0 ? (
                <div style={styles.taskList}>
                  {orderedWorkItems.map((workItem, workItemIndex) => (
                    (() => {
                      const latestRun = latestWorkflowRunByWorkItemId.get(workItem.id) ?? null;
                      const runtimeStatus = describeWorkItemRuntime(workItem, latestRun);

                      return (
                        <div
                          key={workItem.id}
                          style={{
                            ...(selectedWorkItemId === workItem.id ? styles.taskCardActive : styles.taskCard),
                            ...(draggedWorkItemId === workItem.id ? styles.dropTarget : null),
                          }}
                          draggable
                          onDragStart={() => setDraggedWorkItemId(workItem.id)}
                          onDragEnd={() => setDraggedWorkItemId(null)}
                          onDragOver={(e) => e.preventDefault()}
                          onDrop={() => {
                            if (!draggedWorkItemId || draggedWorkItemId === workItem.id) {
                              return;
                            }
                            const nextOrder = moveTaskIdToIndex(workItemOrderIds, draggedWorkItemId, workItemIndex);
                            setWorkItemOrderIds(nextOrder);
                            reorderWorkItemsMutation.mutate(nextOrder);
                            setDraggedWorkItemId(null);
                          }}
                          onClick={() => {
                            setActiveWorkItem(workItem.id);
                            setWorkItemWorkspaceTab("detail");
                          }}
                        >
                          <div style={styles.taskRowCard}>
                            <div style={styles.taskMain}>
                              <div style={styles.taskTitle}>{workItem.title}</div>
                              <div style={styles.taskMeta}>
                                <span>{workItem.work_item_type}</span>
                                <span>{workItem.priority}</span>
                                {runtimeStatus.stageLabel ? <span>stage: {runtimeStatus.stageLabel}</span> : null}
                              </div>
                            </div>
                            <div style={styles.taskStatusLine}>
                              <div style={styles.badgeRow}>
                                <span style={{ ...styles.badge, ...getToneBadgeStyle(runtimeStatus.tone) }}>
                                  {runtimeStatus.label}
                                </span>
                                <span style={{ ...styles.badge, backgroundColor: statusColors[workItem.status] || "#444", color: "#fff" }}>
                                  {workItem.status.replace(/_/g, " ")}
                                </span>
                              </div>
                              <div style={styles.taskStatusSummary}>{runtimeStatus.detail}</div>
                            </div>
                            <div style={styles.taskActions}>
                              <span style={styles.dragHandle} title="Drag to reorder">::</span>
                              <div style={styles.overflowWrap}>
                                <button
                                  style={styles.ghostBtn}
                                  onClick={(e) => {
                                    e.stopPropagation();
                                    setOpenOverflowWorkItemId((current) => (current === workItem.id ? null : workItem.id));
                                  }}
                                  aria-label={`More actions for ${workItem.title}`}
                                >
                                  •••
                                </button>
                                {openOverflowWorkItemId === workItem.id && (
                                  <div style={styles.overflowMenu} onClick={(e) => e.stopPropagation()}>
                                    <button
                                      style={styles.overflowMenuItem}
                                      onClick={() => {
                                        setActiveWorkItem(workItem.id);
                                        setWorkItemWorkspaceTab("detail");
                                        setOpenOverflowWorkItemId(null);
                                      }}
                                    >
                                      Open details
                                    </button>
                                    <button
                                      style={styles.overflowMenuItemDanger}
                                      onClick={() => {
                                        setOpenOverflowWorkItemId(null);
                                        deleteMutation.mutate(workItem.id);
                                      }}
                                    >
                                      Delete work item
                                    </button>
                                  </div>
                                )}
                              </div>
                            </div>
                          </div>
                        </div>
                      );
                    })()
                  ))}
                </div>
              ) : (
                <div style={styles.empty}>No work items in the current scope yet.</div>
              )}
            </>
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
                  <button style={styles.ghostBtn} onClick={() => setIsEditingWorkItem(true)}>
                    Edit Work Item
                  </button>
                </div>
                {actionError && <div style={styles.errorText}>{actionError}</div>}
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

                <>
                  <div style={styles.detailCard}>
                    <div style={styles.detailLabel}>Description</div>
                    <div style={styles.detailValue}>{selectedWorkItemSummary.description || "No description yet."}</div>
                  </div>
                  <div style={styles.row}>
                    <div style={styles.detailCard}><div style={styles.detailLabel}>Work Item Status</div><div style={styles.detailValue}>{selectedWorkItemSummary.status.replace(/_/g, " ")}</div></div>
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
              <div style={styles.empty}>Select a work item from the queue to refine it.</div>
            )
          )}

          {workItemWorkspaceTab === "review" && (
            <>
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
                  <div style={styles.detailValue}>No workflow run yet. Start a workflow from the Work Item Detail tab.</div>
                )}
              </div>
              <div style={styles.detailCard}>
                <div style={styles.detailLabel}>Subtasks</div>
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
                  <div style={styles.detailValue}>No child work items yet.</div>
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
        <ModalShell
          title={`Artifact: ${getArtifactFileName(artifactModalArtifact)}`}
          onClose={() => setArtifactModalArtifact(null)}
        >
          <div style={styles.detailCard}>
            <div style={styles.detailLabel}>Type</div>
            <div style={styles.detailValue}>{artifactModalArtifact.artifact_type}</div>
            <div style={{ ...styles.detailLabel, marginTop: 10 }}>Path</div>
            <div style={styles.smallText}>{artifactModalArtifact.storage_path}</div>
            <div style={{ ...styles.detailLabel, marginTop: 10 }}>Summary</div>
            <div style={styles.smallText}>{artifactModalArtifact.summary}</div>
          </div>
          <div style={styles.previewBox}>
            {(artifactModalContent ?? "").trim() || "Artifact content is empty."}
          </div>
        </ModalShell>
      )}

      {(showCreateForm || workItemCreateDialogOpen) && (
        <ModalShell title="Create Work Item" onClose={() => { setShowCreateForm(false); closeWorkItemCreateDialog(); }}>
          <div style={styles.detailCard}>
            <div style={styles.detailLabel}>Creation Scope</div>
            <div style={styles.detailValue}>
              {activeCapabilityId ? "Current outcome" : activeModuleId ? "Current module" : activeProductId ? "Current product" : "No product selected"}
            </div>
          </div>
          <label style={styles.detailLabel}>Title</label>
          <input style={styles.input} value={createForm.title} onChange={(e) => setCreateForm({ ...createForm, title: e.target.value })} />
          <label style={styles.detailLabel}>Problem Statement</label>
          <textarea style={styles.textarea} value={createForm.problemStatement} onChange={(e) => setCreateForm({ ...createForm, problemStatement: e.target.value })} />
          <label style={styles.detailLabel}>Description</label>
          <textarea style={styles.textarea} value={createForm.description} onChange={(e) => setCreateForm({ ...createForm, description: e.target.value })} />
          <label style={styles.detailLabel}>Acceptance Criteria</label>
          <textarea style={styles.textarea} value={createForm.acceptanceCriteria} onChange={(e) => setCreateForm({ ...createForm, acceptanceCriteria: e.target.value })} />
          <label style={styles.detailLabel}>Constraints</label>
          <textarea style={styles.textarea} value={createForm.constraints} onChange={(e) => setCreateForm({ ...createForm, constraints: e.target.value })} />
          <div style={styles.row}>
            <div>
              <label style={styles.detailLabel}>Type</label>
              <select style={styles.filterSelect} value={createForm.workItemType} onChange={(e) => setCreateForm({ ...createForm, workItemType: e.target.value })}>
                {["feature", "bug", "refactor", "test", "review", "security_fix", "performance_improvement"].map((type) => (
                  <option key={type} value={type}>{type}</option>
                ))}
              </select>
            </div>
            <div>
              <label style={styles.detailLabel}>Priority</label>
              <select style={styles.filterSelect} value={createForm.priority} onChange={(e) => setCreateForm({ ...createForm, priority: e.target.value })}>
                {["critical", "high", "medium", "low"].map((priority) => (
                  <option key={priority} value={priority}>{priority}</option>
                ))}
              </select>
            </div>
          </div>
          {!activeProductId && <div style={styles.warning}>Select a product before creating a work item.</div>}
          {formError && <div style={styles.errorText}>{formError}</div>}
          <div style={{ display: "flex", justifyContent: "flex-end", gap: 8 }}>
            <button style={styles.ghostBtn} onClick={() => { setShowCreateForm(false); closeWorkItemCreateDialog(); }}>Cancel</button>
            <button style={styles.btn} onClick={() => createMutation.mutate()} disabled={!activeProductId || !createForm.title}>
              {createMutation.isPending ? "Creating..." : "Create Work Item"}
            </button>
          </div>
        </ModalShell>
      )}

      {isEditingWorkItem && selectedWorkItemSummary && (
        <ModalShell title="Edit Work Item" onClose={() => setIsEditingWorkItem(false)}>
          {formError && <div style={styles.errorText}>{formError}</div>}
          <label style={styles.detailLabel}>Title</label>
          <input style={styles.input} value={workItemDraft.title} onChange={(e) => setWorkItemDraft({ ...workItemDraft, title: e.target.value })} />
          <label style={styles.detailLabel}>Description</label>
          <textarea style={styles.textarea} value={workItemDraft.description} onChange={(e) => setWorkItemDraft({ ...workItemDraft, description: e.target.value })} />
          <label style={styles.detailLabel}>Problem Statement</label>
          <textarea style={styles.textarea} value={workItemDraft.problemStatement} onChange={(e) => setWorkItemDraft({ ...workItemDraft, problemStatement: e.target.value })} />
          <label style={styles.detailLabel}>Acceptance Criteria</label>
          <textarea style={styles.textarea} value={workItemDraft.acceptanceCriteria} onChange={(e) => setWorkItemDraft({ ...workItemDraft, acceptanceCriteria: e.target.value })} />
          <label style={styles.detailLabel}>Constraints</label>
          <textarea style={styles.textarea} value={workItemDraft.constraints} onChange={(e) => setWorkItemDraft({ ...workItemDraft, constraints: e.target.value })} />
          <label style={styles.detailLabel}>Status</label>
          <select style={styles.filterSelect} value={workItemDraft.status} onChange={(e) => setWorkItemDraft({ ...workItemDraft, status: e.target.value })}>
            {Object.keys(statusColors).map((status) => (
              <option key={status} value={status}>{status.replace(/_/g, " ")}</option>
            ))}
          </select>
          <div style={{ display: "flex", justifyContent: "flex-end", gap: 8 }}>
            <button style={styles.ghostBtn} onClick={() => setIsEditingWorkItem(false)}>Cancel</button>
            <button style={styles.btn} onClick={() => updateWorkItemMutation.mutate()} disabled={!workItemDraft.title}>
              {updateWorkItemMutation.isPending ? "Saving..." : "Save Work Item"}
            </button>
          </div>
        </ModalShell>
      )}
    </div>
  );
}

function ModalShell({ title, onClose, children }: { title: string; onClose: () => void; children: React.ReactNode }) {
  return (
    <div style={styles.modalBackdrop} onClick={onClose}>
      <div style={styles.modal} onClick={(e) => e.stopPropagation()}>
        <div style={styles.modalHeader}>
          <div style={styles.modalTitle}>{title}</div>
          <button style={styles.ghostBtn} onClick={onClose}>Close</button>
        </div>
        <div style={styles.modalBody}>{children}</div>
      </div>
    </div>
  );
}

function moveTaskIdToIndex(ids: string[], id: string, nextIndex: number): string[] {
  const currentIndex = ids.indexOf(id);
  if (currentIndex === -1 || nextIndex < 0 || nextIndex >= ids.length) {
    return ids;
  }
  const nextIds = [...ids];
  const [item] = nextIds.splice(currentIndex, 1);
  nextIds.splice(nextIndex, 0, item);
  return nextIds;
}

function orderWorkItemsByIds(workItems: WorkItem[], orderedIds: string[]) {
  if (orderedIds.length === 0) {
    return workItems;
  }
  const rank = new Map(orderedIds.map((id, index) => [id, index]));
  return [...workItems].sort((a, b) => (rank.get(a.id) ?? Number.MAX_SAFE_INTEGER) - (rank.get(b.id) ?? Number.MAX_SAFE_INTEGER));
}

type RuntimeTone = "neutral" | "info" | "success" | "warning" | "danger";

function describeWorkItemRuntime(workItem: WorkItem, workflowRun: WorkflowRun | null) {
  if (!workflowRun) {
    if (workItem.status === "approved") {
      return {
        label: "Ready",
        tone: "info" as RuntimeTone,
        detail: "Approved and ready to start",
        stageLabel: null,
      };
    }
    if (workItem.status === "done") {
      return {
        label: "Completed",
        tone: "success" as RuntimeTone,
        detail: "Work item marked done",
        stageLabel: null,
      };
    }
    if (workItem.status === "failed" || workItem.status === "blocked" || workItem.status === "cancelled") {
      return {
        label: workItem.status.replace(/_/g, " "),
        tone: "danger" as RuntimeTone,
        detail: `Work item ${workItem.status.replace(/_/g, " ")}`,
        stageLabel: null,
      };
    }
    return {
      label: "Not started",
      tone: "neutral" as RuntimeTone,
      detail: `Work item ${workItem.status.replace(/_/g, " ")}`,
      stageLabel: null,
    };
  }

  const stageLabel = workflowRun.current_stage.replace(/_/g, " ");
  if (workflowRun.status === "running") {
    return {
      label: workflowRun.current_stage.startsWith("pending_") ? "Awaiting review" : "Running",
      tone: workflowRun.current_stage.startsWith("pending_") ? "warning" as RuntimeTone : "info" as RuntimeTone,
      detail: `Workflow ${workflowRun.status} at ${stageLabel}`,
      stageLabel,
    };
  }
  if (workflowRun.status === "completed" || workflowRun.current_stage === "done") {
    return {
      label: "Completed",
      tone: "success" as RuntimeTone,
      detail: "Workflow completed successfully",
      stageLabel,
    };
  }
  if (workflowRun.status === "failed") {
    return {
      label: "Failed",
      tone: "danger" as RuntimeTone,
      detail: `Workflow failed at ${stageLabel}`,
      stageLabel,
    };
  }
  if (workflowRun.status === "cancelled") {
    return {
      label: "Cancelled",
      tone: "danger" as RuntimeTone,
      detail: `Workflow cancelled at ${stageLabel}`,
      stageLabel,
    };
  }
  if (workflowRun.status === "paused") {
    return {
      label: "Paused",
      tone: "warning" as RuntimeTone,
      detail: `Workflow paused at ${stageLabel}`,
      stageLabel,
    };
  }

  return {
    label: workflowRun.status,
    tone: "neutral" as RuntimeTone,
    detail: `Workflow ${workflowRun.status} at ${stageLabel}`,
    stageLabel,
  };
}

function getToneBadgeStyle(tone: RuntimeTone): React.CSSProperties {
  switch (tone) {
    case "info":
      return { backgroundColor: "#0e639c", color: "#fff" };
    case "success":
      return { backgroundColor: "#2d6a3f", color: "#fff" };
    case "warning":
      return { backgroundColor: "#7a5b16", color: "#fff" };
    case "danger":
      return { backgroundColor: "#8b2d2d", color: "#fff" };
    default:
      return { backgroundColor: "#444", color: "#fff" };
  }
}
