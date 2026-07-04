import type { PlannerMessage, PlannerMutationResult } from "./plannerPageTypes";
import { isInformationalOnly } from "./plannerPageCore";

type IdFactory = () => string;

function joinResponseLines(lines: Array<string | undefined | null>) {
  return lines.filter(Boolean).join("\n");
}

function formatExecutionErrors(errors: string[]) {
  return errors.length ? `Errors: ${errors.join(" | ")}` : null;
}

export function buildPlannerMutationMessages(
  currentMessages: PlannerMessage[],
  result: PlannerMutationResult,
  makeId: IdFactory,
): PlannerMessage[] {
  const next: PlannerMessage[] = [
    ...currentMessages,
    { id: makeId(), role: "user", content: result.userInput, kind: "text" },
  ];

  if (result.mode === "confirmation_required") {
    next.push({
      id: makeId(),
      role: "assistant",
      content: result.plan.assistant_response,
      meta: "Suggestion awaiting confirmation",
      kind: "proposal",
      plan: result.plan,
      treeNodes: result.treeNodes,
      traceEvents: result.traceEvents,
    });
    return next;
  }

  if (result.mode === "draft_updated") {
    next.push({
      id: makeId(),
      role: "assistant",
      content: joinResponseLines([result.plan.assistant_response, ...(result.execution?.lines ?? [])]),
      meta: "Design updated",
      kind: "proposal",
      plan: result.plan,
      treeNodes: result.treeNodes,
      traceEvents: result.traceEvents,
    });
    return next;
  }

  if (result.mode === "confirmed") {
    next.push({
      id: makeId(),
      role: "assistant",
      content: joinResponseLines([
        "Executed pending plan.",
        ...(result.execution?.lines ?? []),
        formatExecutionErrors(result.execution?.errors ?? []),
      ]),
      meta: "Planner execution",
      kind: result.treeNodes ? "tree" : "execution",
      treeNodes: result.treeNodes,
      plan: result.plan,
      traceEvents: result.traceEvents,
    });
    return next;
  }

  if (result.mode === "clarification") {
    next.push({
      id: makeId(),
      role: "assistant",
      content: result.plan.clarification_question ?? result.plan.assistant_response,
      meta: "Need more detail",
      kind: "text",
      traceEvents: result.traceEvents,
    });
    return next;
  }

  if (result.mode === "session_updated") {
    next.push({
      id: makeId(),
      role: "assistant",
      content: joinResponseLines([
        result.plan.assistant_response,
        ...(result.execution?.lines ?? []),
        formatExecutionErrors(result.execution?.errors ?? []),
      ]),
      meta: "Planner state updated",
      kind: "text",
      traceEvents: result.traceEvents,
    });
    return next;
  }

  if (result.mode === "failed") {
    next.push({
      id: makeId(),
      role: "assistant",
      content: joinResponseLines([
        result.plan.assistant_response,
        formatExecutionErrors(result.execution.errors),
      ]),
      meta: "Planner error",
      kind: "error",
      traceEvents: result.traceEvents,
    });
    return next;
  }

  next.push({
    id: makeId(),
    role: "assistant",
    content: joinResponseLines([
      result.plan.assistant_response,
      ...(result.execution?.lines ?? []),
      formatExecutionErrors(result.execution?.errors ?? []),
    ]),
    meta: isInformationalOnly(result.plan) ? "Status report" : "Planner execution",
    kind: result.treeNodes ? "tree" : isInformationalOnly(result.plan) ? "report" : "execution",
    treeNodes: result.treeNodes,
    plan: result.plan,
    traceEvents: result.traceEvents,
  });
  return next;
}

export function getPlannerMutationSpeechText(result: PlannerMutationResult) {
  if (result.mode === "clarification") {
    return result.plan.clarification_question ?? result.plan.assistant_response;
  }
  if (result.mode === "confirmation_required") {
    return `${result.plan.assistant_response}. Say confirm to apply the proposal.`;
  }
  if (result.mode === "draft_updated") {
    return `${result.plan.assistant_response}. The design tree has been updated.`;
  }
  if (result.mode === "session_updated") {
    return result.plan.assistant_response;
  }
  if (result.mode === "confirmed") {
    return "Executed the pending planner actions.";
  }
  return result.plan.assistant_response;
}
