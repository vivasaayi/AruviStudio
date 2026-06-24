import { normalize } from "./plannerPageCore";

export type PlannerVoiceViewCommand = "draft" | "trace" | "conversation";

const VIEW_DRAFT_COMMANDS = new Set([
  "view draft",
  "open draft",
  "show draft",
  "show draft tree",
  "view draft tree",
  "view design",
  "open design",
  "show design",
  "show design tree",
  "view design tree",
  "open workspace",
  "show workspace",
]);

const VIEW_TRACE_COMMANDS = new Set(["view trace", "show trace", "open trace"]);

const VIEW_CONVERSATION_COMMANDS = new Set([
  "view conversation",
  "open conversation",
  "show conversation",
  "back to chat",
  "view chat",
]);

const EXPAND_DRAFT_COMMANDS = new Set([
  "expand draft",
  "expand the draft",
  "expand tree",
  "expand all",
  "open all branches",
]);

const COLLAPSE_DRAFT_COMMANDS = new Set([
  "collapse draft",
  "collapse the draft",
  "collapse tree",
  "collapse all",
]);

const DRAFT_WIDE_TARGETS = new Set(["draft", "tree", "all"]);

const NODE_REFERENCE_PREFIXES: Array<{ prefix: string; type: string }> = [
  { prefix: "work item ", type: "work item" },
  { prefix: "work-item ", type: "work item" },
  { prefix: "capability ", type: "capability" },
  { prefix: "product area ", type: "product area" },
  { prefix: "product ", type: "product" },
  { prefix: "node ", type: "node" },
];

export function getPlannerVoiceViewCommand(normalizedTranscript: string): PlannerVoiceViewCommand | null {
  if (VIEW_DRAFT_COMMANDS.has(normalizedTranscript)) {
    return "draft";
  }
  if (VIEW_TRACE_COMMANDS.has(normalizedTranscript)) {
    return "trace";
  }
  if (VIEW_CONVERSATION_COMMANDS.has(normalizedTranscript)) {
    return "conversation";
  }
  return null;
}

export function isExpandDraftVoiceCommand(normalizedTranscript: string) {
  return EXPAND_DRAFT_COMMANDS.has(normalizedTranscript);
}

export function isCollapseDraftVoiceCommand(normalizedTranscript: string) {
  return COLLAPSE_DRAFT_COMMANDS.has(normalizedTranscript);
}

export function isDraftWideVoiceTarget(targetText: string) {
  return DRAFT_WIDE_TARGETS.has(normalize(targetText));
}

export function parseVoiceNodeReference(
  spokenRemainder: string,
): { explicitType?: string; reference: string } {
  const trimmed = spokenRemainder.trim();
  for (const option of NODE_REFERENCE_PREFIXES) {
    if (trimmed === option.prefix.trim()) {
      return { explicitType: option.type, reference: `selected ${option.type}` };
    }
    if (trimmed.startsWith(option.prefix)) {
      return { explicitType: option.type, reference: trimmed.slice(option.prefix.length).trim() };
    }
  }
  return { reference: trimmed };
}
