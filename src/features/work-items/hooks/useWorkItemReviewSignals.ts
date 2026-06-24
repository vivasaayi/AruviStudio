import { useMemo } from "react";
import { useQuery } from "@tanstack/react-query";

import { listExternalCliRunEvents } from "../../../lib/tauri";
import type {
  AgentDefinition,
  AgentModelBinding,
  AgentRun,
  AgentTeam,
  AgentTeamMembership,
  Approval,
  Artifact,
  ExternalCliRun,
  Finding,
  ModelCall,
  ModelDefinition,
  ModelProvider,
  Repository,
  TeamAssignment,
  WorkItem,
  WorkflowRun,
  WorkflowStageHistory,
  WorkflowStagePolicy,
} from "../../../lib/types";
import {
  EXTERNAL_CLI_TRACE_LIMIT,
  WORKFLOW_DAG_LANES,
  WORKFLOW_DAG_NODES,
  buildWorkflowLaneStatusById,
  filterArtifactsForWorkflowStages,
  filterWorkflowHistoryForStages,
  findLatestAgentRunForStage,
  formatExternalCliTerminal,
  formatWorkflowElapsedLabel,
  getRunningAgentRunStartMs,
  groupArtifactsByAgentRunId,
  groupModelCallsByAgentRunId,
  isWorkflowRunStale,
  summarizeModelUsage,
} from "../lib/workItemListPageHelpers";
import { buildWorkItemWorkflowReadiness } from "../lib/workItemWorkflowReadiness";

type WorkItemReviewSignalsInput = {
  selectedArtifactStage: string | null;
  activeWorkflowStage: string | null;
  latestWorkflowRun: WorkflowRun | null | undefined;
  workflowRunId: string | null;
  workflowHistory: WorkflowStageHistory[] | undefined;
  agentRuns: AgentRun[] | undefined;
  agentModelCalls: ModelCall[] | undefined;
  artifacts: Artifact[] | undefined;
  approvals: Approval[] | undefined;
  findings: Finding[] | undefined;
  externalCliRuns: ExternalCliRun[] | undefined;
  selectedExternalCliRunId: string | null;
  selectedWorkItem: WorkItem | null | undefined;
  teamAssignments: TeamAssignment[] | undefined;
  agentTeams: AgentTeam[] | undefined;
  teamMemberships: AgentTeamMembership[] | undefined;
  agentDefinitions: AgentDefinition[] | undefined;
  workflowPolicies: WorkflowStagePolicy[] | undefined;
  modelBindings: AgentModelBinding[] | undefined;
  modelDefinitions: ModelDefinition[] | undefined;
  providers: ModelProvider[] | undefined;
  resolvedRepository: Repository | null | undefined;
};

export function useWorkItemReviewSignals({
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
  selectedWorkItem,
  teamAssignments,
  agentTeams,
  teamMemberships,
  agentDefinitions,
  workflowPolicies,
  modelBindings,
  modelDefinitions,
  providers,
  resolvedRepository,
}: WorkItemReviewSignalsInput) {
  const selectedDagNodeId = selectedArtifactStage
    ?? WORKFLOW_DAG_NODES.find((node) => node.actualStageIds.includes(activeWorkflowStage ?? ""))?.id
    ?? "draft";
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
    selectedWorkItem,
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
    selectedWorkItem,
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

  return {
    selectedDagNodeId,
    latestAgentRunForActiveStage,
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
  };
}
