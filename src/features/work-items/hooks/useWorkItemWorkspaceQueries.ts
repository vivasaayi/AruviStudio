import { useQuery } from "@tanstack/react-query";

import {
  getLatestWorkflowRunForWorkItem,
  getSubWorkItems,
  getWorkflowHistory,
  getWorkItemApprovals,
  listAgentDefinitions,
  listAgentModelBindings,
  listAgentModelCallsForWorkflow,
  listAgentRunsForWorkflow,
  listAgentTeams,
  listExternalCliRunsForWorkItem,
  listModelDefinitions,
  listProviders,
  listTeamAssignments,
  listTeamMemberships,
  listWorkflowStagePolicies,
  listWorkItemArtifacts,
  listWorkItemFindings,
  readArtifactContent,
  resolveRepositoryForWorkItem,
} from "../../../lib/tauri";
import type { Artifact } from "../../../lib/types";
import { SUB_WORK_ITEM_PAGE_SIZE } from "../lib/workItemListPageHelpers";

type WorkItemWorkspaceQueriesInput = {
  selectedWorkItemId: string | null;
  activeWorkflowRunId: string | null;
  artifactModalArtifact: Artifact | null;
};

export function useWorkItemWorkspaceQueries({
  selectedWorkItemId,
  activeWorkflowRunId,
  artifactModalArtifact,
}: WorkItemWorkspaceQueriesInput) {
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

  return {
    latestWorkflowRun,
    workflowRunId,
    subWorkItems,
    approvals,
    artifacts,
    resolvedRepositoryFromQuery,
    artifactModalContent,
    findings,
    teamAssignments,
    agentTeams,
    teamMemberships,
    agentDefinitions,
    workflowPolicies,
    modelBindings,
    modelDefinitions,
    providers,
    workflowHistory,
    activeWorkflowStage,
    agentRuns,
    agentModelCalls,
    externalCliRuns,
  };
}
