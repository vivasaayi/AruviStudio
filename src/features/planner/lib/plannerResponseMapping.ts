import type { PlannerTurnResponse } from "../../../lib/types";
import type {
  ExecutionResult,
  PlannerMutationResult,
  PlannerPlan,
  PlannerTreeNode,
} from "./plannerPageTypes";

export function mapPlannerResponseToMutationResult(
  response: PlannerTurnResponse,
  userInput: string,
): PlannerMutationResult {
  const backendPlan = (response.pending_plan as unknown as PlannerPlan) ?? {
    assistant_response: response.assistant_message,
    needs_confirmation: false,
    clarification_question: response.status === "clarification" ? response.assistant_message : null,
    actions: [],
  };
  const execution: ExecutionResult = {
    lines: response.execution_lines,
    errors: response.execution_errors,
  };
  const treeNodes = (response.tree_nodes as unknown as PlannerTreeNode[] | undefined) ?? undefined;
  const responseDraftTreeNodes = (response.draft_tree_nodes as unknown as PlannerTreeNode[] | undefined) ?? undefined;
  const responseSelectedDraftNodeId = response.selected_draft_node_id ?? null;
  const traceEvents = response.trace_events ?? [];

  if (response.status === "proposal" && responseDraftTreeNodes) {
    return {
      mode: "draft_updated",
      userInput,
      plan: backendPlan,
      execution,
      treeNodes,
      draftTreeNodes: responseDraftTreeNodes,
      selectedDraftNodeId: responseSelectedDraftNodeId,
      traceEvents,
    };
  }

  if (response.status === "proposal") {
    return {
      mode: "confirmation_required",
      userInput,
      plan: backendPlan,
      execution: null,
      treeNodes,
      draftTreeNodes: responseDraftTreeNodes,
      selectedDraftNodeId: responseSelectedDraftNodeId,
      traceEvents,
    };
  }

  if (response.status === "clarification") {
    return {
      mode: "clarification",
      userInput,
      plan: backendPlan,
      execution: null,
      treeNodes,
      draftTreeNodes: responseDraftTreeNodes,
      selectedDraftNodeId: responseSelectedDraftNodeId,
      traceEvents,
    };
  }

  if (response.status === "session_update") {
    return {
      mode: "session_updated",
      userInput,
      plan: backendPlan,
      execution,
      treeNodes,
      draftTreeNodes: responseDraftTreeNodes,
      selectedDraftNodeId: responseSelectedDraftNodeId,
      traceEvents,
    };
  }

  if (response.status === "error") {
    return {
      mode: "failed",
      userInput,
      plan: backendPlan,
      execution,
      treeNodes,
      draftTreeNodes: responseDraftTreeNodes,
      selectedDraftNodeId: responseSelectedDraftNodeId,
      traceEvents,
    };
  }

  return {
    mode: "executed",
    userInput,
    plan: backendPlan,
    execution,
    treeNodes,
    draftTreeNodes: responseDraftTreeNodes,
    selectedDraftNodeId: responseSelectedDraftNodeId,
    traceEvents,
  };
}
