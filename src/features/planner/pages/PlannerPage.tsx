import React, { useEffect, useMemo, useRef, useState } from "react";
import { useMutation, useQueries, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  addPlannerDraftChild,
  approveWorkItem,
  approveWorkItemPlan,
  approveWorkItemTestReview,
  archiveProduct,
  clearPlannerPending,
  confirmPlannerPlan,
  createCapability,
  createModule,
  createPlannerSession,
  createProduct,
  createWorkItem,
  deletePlannerDraftNode,
  deleteCapability,
  deleteModule,
  deleteWorkItem,
  getSetting,
  getLatestWorkflowRunForWorkItem,
  getProductTree,
  handleWorkflowUserAction,
  listRepositories,
  listModelDefinitions,
  listProducts,
  listProviders,
  listWorkItems,
  browseForRepositoryPath,
  rejectWorkItem,
  rejectWorkItemPlan,
  registerRepository,
  routePlannerContact,
  analyzeRepositoryForPlanner,
  renamePlannerDraftNode,
  runModelChatCompletion,
  speakTextNatively,
  sendTwilioWhatsappMessage,
  startWorkItemWorkflow,
  startTwilioVoiceCall,
  submitPlannerTurn,
  submitPlannerVoiceTurn,
  transcribeAudio,
  updatePlannerSession,
  updateCapability,
  updateModule,
  updateProduct,
  updateWorkItem,
} from "../../../lib/tauri";
import { blobToBase64, speakInBrowser, startWavCapture, type ActiveAudioCapture } from "../../shared/voice";
import { useWorkspaceStore } from "../../../state/workspaceStore";
import type {
  CapabilityTree,
  ModelDefinition,
  PlannerDraftChildType,
  PlannerTraceEvent,
  PlannerTurnResponse,
  Product,
  ProductTree,
  WorkItem,
} from "../../../lib/types";

const styles: Record<string, React.CSSProperties> = {
  page: { display: "flex", flexDirection: "column", gap: 8, height: "100%" },
  topGrid: { display: "grid", gridTemplateColumns: "minmax(0, 1fr) 320px", gap: 12, minHeight: 0, flex: 1 },
  compactStack: { display: "flex", flexDirection: "column", gap: 12, minHeight: 0, flex: 1 },
  panel: { backgroundColor: "#212327", border: "1px solid #32353d", borderRadius: 14, minHeight: 0, overflow: "hidden" },
  panelBody: { padding: 16, height: "100%", overflow: "auto" },
  compactPanelBody: { padding: 14, height: "100%", overflow: "auto" },
  sectionTitle: { fontSize: 11, fontWeight: 800, letterSpacing: 0.8, textTransform: "uppercase" as const, color: "#8f96a3", marginBottom: 10 },
  sectionHeader: { display: "flex", justifyContent: "space-between", alignItems: "center", gap: 10, marginBottom: 10 },
  transcript: { display: "flex", flexDirection: "column", gap: 10 },
  bubbleUser: { alignSelf: "flex-end", maxWidth: "80%", backgroundColor: "#0e639c", color: "#fff", borderRadius: 14, padding: "12px 14px", whiteSpace: "pre-wrap" as const },
  bubbleAssistant: { alignSelf: "flex-start", maxWidth: "84%", backgroundColor: "#2c3139", color: "#edf1f8", borderRadius: 14, padding: "12px 14px", whiteSpace: "pre-wrap" as const },
  bubblePending: { alignSelf: "flex-end", maxWidth: "80%", backgroundColor: "#17405f", color: "#e9f4ff", borderRadius: 14, padding: "12px 14px", whiteSpace: "pre-wrap" as const, border: "1px dashed #5aa9e6", opacity: 0.95 },
  bubbleMeta: { display: "block", marginTop: 8, fontSize: 11, color: "#a3adbb" },
  composerWrap: { display: "flex", flexDirection: "column", gap: 10, borderTop: "1px solid #32353d", paddingTop: 12, marginTop: 12 },
  textarea: { width: "100%", minHeight: 92, resize: "vertical" as const, padding: "12px 14px", borderRadius: 12, backgroundColor: "#181a1f", border: "1px solid #3c4048", color: "#edf1f8", fontSize: 14, boxSizing: "border-box" as const },
  actionRow: { display: "flex", flexWrap: "wrap" as const, gap: 10, alignItems: "center" },
  btn: { padding: "9px 14px", fontSize: 13, backgroundColor: "#0e639c", color: "#fff", border: "none", borderRadius: 10, cursor: "pointer", fontWeight: 700 },
  btnGhost: { padding: "9px 14px", fontSize: 13, backgroundColor: "#2c3139", color: "#e3e8f0", border: "1px solid #3c4048", borderRadius: 10, cursor: "pointer", fontWeight: 700 },
  btnDanger: { padding: "9px 14px", fontSize: 13, backgroundColor: "#6c2020", color: "#fff", border: "none", borderRadius: 10, cursor: "pointer", fontWeight: 700 },
  status: { fontSize: 12, color: "#9da7b5" },
  sideCard: { border: "1px solid #32353d", borderRadius: 12, backgroundColor: "#1b1d22", padding: 12, marginBottom: 12 },
  label: { fontSize: 12, color: "#9da7b5", display: "block", marginBottom: 6 },
  input: { width: "100%", padding: "9px 10px", backgroundColor: "#17191d", border: "1px solid #3c4048", borderRadius: 10, color: "#edf1f8", fontSize: 13, boxSizing: "border-box" as const },
  select: { width: "100%", padding: "9px 10px", backgroundColor: "#17191d", border: "1px solid #3c4048", borderRadius: 10, color: "#edf1f8", fontSize: 13, boxSizing: "border-box" as const },
  helper: { fontSize: 12, color: "#8f96a3", lineHeight: 1.45 },
  chipRow: { display: "flex", flexWrap: "wrap" as const, gap: 8, marginTop: 10 },
  chip: { padding: "5px 8px", borderRadius: 999, backgroundColor: "#223247", color: "#d7e8fb", fontSize: 11, fontWeight: 700 },
  warning: { color: "#ffb86c", fontSize: 12, lineHeight: 1.45 },
  error: { color: "#ff7b72", fontSize: 12, lineHeight: 1.45 },
  success: { color: "#59d6b2", fontSize: 12, lineHeight: 1.45 },
  list: { display: "flex", flexDirection: "column", gap: 8, marginTop: 10 },
  listItem: { borderRadius: 10, border: "1px solid #303742", backgroundColor: "#151922", padding: 10 },
  listItemTitle: { fontSize: 12, fontWeight: 700, color: "#edf1f8", marginBottom: 4 },
  listItemMeta: { fontSize: 12, color: "#9da7b5", whiteSpace: "pre-wrap" as const },
  card: { border: "1px solid #3a4250", backgroundColor: "#1b2029", borderRadius: 12, padding: 12, marginTop: 10 },
  voiceReviewCard: { border: "1px solid #375172", backgroundColor: "#142536", borderRadius: 12, padding: 12, marginBottom: 10 },
  voiceReviewHeader: { display: "flex", justifyContent: "space-between", alignItems: "center", gap: 10, marginBottom: 10, flexWrap: "wrap" as const },
  voiceReviewTitle: { fontSize: 13, fontWeight: 800, color: "#eef6ff" },
  cardTitle: { fontSize: 13, fontWeight: 800, color: "#eef3fb", marginBottom: 8 },
  cardSection: { marginTop: 10 },
  diffRow: { display: "grid", gridTemplateColumns: "24px minmax(0, 1fr)", gap: 8, alignItems: "start", padding: "8px 0", borderTop: "1px solid #2c3440" },
  diffSymbolAdd: { color: "#59d6b2", fontWeight: 800, fontSize: 16, lineHeight: 1.2 },
  diffSymbolUpdate: { color: "#7db7ff", fontWeight: 800, fontSize: 16, lineHeight: 1.2 },
  diffSymbolWarn: { color: "#ffb86c", fontWeight: 800, fontSize: 16, lineHeight: 1.2 },
  diffPrimary: { fontSize: 13, color: "#edf1f8", fontWeight: 700 },
  diffSecondary: { fontSize: 12, color: "#9da7b5", marginTop: 4, whiteSpace: "pre-wrap" as const },
  treePanel: { border: "1px solid #3a4250", backgroundColor: "#161b22", borderRadius: 12, padding: 12, marginTop: 10 },
  treeNode: { marginLeft: 0 },
  treeSummary: { cursor: "pointer", listStyle: "none", fontSize: 13, color: "#edf1f8", fontWeight: 700, padding: "4px 0" },
  treeChildren: { marginLeft: 18, borderLeft: "1px solid #2e3744", paddingLeft: 10 },
  treeLeaf: { fontSize: 13, color: "#d6dce7", padding: "4px 0" },
  treeMeta: { fontSize: 11, color: "#8f96a3", marginLeft: 6 },
  treeButton: { width: "100%", textAlign: "left" as const, background: "transparent", border: "1px solid transparent", borderRadius: 8, color: "#edf1f8", padding: "6px 8px", cursor: "pointer", fontSize: 13, fontWeight: 700 },
  treeButtonSelected: { border: "1px solid #0e639c", backgroundColor: "#14314a" },
  treeExplorer: { display: "flex", flexDirection: "column", gap: 4 },
  treeToolbar: { display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" as const, marginBottom: 12 },
  treeToolbarSpacer: { flex: 1 },
  treeLevel: { display: "flex", flexDirection: "column", gap: 6 },
  treeRow: { display: "grid", gridTemplateColumns: "26px minmax(0, 1fr)", gap: 8, alignItems: "start" },
  treeToggle: { width: 26, height: 26, borderRadius: 8, border: "1px solid #344050", backgroundColor: "#1a2130", color: "#dfe8f6", cursor: "pointer", fontSize: 12, fontWeight: 800, padding: 0 },
  treeToggleGhost: { width: 26, height: 26, borderRadius: 8, border: "1px solid transparent", backgroundColor: "transparent", color: "#5f6b7e", padding: 0 },
  treeCard: { width: "100%", textAlign: "left" as const, border: "1px solid #324054", backgroundColor: "#111821", color: "#edf1f8", borderRadius: 12, padding: "10px 12px", cursor: "pointer" },
  treeCardSelected: { border: "1px solid #0e639c", backgroundColor: "#173450", boxShadow: "inset 0 0 0 1px rgba(14,99,156,0.18)" },
  treeCardHeader: { display: "flex", alignItems: "center", justifyContent: "space-between", gap: 10, flexWrap: "wrap" as const },
  treeCardTitle: { fontSize: 14, fontWeight: 800, color: "#eef3fb" },
  treeCardMetaRow: { display: "flex", alignItems: "center", gap: 8, flexWrap: "wrap" as const, marginTop: 6 },
  treeTypeBadge: { fontSize: 11, fontWeight: 800, textTransform: "uppercase" as const, letterSpacing: 0.4, color: "#c6d8ee", backgroundColor: "#24364d", borderRadius: 999, padding: "3px 8px" },
  treeCountBadge: { fontSize: 11, fontWeight: 700, color: "#91a0b5", backgroundColor: "#1e2632", borderRadius: 999, padding: "3px 8px" },
  treeRowChildren: { marginLeft: 18, paddingLeft: 16, borderLeft: "1px solid #263243", display: "flex", flexDirection: "column", gap: 6 },
  inlineButtonRow: { display: "flex", gap: 8, marginTop: 12, flexWrap: "wrap" as const },
  viewToggleRow: { display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" as const },
  draftWorkspace: { display: "grid", gridTemplateColumns: "minmax(0, 1.6fr) minmax(260px, 0.9fr)", gap: 12, flex: 1, minHeight: 0 },
  draftWorkspaceMain: { display: "flex", flexDirection: "column", minHeight: 0 },
  draftWorkspaceSide: { display: "flex", flexDirection: "column", gap: 12, minHeight: 0 },
  draftCanvas: { border: "1px solid #3a4250", backgroundColor: "#161b22", borderRadius: 12, padding: 14, flex: 1, minHeight: 0, overflow: "auto" },
  draftCanvasHeader: { display: "flex", justifyContent: "space-between", alignItems: "center", gap: 10, marginBottom: 10, flexWrap: "wrap" as const },
  draftCanvasTitle: { fontSize: 14, fontWeight: 800, color: "#eef3fb" },
  emptyState: { border: "1px dashed #3a4250", borderRadius: 12, padding: 18, color: "#9da7b5", backgroundColor: "#171a20", lineHeight: 1.5 },
  metricGrid: { display: "grid", gridTemplateColumns: "repeat(4, minmax(0, 1fr))", gap: 8, marginTop: 10 },
  metricCard: { borderRadius: 12, border: "1px solid #324054", backgroundColor: "#141c27", padding: 10 },
  metricLabel: { fontSize: 11, color: "#8f96a3", textTransform: "uppercase" as const, letterSpacing: 0.5 },
  metricValue: { marginTop: 6, fontSize: 20, fontWeight: 800, color: "#eef3fb" },
  readinessBanner: { display: "flex", alignItems: "center", justifyContent: "space-between", gap: 12, padding: "10px 12px", borderRadius: 12, border: "1px solid #324054", backgroundColor: "#151d28", marginBottom: 12, flexWrap: "wrap" as const },
  readinessScore: { fontSize: 24, fontWeight: 800, color: "#eef3fb" },
  readinessMeta: { fontSize: 12, color: "#9da7b5" },
  issueList: { display: "flex", flexDirection: "column", gap: 8, marginTop: 10 },
  issueCard: { borderRadius: 10, border: "1px solid #374252", backgroundColor: "#121923", padding: 10 },
  issueCardWarn: { borderRadius: 10, border: "1px solid #6b5632", backgroundColor: "#221b12", padding: 10 },
  issueCardOk: { borderRadius: 10, border: "1px solid #2f5a4e", backgroundColor: "#12201c", padding: 10 },
  issueTitle: { fontSize: 12, fontWeight: 800, color: "#eef3fb", marginBottom: 4 },
  issueDetail: { fontSize: 12, color: "#9da7b5", lineHeight: 1.45 },
  promptList: { display: "flex", flexDirection: "column", gap: 8, marginTop: 10 },
  promptButton: { width: "100%", textAlign: "left" as const, padding: "9px 10px", borderRadius: 10, border: "1px solid #344050", backgroundColor: "#151c26", color: "#e6edf8", cursor: "pointer", fontSize: 12, lineHeight: 1.45 },
  pathText: { fontSize: 12, color: "#aeb8c6", lineHeight: 1.4, marginTop: 8 },
  sectionDivider: { height: 1, backgroundColor: "#313844", margin: "12px 0" },
  fieldGroup: { display: "flex", flexDirection: "column", gap: 8, marginTop: 10 },
  compactTextarea: { width: "100%", minHeight: 70, resize: "vertical" as const, padding: "9px 10px", borderRadius: 10, backgroundColor: "#17191d", border: "1px solid #3c4048", color: "#edf1f8", fontSize: 13, boxSizing: "border-box" as const },
  mutedButton: { padding: "9px 14px", fontSize: 13, backgroundColor: "#1f2530", color: "#dce6f4", border: "1px solid #344050", borderRadius: 10, cursor: "pointer", fontWeight: 700 },
  statusBanner: { border: "1px solid #334154", backgroundColor: "#18202b", borderRadius: 12, padding: "10px 12px", marginBottom: 12, display: "flex", justifyContent: "space-between", alignItems: "flex-start", gap: 12, flexWrap: "wrap" as const },
  statusBannerStrong: { fontSize: 13, fontWeight: 800, color: "#edf1f8" },
  statusBannerMeta: { fontSize: 12, color: "#9da7b5", lineHeight: 1.45, marginTop: 4 },
  compactControlStrip: { display: "flex", gap: 8, flexWrap: "wrap" as const, alignItems: "center", marginBottom: 12 },
  compactSummaryCard: { border: "1px solid #32353d", backgroundColor: "#1b1d22", borderRadius: 12, padding: 12 },
  compactSummaryGrid: { display: "grid", gridTemplateColumns: "repeat(4, minmax(0, 1fr))", gap: 8 },
  compactSummaryItem: { borderRadius: 10, backgroundColor: "#161920", border: "1px solid #313844", padding: "10px 12px" },
  compactSummaryLabel: { fontSize: 11, color: "#8f96a3", textTransform: "uppercase" as const, letterSpacing: 0.5 },
  compactSummaryValue: { fontSize: 13, fontWeight: 800, color: "#edf1f8", marginTop: 6, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" as const },
};

type PlannerMessageKind = "text" | "proposal" | "tree" | "report" | "execution" | "error";

type PlannerTreeNode = {
  id: string;
  label: string;
  meta?: string | null;
  node_type?: string | null;
  summary?: string | null;
  source?: string | null;
  confidence?: string | null;
  evidence?: string[];
  children: PlannerTreeNode[];
};

type PlannerMessage = {
  id: string;
  role: "user" | "assistant";
  content: string;
  meta?: string;
  kind?: PlannerMessageKind;
  plan?: PlannerPlan;
  treeNodes?: PlannerTreeNode[];
  traceEvents?: PlannerTraceEvent[];
};

type PlannerAction =
  | {
      type: "create_product";
      name: string;
      description?: string;
      vision?: string;
      goals?: string[];
      tags?: string[];
    }
  | {
      type: "update_product";
      target?: { productName?: string };
      fields: { name?: string; description?: string; vision?: string; goals?: string[]; tags?: string[] };
    }
  | { type: "archive_product"; target?: { productName?: string } }
  | {
      type: "create_module";
      target?: { productName?: string };
      name: string;
      description?: string;
      purpose?: string;
    }
  | {
      type: "update_module";
      target?: { productName?: string; moduleName?: string };
      fields: { name?: string; description?: string; purpose?: string };
    }
  | { type: "delete_module"; target?: { productName?: string; moduleName?: string } }
  | {
      type: "create_capability";
      target?: { productName?: string; moduleName?: string; capabilityName?: string };
      name: string;
      description?: string;
      acceptanceCriteria?: string;
      technicalNotes?: string;
      priority?: "critical" | "high" | "medium" | "low";
      risk?: "high" | "medium" | "low";
    }
  | {
      type: "update_capability";
      target?: { productName?: string; moduleName?: string; capabilityName?: string };
      fields: { name?: string; description?: string; acceptanceCriteria?: string; technicalNotes?: string; priority?: "critical" | "high" | "medium" | "low"; risk?: "high" | "medium" | "low" };
    }
  | { type: "delete_capability"; target?: { productName?: string; moduleName?: string; capabilityName?: string } }
  | {
      type: "create_work_item";
      target?: { productName?: string; moduleName?: string; capabilityName?: string };
      title: string;
      description?: string;
      problemStatement?: string;
      acceptanceCriteria?: string;
      constraints?: string;
      workItemType?: WorkItem["work_item_type"];
      priority?: WorkItem["priority"];
      complexity?: WorkItem["complexity"];
    }
  | {
      type: "update_work_item";
      target?: { productName?: string; workItemTitle?: string };
      fields: { title?: string; description?: string; problemStatement?: string; acceptanceCriteria?: string; constraints?: string; status?: WorkItem["status"] };
    }
  | { type: "delete_work_item"; target?: { productName?: string; workItemTitle?: string } }
  | { type: "approve_work_item"; target?: { productName?: string; workItemTitle?: string }; notes?: string }
  | { type: "reject_work_item"; target?: { productName?: string; workItemTitle?: string }; notes?: string }
  | { type: "approve_work_item_plan"; target?: { productName?: string; workItemTitle?: string }; notes?: string }
  | { type: "reject_work_item_plan"; target?: { productName?: string; workItemTitle?: string }; notes?: string }
  | { type: "approve_work_item_test_review"; target?: { productName?: string; workItemTitle?: string }; notes?: string }
  | { type: "start_workflow"; target?: { productName?: string; workItemTitle?: string } }
  | { type: "workflow_action"; target?: { productName?: string; workItemTitle?: string }; action: "approve" | "reject" | "pause" | "resume" | "cancel"; notes?: string }
  | { type: "report_status"; target?: { productName?: string; workItemTitle?: string } }
  | { type: "report_tree"; target?: { productName?: string } };

type PlannerPlan = {
  assistant_response: string;
  needs_confirmation: boolean;
  clarification_question: string | null;
  actions: PlannerAction[];
};

type PendingPlan = {
  sourceText: string;
  plan: PlannerPlan;
};

type ResolverContext = {
  products: Product[];
  productTrees: ProductTree[];
  workItems: WorkItem[];
  activeProductId: string | null;
  activeModuleId: string | null;
  activeCapabilityId: string | null;
  activeWorkItemId: string | null;
};

type ExecutionResult = {
  lines: string[];
  errors: string[];
};

type PlannerToolCall = {
  type: "tool_call";
  tool: "list_products" | "get_product_tree" | "list_work_items";
  arguments?: Record<string, unknown>;
  reason?: string;
};

type PlannerFinalResponse = {
  type: "final";
  assistant_response: string;
  needs_confirmation: boolean;
  clarification_question: string | null;
  actions: PlannerAction[];
};

type PlannerMutationResult =
  | {
      mode: "confirmed";
      userInput: string;
      plan: PlannerPlan;
      execution: ExecutionResult;
      treeNodes?: PlannerTreeNode[];
      draftTreeNodes?: PlannerTreeNode[];
      selectedDraftNodeId?: string | null;
      traceEvents?: PlannerTraceEvent[];
    }
  | {
      mode: "confirmation_required";
      userInput: string;
      plan: PlannerPlan;
      execution: null;
      treeNodes?: PlannerTreeNode[];
      draftTreeNodes?: PlannerTreeNode[];
      selectedDraftNodeId?: string | null;
      traceEvents?: PlannerTraceEvent[];
    }
  | {
      mode: "draft_updated";
      userInput: string;
      plan: PlannerPlan;
      execution: ExecutionResult;
      treeNodes?: PlannerTreeNode[];
      draftTreeNodes?: PlannerTreeNode[];
      selectedDraftNodeId?: string | null;
      traceEvents?: PlannerTraceEvent[];
    }
  | {
      mode: "clarification";
      userInput: string;
      plan: PlannerPlan;
      execution: null;
      treeNodes?: PlannerTreeNode[];
      draftTreeNodes?: PlannerTreeNode[];
      selectedDraftNodeId?: string | null;
      traceEvents?: PlannerTraceEvent[];
    }
  | {
      mode: "executed";
      userInput: string;
      plan: PlannerPlan;
      execution: ExecutionResult;
      treeNodes?: PlannerTreeNode[];
      draftTreeNodes?: PlannerTreeNode[];
      selectedDraftNodeId?: string | null;
      traceEvents?: PlannerTraceEvent[];
    }
  | {
      mode: "session_updated";
      userInput: string;
      plan: PlannerPlan;
      execution: ExecutionResult;
      treeNodes?: PlannerTreeNode[];
      draftTreeNodes?: PlannerTreeNode[];
      selectedDraftNodeId?: string | null;
      traceEvents?: PlannerTraceEvent[];
    }
  | {
      mode: "failed";
      userInput: string;
      plan: PlannerPlan;
      execution: ExecutionResult;
      treeNodes?: PlannerTreeNode[];
      draftTreeNodes?: PlannerTreeNode[];
      selectedDraftNodeId?: string | null;
      traceEvents?: PlannerTraceEvent[];
    };

type DraftEditOperation =
  | { kind: "rename"; nodeId: string; name: string }
  | { kind: "add_child"; parentNodeId: string; childType: PlannerDraftChildType; name: string; summary?: string }
  | { kind: "delete"; nodeId: string };

const DEFAULT_ASSISTANT_OPENING =
  "Talk to me like a planning lead. Describe the product or outcome you want, and I’ll check what already exists, suggest any missing products, capabilities, or work items, and wait for your confirmation before adding them.";

const SPEECH_PROVIDER_KEY = "speech.transcription_provider_id";
const SPEECH_MODEL_KEY = "speech.transcription_model_name";
const SPEECH_LOCALE_KEY = "speech.locale";
const SPEECH_NATIVE_VOICE_KEY = "speech.native_voice";
const SPEECH_ENABLE_MIC_KEY = "speech.enable_mic";
const SPEECH_AUTO_SPEAK_REPLIES_KEY = "speech.auto_speak_replies";

const PLANNER_SYSTEM_PROMPT = `You are an AI planning lead for a product-management desktop app.
You can inspect the workspace with tools before proposing changes.
Return exactly one JSON object each turn.

If you need more context, return:
{
  "type": "tool_call",
  "tool": "list_products|get_product_tree|list_work_items",
  "arguments": {},
  "reason": "brief reason"
}

When you are done, return:
{
  "type": "final",
  "assistant_response": "brief natural-language reply",
  "needs_confirmation": true,
  "clarification_question": null,
  "actions": []
}

Rules:
- Output valid JSON only. No markdown.
- Behave conversationally. First reason about what already exists in the supplied context, then suggest what should be added, changed, approved, or tracked.
- If the user is exploring or describing a need, prefer proposing actions rather than assuming immediate execution.
- If an entity already seems to exist, do not suggest creating a duplicate unless the user explicitly asks for a separate one.
- For any mutating action, assume confirmation is required before execution. Set needs_confirmation=true.
- Only set needs_confirmation=false for purely informational replies such as status reporting with no mutations.
- If the request is ambiguous, set actions=[] and put the missing detail in clarification_question.
- Use tools when the request depends on current repo state or structure instead of guessing from the prompt alone.
- Do not call mutation tools. You are only planning. Proposed mutations go in final.actions.
- After receiving tool results, continue reasoning and either call another tool or return type=final.
- Use these action types only:
create_product, update_product, archive_product,
create_module, update_module, delete_module,
create_capability, update_capability, delete_capability,
create_work_item, update_work_item, delete_work_item,
approve_work_item, reject_work_item, approve_work_item_plan, reject_work_item_plan, approve_work_item_test_review,
start_workflow, workflow_action, report_status, report_tree.
- Use product/module/capability/work item names in target fields, never IDs.
- For create_work_item defaults when omitted: workItemType=feature, priority=medium, complexity=medium.
- For create_capability defaults when omitted: priority=medium, risk=medium.
- For workflow_action action must be one of approve,reject,pause,resume,cancel.
- assistant_response should sound like a planning lead: mention what already exists, what is missing, and what you recommend doing next.
- When you propose actions, phrase assistant_response as a suggestion awaiting confirmation.`;

function normalize(value?: string | null) {
  return (value ?? "").trim().toLowerCase();
}

function makeId() {
  return crypto.randomUUID();
}

function summarizeContext(context: ResolverContext) {
  const productNameById = new Map(context.products.map((product) => [product.id, product.name]));
  const moduleNameById = new Map<string, string>();
  const capabilityNameById = new Map<string, string>();
  const productLines = context.productTrees.map((tree) => {
    const modules = tree.modules.map((moduleTree) => {
      moduleNameById.set(moduleTree.module.id, moduleTree.module.name);
      const capabilities: string[] = [];
      const visit = (node: CapabilityTree) => {
        capabilityNameById.set(node.capability.id, node.capability.name);
        capabilities.push(node.capability.name);
        node.children.forEach(visit);
      };
      moduleTree.features.forEach(visit);
      return `${moduleTree.module.name}${capabilities.length ? ` [${capabilities.join(", ")}]` : ""}`;
    });
    return `${tree.product.name}: ${modules.join(" | ") || "no modules"}`;
  });
  const workItemLines = context.workItems.slice(0, 120).map((item) => {
    const parts = [
      `${item.title} [${item.status}]`,
      productNameById.get(item.product_id) ? `product=${productNameById.get(item.product_id)}` : null,
      item.module_id ? `module=${moduleNameById.get(item.module_id) ?? item.module_id}` : null,
      item.capability_id ? `capability=${capabilityNameById.get(item.capability_id) ?? item.capability_id}` : null,
    ].filter(Boolean);
    return parts.join(" | ");
  });
  return [
    "Products and structure:",
    ...productLines,
    "Work items:",
    ...workItemLines,
  ].join("\n");
}

function extractJsonObject(raw: string) {
  const withoutFences = raw
    .trim()
    .replace(/^```(?:json)?\s*/i, "")
    .replace(/\s*```$/i, "");
  const withoutComments = withoutFences
    .replace(/^\s*\/\/.*$/gm, "")
    .replace(/^\s*#.*$/gm, "")
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .trim();

  if (withoutComments.startsWith("{") && withoutComments.endsWith("}")) {
    return withoutComments;
  }

  let depth = 0;
  let start = -1;
  let inString = false;
  let escaped = false;
  for (let index = 0; index < withoutComments.length; index += 1) {
    const char = withoutComments[index];
    if (inString) {
      if (escaped) {
        escaped = false;
      } else if (char === "\\") {
        escaped = true;
      } else if (char === "\"") {
        inString = false;
      }
      continue;
    }
    if (char === "\"") {
      inString = true;
      continue;
    }
    if (char === "{") {
      if (depth === 0) {
        start = index;
      }
      depth += 1;
      continue;
    }
    if (char === "}") {
      depth -= 1;
      if (depth === 0 && start >= 0) {
        return withoutComments.slice(start, index + 1);
      }
    }
  }

  const looseStart = withoutComments.indexOf("{");
  const looseEnd = withoutComments.lastIndexOf("}");
  if (looseStart >= 0 && looseEnd > looseStart) {
    return withoutComments.slice(looseStart, looseEnd + 1);
  }
  throw new Error("Planner model did not return JSON.");
}

function parsePlannerResponse(raw: string): PlannerPlan {
  let parsed: Partial<PlannerPlan>;
  try {
    parsed = JSON.parse(extractJsonObject(raw)) as Partial<PlannerPlan>;
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(`Planner response was not usable JSON. ${message}`);
  }
  return {
    assistant_response: typeof parsed.assistant_response === "string" ? parsed.assistant_response : "I translated that into planner actions.",
    needs_confirmation: Boolean(parsed.needs_confirmation),
    clarification_question: typeof parsed.clarification_question === "string" ? parsed.clarification_question : null,
    actions: Array.isArray(parsed.actions)
      ? (parsed.actions.filter(isPlannerAction) as PlannerAction[])
      : [],
  };
}

function parsePlannerAgentTurn(raw: string): PlannerToolCall | PlannerFinalResponse {
  let parsed: unknown;
  try {
    parsed = JSON.parse(extractJsonObject(raw));
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(`Planner response was not usable JSON. ${message}`);
  }

  if (!parsed || typeof parsed !== "object") {
    throw new Error("Planner response was not an object.");
  }

  const candidate = parsed as Record<string, unknown>;
  if (candidate.type === "tool_call") {
    const tool = candidate.tool;
    if (tool !== "list_products" && tool !== "get_product_tree" && tool !== "list_work_items") {
      throw new Error("Planner requested an unsupported tool.");
    }
    return {
      type: "tool_call",
      tool,
      arguments: candidate.arguments && typeof candidate.arguments === "object"
        ? (candidate.arguments as Record<string, unknown>)
        : {},
      reason: typeof candidate.reason === "string" ? candidate.reason : undefined,
    };
  }

  if (candidate.type === "final") {
    const plan = parsePlannerResponse(JSON.stringify(candidate));
    return {
      type: "final",
      assistant_response: plan.assistant_response,
      needs_confirmation: plan.needs_confirmation,
      clarification_question: plan.clarification_question,
      actions: plan.actions,
    };
  }

  const plan = parsePlannerResponse(JSON.stringify(candidate));
  return {
    type: "final",
    assistant_response: plan.assistant_response,
    needs_confirmation: plan.needs_confirmation,
    clarification_question: plan.clarification_question,
    actions: plan.actions,
  };
}

function isInformationalOnly(plan: PlannerPlan) {
  return plan.actions.length > 0 && plan.actions.every((action) => action.type === "report_status" || action.type === "report_tree");
}

function requiresConfirmation(plan: PlannerPlan) {
  if (plan.actions.length === 0) {
    return false;
  }
  return plan.needs_confirmation || !isInformationalOnly(plan);
}

function buildConversationHistory(messages: PlannerMessage[]) {
  return messages.slice(-8).map((message) => `${message.role.toUpperCase()}: ${message.content}`).join("\n");
}

function heuristicPlan(input: string): PlannerPlan {
  const lower = normalize(input);
  if (!lower) {
    return {
      assistant_response: "I need an instruction to act on.",
      needs_confirmation: false,
      clarification_question: "What do you want me to create, update, approve, or report on?",
      actions: [],
    };
  }
  if (lower === "yes" || lower === "confirm" || lower === "go ahead") {
    return {
      assistant_response: "Confirmation received.",
      needs_confirmation: false,
      clarification_question: null,
      actions: [],
    };
  }
  if ((lower.includes("tree") || lower.includes("hierarch")) && (lower.includes("work item") || lower.includes("workitem") || lower.includes("tasks"))) {
    return {
      assistant_response: "I’ll show the current work items in a hierarchical tree.",
      needs_confirmation: false,
      clarification_question: null,
      actions: [{ type: "report_tree" }],
    };
  }
  if (lower.includes("status")) {
    return {
      assistant_response: "I’ll report the status for the requested item if I can resolve it.",
      needs_confirmation: false,
      clarification_question: null,
      actions: [{ type: "report_status" }],
    };
  }
  if (lower.includes("approve")) {
    return {
      assistant_response: "I found an approval intent. I’ll hold it as a proposed action until you confirm.",
      needs_confirmation: true,
      clarification_question: null,
      actions: [{ type: "approve_work_item" }],
    };
  }
  return {
    assistant_response: "I need a model to turn open-ended planning conversation into structured suggestions.",
    needs_confirmation: false,
    clarification_question: "Configure a model, or tell me explicitly what product, capability, or work item you want me to assess.",
    actions: [],
  };
}

function detectLocalInformationalPlan(input: string): PlannerPlan | null {
  const lower = normalize(input);
  if ((lower.includes("tree") || lower.includes("hierarch")) && (lower.includes("work item") || lower.includes("workitem") || lower.includes("tasks"))) {
    return {
      assistant_response: "I’ll show the current work items in a hierarchical tree.",
      needs_confirmation: false,
      clarification_question: null,
      actions: [{ type: "report_tree" }],
    };
  }
  if (lower.includes("status")) {
    return {
      assistant_response: "I’ll report the current status from the local workspace data.",
      needs_confirmation: false,
      clarification_question: null,
      actions: [{ type: "report_status" }],
    };
  }
  return null;
}

function stringifyToolResult(value: unknown) {
  return JSON.stringify(value, null, 2);
}

function getStringArg(args: Record<string, unknown> | undefined, key: string) {
  const value = args?.[key];
  return typeof value === "string" && value.trim() ? value.trim() : undefined;
}

async function executePlannerReadTool(toolCall: PlannerToolCall, context: ResolverContext) {
  switch (toolCall.tool) {
    case "list_products":
      return context.products.map((product) => ({
        name: product.name,
        description: product.description,
        vision: product.vision,
        status: product.status,
        tags: product.tags,
      }));
    case "get_product_tree": {
      const product = findProduct(context, getStringArg(toolCall.arguments, "productName"));
      const tree = findTree(context, product);
      return {
        product: tree.product.name,
        modules: tree.modules.map((moduleTree) => ({
          name: moduleTree.module.name,
          description: moduleTree.module.description,
          capabilities: flattenCapabilities(moduleTree.features).map((capabilityTree) => ({
            name: capabilityTree.capability.name,
            description: capabilityTree.capability.description,
          })),
        })),
      };
    }
    case "list_work_items": {
      const productName = getStringArg(toolCall.arguments, "productName");
      const status = getStringArg(toolCall.arguments, "status");
      const scopedProduct = productName ? findProduct(context, productName) : null;
      return context.workItems
        .filter((item) => (scopedProduct ? item.product_id === scopedProduct.id : true))
        .filter((item) => (status ? item.status === status : true))
        .map((item) => ({
          title: item.title,
          status: item.status,
          product: context.products.find((product) => product.id === item.product_id)?.name ?? item.product_id,
          moduleId: item.module_id,
          capabilityId: item.capability_id,
          parentWorkItemId: item.parent_work_item_id,
        }));
    }
  }
}

async function runPlannerToolLoop(params: {
  providerId: string;
  modelName: string;
  context: ResolverContext;
  messages: PlannerMessage[];
  userInput: string;
}): Promise<PlannerPlan> {
  const contextSummary = summarizeContext(params.context);
  const conversationHistory = buildConversationHistory(params.messages);
  const toolMessages: Array<{ role: "user" | "assistant"; content: string }> = [
    {
      role: "user",
      content: `Current context snapshot:\n${contextSummary}\n\nRecent conversation:\n${conversationHistory || "No prior conversation."}\n\nLatest user request:\n${params.userInput}`,
    },
  ];

  for (let step = 0; step < 6; step += 1) {
    const completion = await runModelChatCompletion({
      providerId: params.providerId,
      model: params.modelName,
      temperature: 0.1,
      maxTokens: 1800,
      messages: [
        { role: "system", content: PLANNER_SYSTEM_PROMPT },
        ...toolMessages,
      ],
    });

    const turn = parsePlannerAgentTurn(completion.content);
    if (turn.type === "final") {
      return {
        assistant_response: turn.assistant_response,
        needs_confirmation: turn.needs_confirmation,
        clarification_question: turn.clarification_question,
        actions: turn.actions,
      };
    }

    const toolResult = await executePlannerReadTool(turn, params.context);
    toolMessages.push({
      role: "assistant",
      content: JSON.stringify(turn),
    });
    toolMessages.push({
      role: "user",
      content: `Tool result for ${turn.tool}:\n${stringifyToolResult(toolResult)}`,
    });
  }

  throw new Error("Planner exceeded tool-step limit before returning a final plan.");
}

function findProduct(context: ResolverContext, productName?: string) {
  if (productName) {
    const normalized = normalize(productName);
    const exact = context.products.find((product) => normalize(product.name) === normalized);
    if (exact) {
      return exact;
    }
    const partial = context.products.filter((product) => normalize(product.name).includes(normalized));
    if (partial.length === 1) {
      return partial[0];
    }
    if (partial.length > 1) {
      throw new Error(`Multiple products match "${productName}".`);
    }
    throw new Error(`No product matches "${productName}".`);
  }
  if (context.activeProductId) {
    const active = context.products.find((product) => product.id === context.activeProductId);
    if (active) {
      return active;
    }
  }
  if (context.products.length === 1) {
    return context.products[0];
  }
  throw new Error("Product is required.");
}

function formatElapsedMs(value: number) {
  const totalSeconds = Math.max(0, Math.round(value / 1000));
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  if (minutes > 0) {
    return `${minutes}:${String(seconds).padStart(2, "0")}`;
  }
  return `${seconds}s`;
}

function findTree(context: ResolverContext, product: Product) {
  const tree = context.productTrees.find((entry) => entry.product.id === product.id);
  if (!tree) {
    throw new Error(`Product tree for "${product.name}" is not loaded.`);
  }
  return tree;
}

function findModule(context: ResolverContext, product: Product, moduleName?: string) {
  const tree = findTree(context, product);
  if (moduleName) {
    const normalized = normalize(moduleName);
    const exact = tree.modules.find((entry) => normalize(entry.module.name) === normalized);
    if (exact) {
      return exact.module;
    }
    const partial = tree.modules.filter((entry) => normalize(entry.module.name).includes(normalized));
    if (partial.length === 1) {
      return partial[0].module;
    }
    if (partial.length > 1) {
      throw new Error(`Multiple modules match "${moduleName}" in "${product.name}".`);
    }
    throw new Error(`No module matches "${moduleName}" in "${product.name}".`);
  }
  if (context.activeModuleId) {
    const active = tree.modules.find((entry) => entry.module.id === context.activeModuleId);
    if (active) {
      return active.module;
    }
  }
  if (tree.modules.length === 1) {
    return tree.modules[0].module;
  }
  throw new Error("Module is required.");
}

function flattenCapabilities(tree: CapabilityTree[], bucket: CapabilityTree[] = []) {
  tree.forEach((node) => {
    bucket.push(node);
    flattenCapabilities(node.children, bucket);
  });
  return bucket;
}

function findCapability(context: ResolverContext, product: Product, moduleName?: string, capabilityName?: string) {
  const module = findModule(context, product, moduleName);
  const tree = findTree(context, product);
  const moduleTree = tree.modules.find((entry) => entry.module.id === module.id);
  if (!moduleTree) {
    throw new Error(`Module "${module.name}" has no capability tree.`);
  }
  const capabilities = flattenCapabilities(moduleTree.features);
  if (capabilityName) {
    const normalized = normalize(capabilityName);
    const exact = capabilities.find((entry) => normalize(entry.capability.name) === normalized);
    if (exact) {
      return exact.capability;
    }
    const partial = capabilities.filter((entry) => normalize(entry.capability.name).includes(normalized));
    if (partial.length === 1) {
      return partial[0].capability;
    }
    if (partial.length > 1) {
      throw new Error(`Multiple capabilities match "${capabilityName}" in "${module.name}".`);
    }
    throw new Error(`No capability matches "${capabilityName}" in "${module.name}".`);
  }
  if (context.activeCapabilityId) {
    const active = capabilities.find((entry) => entry.capability.id === context.activeCapabilityId);
    if (active) {
      return active.capability;
    }
  }
  throw new Error("Capability is required.");
}

function findWorkItem(context: ResolverContext, workItemTitle?: string, productName?: string) {
  const inScope = productName
    ? context.workItems.filter((item) => {
        const product = context.products.find((entry) => entry.id === item.product_id);
        return product && normalize(product.name) === normalize(productName);
      })
    : context.workItems;
  if (workItemTitle) {
    const normalized = normalize(workItemTitle);
    const exact = inScope.find((item) => normalize(item.title) === normalized);
    if (exact) {
      return exact;
    }
    const partial = inScope.filter((item) => normalize(item.title).includes(normalized));
    if (partial.length === 1) {
      return partial[0];
    }
    if (partial.length > 1) {
      throw new Error(`Multiple work items match "${workItemTitle}".`);
    }
    throw new Error(`No work item matches "${workItemTitle}".`);
  }
  if (context.activeWorkItemId) {
    const active = context.workItems.find((item) => item.id === context.activeWorkItemId);
    if (active) {
      return active;
    }
  }
  throw new Error("Work item is required.");
}

function formatArrayField(values?: string[]) {
  return values?.join(", ") ?? "";
}

function isPlannerAction(value: unknown): value is PlannerAction {
  if (!value || typeof value !== "object") {
    return false;
  }
  const candidate = value as { type?: unknown };
  if (typeof candidate.type !== "string") {
    return false;
  }
  return new Set([
    "create_product",
    "update_product",
    "archive_product",
    "create_module",
    "update_module",
    "delete_module",
    "create_capability",
    "update_capability",
    "delete_capability",
    "create_work_item",
    "update_work_item",
    "delete_work_item",
    "approve_work_item",
    "reject_work_item",
    "approve_work_item_plan",
    "reject_work_item_plan",
    "approve_work_item_test_review",
    "start_workflow",
    "workflow_action",
    "report_status",
    "report_tree",
  ]).has(candidate.type);
}

function formatWorkItemLine(workItem: WorkItem, indent: string) {
  return `${indent}- ${workItem.title} [${workItem.status}]`;
}

function appendWorkItemHierarchy(lines: string[], items: WorkItem[], parentId: string | null, indent: string) {
  const children = items
    .filter((item) => (item.parent_work_item_id ?? null) === parentId)
    .sort((left, right) => left.sort_order - right.sort_order || left.title.localeCompare(right.title));
  children.forEach((child) => {
    lines.push(formatWorkItemLine(child, indent));
    appendWorkItemHierarchy(lines, items, child.id, `${indent}  `);
  });
}

function buildWorkItemTreeReport(context: ResolverContext, productName?: string) {
  const lines: string[] = [];
  const products = productName ? [findProduct(context, productName)] : context.products;

  products.forEach((product) => {
    lines.push(product.name);
    const tree = context.productTrees.find((entry) => entry.product.id === product.id);
    const productItems = context.workItems.filter((item) => item.product_id === product.id);

    if (!tree) {
      appendWorkItemHierarchy(lines, productItems, null, "  ");
      lines.push("");
      return;
    }

    const includedWorkItemIds = new Set<string>();

    tree.modules.forEach((moduleTree) => {
      lines.push(`  ${moduleTree.module.name}`);

      const moduleDirectItems = productItems.filter(
        (item) => item.module_id === moduleTree.module.id && !item.capability_id,
      );
      if (moduleDirectItems.length > 0) {
        lines.push("    direct work items");
        appendWorkItemHierarchy(lines, moduleDirectItems, null, "      ");
        moduleDirectItems.forEach((item) => includedWorkItemIds.add(item.id));
      }

      const flattenedCapabilities = flattenCapabilities(moduleTree.features);
      flattenedCapabilities.forEach((capabilityTree) => {
        const capabilityItems = productItems.filter((item) => item.capability_id === capabilityTree.capability.id);
        if (capabilityItems.length === 0) {
          return;
        }
        lines.push(`    ${capabilityTree.capability.name}`);
        appendWorkItemHierarchy(lines, capabilityItems, null, "      ");
        capabilityItems.forEach((item) => includedWorkItemIds.add(item.id));
      });
    });

    const unscopedItems = productItems.filter(
      (item) => !includedWorkItemIds.has(item.id) && !item.parent_work_item_id,
    );
    if (unscopedItems.length > 0) {
      lines.push("  unscoped");
      appendWorkItemHierarchy(lines, unscopedItems, null, "    ");
      unscopedItems.forEach((item) => includedWorkItemIds.add(item.id));
    }

    if (productItems.length === 0) {
      lines.push("  no work items");
    }
    lines.push("");
  });

  return lines.join("\n").trim();
}

function buildWorkItemTreeNodes(context: ResolverContext, productName?: string): PlannerTreeNode[] {
  const products = productName ? [findProduct(context, productName)] : context.products;

  const buildWorkItemNodes = (items: WorkItem[], parentId: string | null): PlannerTreeNode[] =>
    items
      .filter((item) => (item.parent_work_item_id ?? null) === parentId)
      .sort((left, right) => left.sort_order - right.sort_order || left.title.localeCompare(right.title))
      .map((item) => ({
        id: item.id,
        label: item.title,
        meta: item.status,
        children: buildWorkItemNodes(items, item.id),
      }));

  return products.map((product) => {
    const tree = context.productTrees.find((entry) => entry.product.id === product.id);
    const productItems = context.workItems.filter((item) => item.product_id === product.id);
    const includedWorkItemIds = new Set<string>();
    const moduleNodes: PlannerTreeNode[] = [];

    if (tree) {
      tree.modules.forEach((moduleTree) => {
        const moduleChildren: PlannerTreeNode[] = [];
        const moduleDirectItems = productItems.filter(
          (item) => item.module_id === moduleTree.module.id && !item.capability_id,
        );
        if (moduleDirectItems.length > 0) {
          moduleChildren.push({
            id: `${moduleTree.module.id}-direct`,
            label: "Direct Work Items",
            children: buildWorkItemNodes(moduleDirectItems, null),
          });
          moduleDirectItems.forEach((item) => includedWorkItemIds.add(item.id));
        }

        flattenCapabilities(moduleTree.features).forEach((capabilityTree) => {
          const capabilityItems = productItems.filter((item) => item.capability_id === capabilityTree.capability.id);
          if (capabilityItems.length === 0) {
            return;
          }
          moduleChildren.push({
            id: capabilityTree.capability.id,
            label: capabilityTree.capability.name,
            children: buildWorkItemNodes(capabilityItems, null),
          });
          capabilityItems.forEach((item) => includedWorkItemIds.add(item.id));
        });

        moduleNodes.push({
          id: moduleTree.module.id,
          label: moduleTree.module.name,
          children: moduleChildren,
        });
      });
    }

    const unscopedItems = productItems.filter(
      (item) => !includedWorkItemIds.has(item.id) && !item.parent_work_item_id,
    );
    if (unscopedItems.length > 0) {
      moduleNodes.push({
        id: `${product.id}-unscoped`,
        label: "Unscoped",
        children: buildWorkItemNodes(unscopedItems, null),
      });
    }

    if (moduleNodes.length === 0) {
      moduleNodes.push({
        id: `${product.id}-empty`,
        label: "No work items",
        meta: "empty",
        children: [],
      });
    }

    return {
      id: product.id,
      label: product.name,
      children: moduleNodes,
    };
  });
}

function summarizeAction(action: PlannerAction | Record<string, unknown> | null | undefined) {
  if (!action || typeof action !== "object") {
    return {
      symbol: "?",
      tone: "warn" as const,
      title: "Unknown planner action",
      detail: "The planner returned an empty or invalid action payload.",
    };
  }
  const raw = action as Record<string, unknown>;
  const actionType = typeof (action as { type?: unknown }).type === "string"
    ? String((action as { type: string }).type)
    : "unknown_action";
  const target = raw.target as { productName?: string; moduleName?: string; capabilityName?: string; workItemTitle?: string } | undefined;
  const name = typeof raw.name === "string" ? raw.name : undefined;
  const title = typeof raw.title === "string" ? raw.title : undefined;
  const description = typeof raw.description === "string" ? raw.description : undefined;
  const vision = typeof raw.vision === "string" ? raw.vision : undefined;
  const fields = raw.fields ?? undefined;
  switch (actionType) {
    case "create_product":
      return { symbol: "+", tone: "add", title: `Create product ${name ?? target?.productName ?? "unnamed product"}`, detail: description || vision || "New product proposal." };
    case "create_module":
      return { symbol: "+", tone: "add", title: `Create module ${name ?? target?.moduleName ?? "unnamed module"}`, detail: target?.productName ? `Product: ${target.productName}` : "Attach to selected product." };
    case "create_capability":
      return { symbol: "+", tone: "add", title: `Create capability ${name ?? target?.capabilityName ?? "unnamed capability"}`, detail: [target?.productName, target?.moduleName].filter(Boolean).join(" / ") || "Attach to selected scope." };
    case "create_work_item":
      return { symbol: "+", tone: "add", title: `Create work item ${title ?? target?.workItemTitle ?? "untitled work item"}`, detail: [target?.productName, target?.moduleName, target?.capabilityName].filter(Boolean).join(" / ") || description || "New work item proposal." };
    case "update_product":
      return { symbol: "~", tone: "update", title: `Update product ${target?.productName ?? ""}`.trim(), detail: JSON.stringify(fields, null, 2) };
    case "update_module":
      return { symbol: "~", tone: "update", title: `Update module ${target?.moduleName ?? ""}`.trim(), detail: JSON.stringify(fields, null, 2) };
    case "update_capability":
      return { symbol: "~", tone: "update", title: `Update capability ${target?.capabilityName ?? ""}`.trim(), detail: JSON.stringify(fields, null, 2) };
    case "update_work_item":
      return { symbol: "~", tone: "update", title: `Update work item ${target?.workItemTitle ?? ""}`.trim(), detail: JSON.stringify(fields, null, 2) };
    case "approve_work_item":
    case "approve_work_item_plan":
    case "approve_work_item_test_review":
    case "start_workflow":
    case "workflow_action":
    case "reject_work_item":
    case "reject_work_item_plan":
    case "archive_product":
    case "delete_module":
    case "delete_capability":
    case "delete_work_item":
      return { symbol: "!", tone: "warn", title: actionType.replace(/_/g, " "), detail: JSON.stringify(action, null, 2) };
    case "report_status":
      return { symbol: "i", tone: "update", title: "Status report", detail: target?.productName || target?.workItemTitle || "Current scope" };
    case "report_tree":
      return { symbol: "i", tone: "update", title: "Tree report", detail: target?.productName || "All products" };
    default:
      return {
        symbol: "?",
        tone: "warn",
        title: actionType.replace(/_/g, " "),
        detail: JSON.stringify(action, null, 2),
      };
  }
}

function getReportTreeProductName(plan: PlannerPlan) {
  const treeAction = plan.actions.find((action): action is Extract<PlannerAction, { type: "report_tree" }> => action.type === "report_tree");
  return treeAction?.target?.productName;
}

function TreeNodeView({ node }: { node: PlannerTreeNode }) {
  if (node.children.length === 0) {
    return (
      <div style={styles.treeLeaf}>
        {node.label}
        {node.meta ? <span style={styles.treeMeta}>{node.meta}</span> : null}
      </div>
    );
  }
  return (
    <details open style={styles.treeNode}>
      <summary style={styles.treeSummary}>
        {node.label}
        {node.meta ? <span style={styles.treeMeta}>{node.meta}</span> : null}
      </summary>
      <div style={styles.treeChildren}>
        {node.children.map((child) => (
          <TreeNodeView key={child.id} node={child} />
        ))}
      </div>
    </details>
  );
}

function PlannerComposer({
  draft,
  onDraftChange,
  onSend,
  onToggleListening,
  onConfirm,
  onDismiss,
  isPlannerBusy,
  voiceEnabled,
  isListening,
  isTranscribing,
  isVoiceSubmitting,
  pendingVoiceTranscript,
  draftTreeNodesLength,
  pendingPlan,
  voiceActivity,
  composerRef,
}: {
  draft: string;
  onDraftChange: (value: string) => void;
  onSend: () => void;
  onToggleListening: () => void;
  onConfirm: () => void;
  onDismiss: () => void;
  isPlannerBusy: boolean;
  voiceEnabled: boolean;
  isListening: boolean;
  isTranscribing: boolean;
  isVoiceSubmitting: boolean;
  pendingVoiceTranscript: string | null;
  draftTreeNodesLength: number;
  pendingPlan: PendingPlan | null;
  voiceActivity: string | null;
  composerRef: React.RefObject<HTMLTextAreaElement | null>;
}) {
  return (
    <div style={styles.composerWrap}>
      <textarea
        ref={composerRef}
        data-testid="planner-input"
        style={styles.textarea}
        value={draft}
        onChange={(event) => onDraftChange(event.target.value)}
        placeholder="Say or type what you need. Example: Add a work item called Build voice planner under AruviStudio, then approve it and start the workflow."
      />
      <div style={styles.actionRow}>
        <button data-testid="planner-send" style={styles.btn} onClick={onSend} disabled={isPlannerBusy}>
          {isPlannerBusy ? "Working..." : "Send"}
        </button>
        <button style={styles.btnGhost} onClick={onToggleListening} disabled={!voiceEnabled || isTranscribing || isVoiceSubmitting || Boolean(pendingVoiceTranscript)}>
          {isListening
            ? "Stop Recording"
            : isTranscribing
              ? "Transcribing..."
              : isVoiceSubmitting
                ? "Sending Voice..."
                : "Start Voice Input"}
        </button>
        <button style={styles.btnGhost} onClick={onConfirm} disabled={!pendingPlan && draftTreeNodesLength === 0}>
          {draftTreeNodesLength > 0 ? "Commit Draft" : "Confirm Proposal"}
        </button>
        <button style={styles.btnDanger} onClick={onDismiss} disabled={!pendingPlan && draftTreeNodesLength === 0}>
          {draftTreeNodesLength > 0 ? "Clear Draft" : "Clear Pending"}
        </button>
        <span style={styles.status}>
          {voiceActivity
            ? voiceActivity
            : pendingVoiceTranscript
              ? "Voice transcript is ready. Review, edit, send, retry, or cancel."
              : draftTreeNodesLength > 0
                ? "A staged draft is active. Keep refining it, then commit when ready."
                : pendingPlan
                  ? "A proposed plan is waiting for confirmation."
                  : "No pending proposal."}
        </span>
      </div>
    </div>
  );
}

function parseDraftNodeType(meta?: string | null) {
  if (!meta) {
    return "node";
  }
  if (meta.includes("product")) {
    return "product";
  }
  if (meta.includes("module")) {
    return "module";
  }
  if (meta.includes("capability")) {
    return "capability";
  }
  if (meta.includes("work item")) {
    return "work item";
  }
  return "node";
}

function getPlannerNodeType(node: PlannerTreeNode | null | undefined) {
  if (!node) {
    return "node";
  }
  if (node.node_type) {
    return node.node_type.replace("_", " ");
  }
  return parseDraftNodeType(node.meta);
}

function collectTreeNodeIds(nodes: PlannerTreeNode[]): string[] {
  return nodes.flatMap((node) => [node.id, ...collectTreeNodeIds(node.children)]);
}

function SelectableTreeNodeView({
  node,
  selectedNodeId,
  onSelect,
  expandedNodeIds,
  onToggle,
}: {
  node: PlannerTreeNode;
  selectedNodeId: string | null;
  onSelect: (nodeId: string) => void;
  expandedNodeIds: Set<string>;
  onToggle: (nodeId: string) => void;
}) {
  const isSelected = node.id === selectedNodeId;
  const hasChildren = node.children.length > 0;
  const isExpanded = expandedNodeIds.has(node.id);
  const nodeType = getPlannerNodeType(node);
  const cardStyle = isSelected ? { ...styles.treeCard, ...styles.treeCardSelected } : styles.treeCard;
  return (
    <div style={styles.treeLevel}>
      <div style={styles.treeRow}>
        {hasChildren ? (
          <button type="button" style={styles.treeToggle} onClick={() => onToggle(node.id)} data-testid={`draft-node-toggle-${node.id}`}>
            {isExpanded ? "▾" : "▸"}
          </button>
        ) : (
          <div style={styles.treeToggleGhost}>•</div>
        )}
        <button type="button" style={cardStyle} onClick={() => onSelect(node.id)} data-testid={`draft-node-${node.id}`}>
          <div style={styles.treeCardHeader}>
            <div style={styles.treeCardTitle}>{node.label}</div>
            <div style={styles.treeCardMetaRow}>
              <span style={styles.treeTypeBadge}>{nodeType}</span>
              {hasChildren ? <span style={styles.treeCountBadge}>{node.children.length} children</span> : null}
              {node.confidence ? <span style={styles.treeCountBadge}>{node.confidence} confidence</span> : null}
            </div>
          </div>
          {node.summary ? <div style={styles.diffSecondary}>{node.summary}</div> : null}
          {node.meta ? <div style={styles.diffSecondary}>{node.meta}</div> : null}
        </button>
      </div>
      {hasChildren && isExpanded ? (
        <div style={styles.treeRowChildren}>
          {node.children.map((child) => (
            <SelectableTreeNodeView
              key={child.id}
              node={child}
              selectedNodeId={selectedNodeId}
              onSelect={onSelect}
              expandedNodeIds={expandedNodeIds}
              onToggle={onToggle}
            />
          ))}
        </div>
      ) : null}
    </div>
  );
}

function buildProposalTreeNodes(plan: PlannerPlan): PlannerTreeNode[] {
  const productNodes = new Map<string, PlannerTreeNode>();
  const moduleNodes = new Map<string, PlannerTreeNode>();
  const capabilityNodes = new Map<string, PlannerTreeNode>();

  const ensureProduct = (name?: string | null) => {
    const label = name?.trim() || "Proposed product";
    let node = productNodes.get(label);
    if (!node) {
      node = { id: `proposal-product-${label}`, label, meta: "proposed product", node_type: "product", evidence: [], children: [] };
      productNodes.set(label, node);
    }
    return node!;
  };

  const ensureModule = (productName?: string | null, moduleName?: string | null) => {
    const product = ensureProduct(productName);
    const label = moduleName?.trim() || "Proposed module";
    const key = `${product.label}::${label}`;
    let node = moduleNodes.get(key);
    if (!node) {
      node = { id: `proposal-module-${key}`, label, meta: "proposed module", node_type: "module", evidence: [], children: [] };
      moduleNodes.set(key, node);
      product.children.push(node);
    }
    return node!;
  };

  const ensureCapability = (productName?: string | null, moduleName?: string | null, capabilityName?: string | null) => {
    const module = ensureModule(productName, moduleName);
    const label = capabilityName?.trim() || "Proposed capability";
    const key = `${module.id}::${label}`;
    let node = capabilityNodes.get(key);
    if (!node) {
      node = { id: `proposal-capability-${key}`, label, meta: "proposed capability", node_type: "capability", evidence: [], children: [] };
      capabilityNodes.set(key, node);
      module.children.push(node);
    }
    return node!;
  };

  for (const action of plan.actions) {
    const target = (action as { target?: { productName?: string; moduleName?: string; capabilityName?: string; workItemTitle?: string } }).target;
    switch (action.type) {
      case "create_product":
        ensureProduct(action.name ?? target?.productName ?? null);
        break;
      case "create_module":
        ensureModule(target?.productName, action.name ?? target?.moduleName ?? null);
        break;
      case "create_capability":
        ensureCapability(target?.productName, target?.moduleName, action.name ?? target?.capabilityName ?? null);
        break;
      case "create_work_item": {
        const capability = ensureCapability(target?.productName, target?.moduleName, target?.capabilityName ?? null);
        capability.children.push({
          id: `proposal-work-item-${capability.id}-${action.title ?? target?.workItemTitle ?? capability.children.length}`,
          label: action.title ?? target?.workItemTitle ?? "Proposed work item",
          meta: "proposed work item",
          node_type: "work_item",
          evidence: [],
          children: [],
        });
        break;
      }
      default:
        break;
    }
  }

  return Array.from(productNodes.values());
}

function findTreeNodeById(nodes: PlannerTreeNode[], nodeId: string | null): PlannerTreeNode | null {
  if (!nodeId) {
    return null;
  }
  for (const node of nodes) {
    if (node.id === nodeId) {
      return node;
    }
    const child = findTreeNodeById(node.children, nodeId);
    if (child) {
      return child;
    }
  }
  return null;
}

function findTreeNodePath(nodes: PlannerTreeNode[], nodeId: string | null, trail: PlannerTreeNode[] = []): PlannerTreeNode[] {
  if (!nodeId) {
    return [];
  }
  for (const node of nodes) {
    const nextTrail = [...trail, node];
    if (node.id === nodeId) {
      return nextTrail;
    }
    const childPath = findTreeNodePath(node.children, nodeId, nextTrail);
    if (childPath.length > 0) {
      return childPath;
    }
  }
  return [];
}

function flattenTreeNodes(nodes: PlannerTreeNode[]): PlannerTreeNode[] {
  return nodes.flatMap((node) => [node, ...flattenTreeNodes(node.children)]);
}

function findAncestorNodeByType(path: PlannerTreeNode[], nodeType: string) {
  for (let index = path.length - 1; index >= 0; index -= 1) {
    if (getPlannerNodeType(path[index]) === nodeType) {
      return path[index];
    }
  }
  return null;
}

function resolveVoiceNodeReference(
  nodes: PlannerTreeNode[],
  selectedPath: PlannerTreeNode[],
  rawReference: string,
  explicitType?: string,
) {
  const reference = rawReference.trim().toLowerCase();
  if (!reference) {
    return null;
  }

  if (["this", "selected", "this node", "selected node"].includes(reference)) {
    return explicitType
      ? findAncestorNodeByType(selectedPath, explicitType)
      : (selectedPath.length > 0 ? selectedPath[selectedPath.length - 1] : null);
  }
  if (["root", "product", "this product", "selected product", "root product"].includes(reference)) {
    return findAncestorNodeByType(selectedPath, "product") ?? nodes[0] ?? null;
  }
  if (["this module", "selected module"].includes(reference)) {
    return findAncestorNodeByType(selectedPath, "module");
  }
  if (["this capability", "selected capability"].includes(reference)) {
    return findAncestorNodeByType(selectedPath, "capability");
  }
  if (["this work item", "selected work item"].includes(reference)) {
    return findAncestorNodeByType(selectedPath, "work item");
  }

  const normalizedType = explicitType?.replace("-", " ");
  const flattened = flattenTreeNodes(nodes).filter((node) => {
    if (!normalizedType) {
      return true;
    }
    return getPlannerNodeType(node) === normalizedType;
  });
  const exact = flattened.find((node) => node.label.trim().toLowerCase() === reference);
  if (exact) {
    return exact;
  }
  return flattened.find((node) => node.label.trim().toLowerCase().includes(reference)) ?? null;
}

type DraftValidationIssue = {
  tone: "ok" | "warn";
  title: string;
  detail: string;
};

type DraftValidationSummary = {
  score: number;
  counts: Record<"product" | "module" | "capability" | "work item", number>;
  issues: DraftValidationIssue[];
};

function buildDraftValidation(nodes: PlannerTreeNode[]): DraftValidationSummary {
  const counts: DraftValidationSummary["counts"] = {
    product: 0,
    module: 0,
    capability: 0,
    "work item": 0,
  };
  const issues: DraftValidationIssue[] = [];

  function visit(node: PlannerTreeNode) {
    const nodeType = getPlannerNodeType(node);
    if (nodeType in counts) {
      counts[nodeType as keyof typeof counts] += 1;
    }

    const seenSiblingNames = new Set<string>();
    for (const child of node.children) {
      const normalizedLabel = child.label.trim().toLowerCase();
      if (seenSiblingNames.has(normalizedLabel)) {
        issues.push({
          tone: "warn",
          title: `Duplicate child under ${node.label}`,
          detail: `Multiple children under this branch share the name "${child.label}".`,
        });
      } else {
        seenSiblingNames.add(normalizedLabel);
      }
    }

    if (nodeType === "product" && node.children.length === 0) {
      issues.push({
        tone: "warn",
        title: `${node.label} needs modules`,
        detail: "Products should usually have at least one module before commit.",
      });
    }
    if (nodeType === "module" && node.children.length === 0) {
      issues.push({
        tone: "warn",
        title: `${node.label} is empty`,
        detail: "Modules should contain capabilities or direct work items so the plan is actionable.",
      });
    }
    if (nodeType === "capability" && node.children.length === 0) {
      issues.push({
        tone: "warn",
        title: `${node.label} has no work items`,
        detail: "Capabilities are stronger when they break down into implementation work items.",
      });
    }

    node.children.forEach(visit);
  }

  nodes.forEach(visit);

  if (counts.product === 0) {
    issues.push({
      tone: "warn",
      title: "No staged product root",
      detail: "The draft needs a product root before it can be committed to the catalog.",
    });
  } else {
    issues.unshift({
      tone: "ok",
      title: "Draft tree is structurally valid",
      detail: "A product root exists and the planner can keep refining the staged hierarchy before commit.",
    });
  }

  const warningCount = issues.filter((issue) => issue.tone === "warn").length;
  const score = Math.max(35, 100 - warningCount * 12);
  return { score, counts, issues };
}

function buildSuggestedPrompts(node: PlannerTreeNode | null): string[] {
  if (!node) {
    return [
      "Design the full product, modules, capabilities, and starter work items in one draft.",
      "Show me what is missing in this plan before I commit it.",
    ];
  }
  const resolvedNodeType = getPlannerNodeType(node);
  switch (resolvedNodeType) {
    case "product":
      return [
        `Expand ${node.label} with missing modules and operational areas.`,
        `What is missing under ${node.label} before I commit it?`,
        `Add notification, reporting, and integration modules under ${node.label}.`,
      ];
    case "module":
      return [
        `Enhance ${node.label} with 3 concrete capabilities.`,
        `Break ${node.label} into implementation-ready capabilities and work items.`,
        `What risks or missing outcomes exist under ${node.label}?`,
      ];
    case "capability":
      return [
        `Add implementation work items under ${node.label}.`,
        `Revise ${node.label} to be more concrete and execution-ready.`,
        `What acceptance criteria or technical notes are missing for ${node.label}?`,
      ];
    case "work item":
      return [
        `Revise ${node.label} to be more specific and testable.`,
        `Split ${node.label} into smaller work items if needed.`,
        `Add risks, constraints, and acceptance criteria to ${node.label}.`,
      ];
    default:
      return [
        `Expand ${node.label}.`,
        `What is missing under ${node.label}?`,
      ];
  }
}

function getAllowedDraftChildTypes(node: PlannerTreeNode | null): PlannerDraftChildType[] {
  const nodeType = getPlannerNodeType(node);
  switch (nodeType) {
    case "product":
      return ["module", "work_item"];
    case "module":
      return ["capability", "work_item"];
    case "capability":
      return ["work_item"];
    default:
      return [];
  }
}

function formatDraftChildTypeLabel(type: PlannerDraftChildType) {
  switch (type) {
    case "work_item":
      return "Work Item";
    case "module":
      return "Module";
    case "capability":
      return "Capability";
  }
}

function findRelevantPlanActions(plan: PlannerPlan | null, node: PlannerTreeNode | null) {
  if (!plan || !node) {
    return [];
  }

  const nodeType = getPlannerNodeType(node);
  return plan.actions.filter((action) => {
    const target = (action as { target?: { productName?: string; moduleName?: string; capabilityName?: string; workItemTitle?: string } }).target;
    if (nodeType === "product") {
      return action.type === "create_product"
        ? action.name === node.label
        : target?.productName === node.label;
    }
    if (nodeType === "module") {
      return action.type === "create_module"
        ? action.name === node.label || target?.moduleName === node.label
        : target?.moduleName === node.label;
    }
    if (nodeType === "capability") {
      return action.type === "create_capability"
        ? action.name === node.label || target?.capabilityName === node.label
        : target?.capabilityName === node.label;
    }
    if (nodeType === "work item") {
      return action.type === "create_work_item"
        ? action.title === node.label || target?.workItemTitle === node.label
        : target?.workItemTitle === node.label;
    }
    return false;
  });
}

async function executePlannerAction(action: PlannerAction, context: ResolverContext): Promise<string[]> {
  switch (action.type) {
    case "create_product": {
      const product = await createProduct({
        name: action.name,
        description: action.description ?? "",
        vision: action.vision ?? "",
        goals: formatArrayField(action.goals),
        tags: formatArrayField(action.tags),
      });
      return [`Created product "${product.name}".`];
    }
    case "update_product": {
      const product = findProduct(context, action.target?.productName);
      const updated = await updateProduct({
        id: product.id,
        name: action.fields.name,
        description: action.fields.description,
        vision: action.fields.vision,
        goals: action.fields.goals ? formatArrayField(action.fields.goals) : undefined,
        tags: action.fields.tags ? formatArrayField(action.fields.tags) : undefined,
      });
      return [`Updated product "${updated.name}".`];
    }
    case "archive_product": {
      const product = findProduct(context, action.target?.productName);
      await archiveProduct(product.id);
      return [`Archived product "${product.name}".`];
    }
    case "create_module": {
      const product = findProduct(context, action.target?.productName);
      const module = await createModule({
        productId: product.id,
        name: action.name,
        description: action.description ?? "",
        purpose: action.purpose ?? "",
      });
      return [`Created module "${module.name}" in "${product.name}".`];
    }
    case "update_module": {
      const product = findProduct(context, action.target?.productName);
      const module = findModule(context, product, action.target?.moduleName);
      const updated = await updateModule({
        id: module.id,
        name: action.fields.name,
        description: action.fields.description,
        purpose: action.fields.purpose,
      });
      return [`Updated module "${updated.name}" in "${product.name}".`];
    }
    case "delete_module": {
      const product = findProduct(context, action.target?.productName);
      const module = findModule(context, product, action.target?.moduleName);
      await deleteModule(module.id);
      return [`Deleted module "${module.name}" from "${product.name}".`];
    }
    case "create_capability": {
      const product = findProduct(context, action.target?.productName);
      const module = findModule(context, product, action.target?.moduleName);
      const parentCapability = action.target?.capabilityName
        ? findCapability(context, product, module.name, action.target.capabilityName)
        : null;
      const capability = await createCapability({
        moduleId: module.id,
        parentCapabilityId: parentCapability?.id,
        name: action.name,
        description: action.description ?? "",
        acceptanceCriteria: action.acceptanceCriteria ?? "",
        priority: action.priority ?? "medium",
        risk: action.risk ?? "medium",
        technicalNotes: action.technicalNotes ?? "",
      });
      return [`Created capability "${capability.name}" in "${module.name}".`];
    }
    case "update_capability": {
      const product = findProduct(context, action.target?.productName);
      const capability = findCapability(context, product, action.target?.moduleName, action.target?.capabilityName);
      const updated = await updateCapability({
        id: capability.id,
        name: action.fields.name,
        description: action.fields.description,
        acceptanceCriteria: action.fields.acceptanceCriteria,
        technicalNotes: action.fields.technicalNotes,
        priority: action.fields.priority,
        risk: action.fields.risk,
      });
      return [`Updated capability "${updated.name}".`];
    }
    case "delete_capability": {
      const product = findProduct(context, action.target?.productName);
      const capability = findCapability(context, product, action.target?.moduleName, action.target?.capabilityName);
      await deleteCapability(capability.id);
      return [`Deleted capability "${capability.name}".`];
    }
    case "create_work_item": {
      const product = findProduct(context, action.target?.productName);
      const module = action.target?.moduleName ? findModule(context, product, action.target.moduleName) : context.activeModuleId ? findModule(context, product, undefined) : null;
      const capability = action.target?.capabilityName ? findCapability(context, product, action.target?.moduleName, action.target.capabilityName) : context.activeCapabilityId ? findCapability(context, product, module?.name, undefined) : null;
      const workItem = await createWorkItem({
        productId: product.id,
        moduleId: module?.id,
        capabilityId: capability?.id,
        title: action.title,
        problemStatement: action.problemStatement ?? action.description ?? "",
        description: action.description ?? "",
        acceptanceCriteria: action.acceptanceCriteria ?? "",
        constraints: action.constraints ?? "",
        workItemType: action.workItemType ?? "feature",
        priority: action.priority ?? "medium",
        complexity: action.complexity ?? "medium",
      });
      return [`Created work item "${workItem.title}" in "${product.name}".`];
    }
    case "update_work_item": {
      const workItem = findWorkItem(context, action.target?.workItemTitle, action.target?.productName);
      const updated = await updateWorkItem({
        id: workItem.id,
        title: action.fields.title,
        description: action.fields.description,
        problemStatement: action.fields.problemStatement,
        acceptanceCriteria: action.fields.acceptanceCriteria,
        constraints: action.fields.constraints,
        status: action.fields.status,
      });
      return [`Updated work item "${updated.title}".`];
    }
    case "delete_work_item": {
      const workItem = findWorkItem(context, action.target?.workItemTitle, action.target?.productName);
      await deleteWorkItem(workItem.id);
      return [`Deleted work item "${workItem.title}".`];
    }
    case "approve_work_item": {
      const workItem = findWorkItem(context, action.target?.workItemTitle, action.target?.productName);
      await approveWorkItem(workItem.id, action.notes);
      return [`Approved work item "${workItem.title}".`];
    }
    case "reject_work_item": {
      const workItem = findWorkItem(context, action.target?.workItemTitle, action.target?.productName);
      await rejectWorkItem(workItem.id, action.notes ?? "Rejected from interactive planner.");
      return [`Rejected work item "${workItem.title}".`];
    }
    case "approve_work_item_plan": {
      const workItem = findWorkItem(context, action.target?.workItemTitle, action.target?.productName);
      await approveWorkItemPlan(workItem.id, action.notes);
      return [`Approved plan for "${workItem.title}".`];
    }
    case "reject_work_item_plan": {
      const workItem = findWorkItem(context, action.target?.workItemTitle, action.target?.productName);
      await rejectWorkItemPlan(workItem.id, action.notes ?? "Rejected from interactive planner.");
      return [`Rejected plan for "${workItem.title}".`];
    }
    case "approve_work_item_test_review": {
      const workItem = findWorkItem(context, action.target?.workItemTitle, action.target?.productName);
      await approveWorkItemTestReview(workItem.id, action.notes);
      return [`Approved test review for "${workItem.title}".`];
    }
    case "start_workflow": {
      const workItem = findWorkItem(context, action.target?.workItemTitle, action.target?.productName);
      await startWorkItemWorkflow(workItem.id);
      return [`Started workflow for "${workItem.title}".`];
    }
    case "workflow_action": {
      const workItem = findWorkItem(context, action.target?.workItemTitle, action.target?.productName);
      const run = await getLatestWorkflowRunForWorkItem(workItem.id);
      if (!run) {
        throw new Error(`No workflow run exists for "${workItem.title}".`);
      }
      await handleWorkflowUserAction({
        workflowRunId: run.id,
        action: action.action,
        notes: action.notes,
      });
      return [`Applied workflow action "${action.action}" to "${workItem.title}".`];
    }
    case "report_status": {
      const workItem = action.target?.workItemTitle || context.activeWorkItemId
        ? findWorkItem(context, action.target?.workItemTitle, action.target?.productName)
        : null;
      if (workItem) {
        const run = await getLatestWorkflowRunForWorkItem(workItem.id);
        const product = context.products.find((entry) => entry.id === workItem.product_id);
        return [
          `Status for "${workItem.title}": ${workItem.status}.`,
          `Product: ${product?.name ?? "unknown"}.`,
          run ? `Workflow: ${run.status} at ${run.current_stage}.` : "Workflow: not started.",
        ];
      }
      const product = action.target?.productName ? findProduct(context, action.target.productName) : findProduct(context, undefined);
      const scopedItems = context.workItems.filter((item) => item.product_id === product.id);
      const statusCounts = scopedItems.reduce<Record<string, number>>((acc, item) => {
        acc[item.status] = (acc[item.status] ?? 0) + 1;
        return acc;
      }, {});
      return [
        `Status for "${product.name}".`,
        ...Object.entries(statusCounts).map(([status, count]) => `${status}: ${count}`),
      ];
    }
    case "report_tree": {
      return [buildWorkItemTreeReport(context, action.target?.productName)];
    }
    default:
      return ["No executable action."];
  }
}

async function executePlannerPlan(plan: PlannerPlan, context: ResolverContext): Promise<ExecutionResult> {
  const lines: string[] = [];
  const errors: string[] = [];
  for (const action of plan.actions) {
    try {
      const resultLines = await executePlannerAction(action, context);
      lines.push(...resultLines);
    } catch (error) {
      errors.push(error instanceof Error ? error.message : String(error));
    }
  }
  return { lines, errors };
}

export function PlannerPage() {
  const queryClient = useQueryClient();
  const { activeProductId, activeModuleId, activeCapabilityId, activeWorkItemId } = useWorkspaceStore();
  const [plannerView, setPlannerView] = useState<"conversation" | "draft" | "trace">("conversation");
  const [providerId, setProviderId] = useState("");
  const [modelName, setModelName] = useState("");
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [draft, setDraft] = useState("");
  const [messages, setMessages] = useState<PlannerMessage[]>([
    { id: makeId(), role: "assistant", content: DEFAULT_ASSISTANT_OPENING },
  ]);
  const [pendingPlan, setPendingPlan] = useState<PendingPlan | null>(null);
  const [draftTreeNodes, setDraftTreeNodes] = useState<PlannerTreeNode[]>([]);
  const [selectedDraftNodeId, setSelectedDraftNodeId] = useState<string | null>(null);
  const [expandedDraftNodeIds, setExpandedDraftNodeIds] = useState<string[]>([]);
  const [latestTraceEvents, setLatestTraceEvents] = useState<PlannerTraceEvent[]>([]);
  const [renameDraftName, setRenameDraftName] = useState("");
  const [draftChildType, setDraftChildType] = useState<PlannerDraftChildType>("module");
  const [draftChildName, setDraftChildName] = useState("");
  const [draftChildSummary, setDraftChildSummary] = useState("");
  const [draftEditError, setDraftEditError] = useState<string | null>(null);
  const [draftEditMessage, setDraftEditMessage] = useState<string | null>(null);
  const [selectedRepositoryId, setSelectedRepositoryId] = useState("");
  const [repositoryPathDraft, setRepositoryPathDraft] = useState("");
  const [repoAnalysisMessage, setRepoAnalysisMessage] = useState<string | null>(null);
  const [repoAnalysisError, setRepoAnalysisError] = useState<string | null>(null);
  const [voiceEnabled, setVoiceEnabled] = useState(true);
  const [autoSpeak, setAutoSpeak] = useState(false);
  const [isListening, setIsListening] = useState(false);
  const [isTranscribing, setIsTranscribing] = useState(false);
  const [isVoiceSubmitting, setIsVoiceSubmitting] = useState(false);
  const [pendingVoiceTranscript, setPendingVoiceTranscript] = useState<string | null>(null);
  const [editableVoiceTranscript, setEditableVoiceTranscript] = useState("");
  const [voiceActivity, setVoiceActivity] = useState<string | null>(null);
  const [voiceCaptureStartedAt, setVoiceCaptureStartedAt] = useState<number | null>(null);
  const [voiceElapsedMs, setVoiceElapsedMs] = useState<number>(0);
  const [speechError, setSpeechError] = useState<string | null>(null);
  const [speechProviderSetting, setSpeechProviderSetting] = useState("");
  const [speechModelSetting, setSpeechModelSetting] = useState("");
  const [speechLocaleSetting, setSpeechLocaleSetting] = useState("en-US");
  const [speechNativeVoiceSetting, setSpeechNativeVoiceSetting] = useState("");
  const [showAdvancedPlannerControls, setShowAdvancedPlannerControls] = useState(false);
  const [contactTarget, setContactTarget] = useState("");
  const [contactDraft, setContactDraft] = useState("Call me and ask what work should be prioritized next.");
  const [contactMsg, setContactMsg] = useState<string | null>(null);
  const [contactError, setContactError] = useState<string | null>(null);
  const [windowWidth, setWindowWidth] = useState<number>(() => (typeof window === "undefined" ? 1440 : window.innerWidth));
  const [showCompactTools, setShowCompactTools] = useState(false);
  const audioCaptureRef = useRef<ActiveAudioCapture | null>(null);
  const mediaStreamRef = useRef<MediaStream | null>(null);
  const transcriptRef = useRef<HTMLDivElement | null>(null);
  const composerRef = useRef<HTMLTextAreaElement | null>(null);

  const { data: products = [] } = useQuery({ queryKey: ["products"], queryFn: listProducts });
  const { data: providers = [] } = useQuery({ queryKey: ["plannerProviders"], queryFn: listProviders });
  const { data: models = [] } = useQuery({ queryKey: ["plannerModels"], queryFn: listModelDefinitions });
  const { data: workItems = [] } = useQuery({ queryKey: ["plannerWorkItems"], queryFn: () => listWorkItems() });
  const { data: repositories = [] } = useQuery({ queryKey: ["plannerRepositories"], queryFn: listRepositories });

  const treeQueries = useQueries({
    queries: products.map((product) => ({
      queryKey: ["plannerProductTree", product.id],
      queryFn: () => getProductTree(product.id),
      enabled: !!product.id,
    })),
  });

  const productTrees = useMemo(
    () => treeQueries.map((query) => query.data).filter((value): value is ProductTree => Boolean(value)),
    [treeQueries],
  );
  const hasTreeData = productTrees.length > 0;
  const isFocusedWorkspaceView = plannerView === "draft" || plannerView === "trace";
  const isCompactScreen = windowWidth <= 1360;
  const selectedDraftNode = useMemo(
    () => findTreeNodeById(draftTreeNodes, selectedDraftNodeId),
    [draftTreeNodes, selectedDraftNodeId],
  );
  const selectedDraftNodePath = useMemo(
    () => findTreeNodePath(draftTreeNodes, selectedDraftNodeId),
    [draftTreeNodes, selectedDraftNodeId],
  );
  const expandedDraftNodeIdSet = useMemo(
    () => new Set(expandedDraftNodeIds),
    [expandedDraftNodeIds],
  );
  const latestDraftPlan = useMemo(() => {
    if (pendingPlan?.plan) {
      return pendingPlan.plan;
    }
    for (let index = messages.length - 1; index >= 0; index -= 1) {
      const entry = messages[index];
      if (entry.role === "assistant" && entry.plan && entry.plan.actions.length > 0) {
        return entry.plan;
      }
    }
    return null;
  }, [messages, pendingPlan]);
  const selectedDraftNodePrompts = useMemo(
    () => buildSuggestedPrompts(selectedDraftNode),
    [selectedDraftNode],
  );
  const allowedDraftChildTypes = useMemo(
    () => getAllowedDraftChildTypes(selectedDraftNode),
    [selectedDraftNode],
  );
  const draftValidation = useMemo(
    () => buildDraftValidation(draftTreeNodes),
    [draftTreeNodes],
  );
  const selectedNodeRecentActions = useMemo(
    () => findRelevantPlanActions(latestDraftPlan, selectedDraftNode),
    [latestDraftPlan, selectedDraftNode],
  );
  const latestAssistantMessage = useMemo(() => {
    for (let index = messages.length - 1; index >= 0; index -= 1) {
      if (messages[index].role === "assistant") {
        return messages[index];
      }
    }
    return null;
  }, [messages]);
  const plannerStatusSummary = useMemo(() => {
    if (voiceActivity) {
      return {
        title: voiceActivity,
        detail: pendingVoiceTranscript
          ? "The transcript is ready for review before it becomes a planner turn."
          : "Voice capture is in progress.",
      };
    }
    if (pendingVoiceTranscript) {
      return {
        title: "Voice transcript ready",
        detail: "Review or edit the transcript, then send it to the planner.",
      };
    }
    if (draftTreeNodes.length > 0) {
      return {
        title: `Draft active: ${draftValidation.counts.product} product, ${draftValidation.counts.module} module, ${draftValidation.counts.capability} capability, ${draftValidation.counts["work item"]} work item`,
        detail: selectedDraftNode
          ? `Selected node: ${selectedDraftNode.label}.`
          : "Select a node and keep refining before commit.",
      };
    }
    if (pendingPlan) {
      return {
        title: "Proposal waiting for confirmation",
        detail: `${pendingPlan.plan.actions.length} proposed changes are ready for review.`,
      };
    }
    if (latestAssistantMessage) {
      return {
        title: latestAssistantMessage.meta ?? "Planner ready",
        detail: latestAssistantMessage.content.split("\n")[0] || "Describe the product or outcome you want.",
      };
    }
    return {
      title: "Planner ready",
      detail: "Describe the product or outcome you want to stage.",
    };
  }, [
    draftTreeNodes.length,
    draftValidation.counts,
    latestAssistantMessage,
    pendingPlan,
    pendingVoiceTranscript,
    selectedDraftNode,
    voiceActivity,
  ]);

  const modelOptions = useMemo(
    () => models.filter((model) => model.provider_id === providerId && model.enabled),
    [models, providerId],
  );
  const plannerModelPickerOptions = useMemo(
    () =>
      models
        .filter((model) => model.enabled)
        .map((model) => {
          const provider = providers.find((entry) => entry.id === model.provider_id);
          return {
            value: `${model.provider_id}::${model.name}`,
            label: `${provider?.name ?? "Unknown Provider"} / ${model.name}`,
          };
        }),
    [models, providers],
  );
  const plannerModelPickerValue = providerId && modelName ? `${providerId}::${modelName}` : "";
  const speechModelSelection = useMemo(() => {
    const looksLikeSpeechModel = (model: ModelDefinition) =>
      model.capability_tags.some((tag) => ["speech_to_text", "transcription", "audio"].includes(tag))
      || /whisper|transcrib/i.test(model.name);

    if (speechProviderSetting || speechModelSetting) {
      if (speechProviderSetting && speechModelSetting) {
        return { providerId: speechProviderSetting, modelName: speechModelSetting, source: "settings" as const };
      }
      if (speechProviderSetting) {
        const providerSpeechModel = models.find((model) => model.enabled && model.provider_id === speechProviderSetting && looksLikeSpeechModel(model));
        return {
          providerId: speechProviderSetting,
          modelName: providerSpeechModel?.name ?? speechModelSetting ?? "whisper-1",
          source: "settings" as const,
        };
      }
      const namedSpeechModel = models.find((model) => model.enabled && model.name === speechModelSetting);
      if (namedSpeechModel) {
        return { providerId: namedSpeechModel.provider_id, modelName: namedSpeechModel.name, source: "settings" as const };
      }
    }

    const sameProvider = models.find((model) => model.enabled && model.provider_id === providerId && looksLikeSpeechModel(model));
    if (sameProvider) {
      return { providerId: sameProvider.provider_id, modelName: sameProvider.name, source: "planner" as const };
    }

    const anySpeechModel = models.find((model) => model.enabled && looksLikeSpeechModel(model));
    if (anySpeechModel) {
      return { providerId: anySpeechModel.provider_id, modelName: anySpeechModel.name, source: "auto" as const };
    }

    if (providerId) {
      return { providerId, modelName: "whisper-1", source: "fallback" as const };
    }
    return null;
  }, [models, providerId, speechModelSetting, speechProviderSetting]);

  const context = useMemo<ResolverContext>(() => ({
    products,
    productTrees,
    workItems,
    activeProductId,
    activeModuleId,
    activeCapabilityId,
    activeWorkItemId,
  }), [activeCapabilityId, activeModuleId, activeProductId, activeWorkItemId, productTrees, products, workItems]);

  useEffect(() => {
    if (typeof window === "undefined") {
      return undefined;
    }
    const onResize = () => setWindowWidth(window.innerWidth);
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  }, []);

  useEffect(() => {
    if (!providerId && providers.length > 0) {
      setProviderId(providers[0].id);
    }
  }, [providerId, providers]);

  useEffect(() => {
    if (!selectedRepositoryId && repositories.length > 0) {
      setSelectedRepositoryId(repositories[0].id);
    }
  }, [repositories, selectedRepositoryId]);

  useEffect(() => {
    let cancelled = false;
    void Promise.all([
      getSetting(SPEECH_PROVIDER_KEY),
      getSetting(SPEECH_MODEL_KEY),
      getSetting(SPEECH_LOCALE_KEY),
      getSetting(SPEECH_NATIVE_VOICE_KEY),
      getSetting(SPEECH_ENABLE_MIC_KEY),
      getSetting(SPEECH_AUTO_SPEAK_REPLIES_KEY),
    ]).then(([providerSetting, modelSetting, localeSetting, nativeVoiceSetting, micEnabledSetting, autoSpeakSetting]) => {
      if (cancelled) {
        return;
      }
      if (providerSetting) {
        setSpeechProviderSetting(providerSetting);
      }
      if (modelSetting) {
        setSpeechModelSetting(modelSetting);
      }
      if (localeSetting) {
        setSpeechLocaleSetting(localeSetting);
      }
      if (nativeVoiceSetting) {
        setSpeechNativeVoiceSetting(nativeVoiceSetting);
      }
      if (typeof micEnabledSetting === "string") {
        setVoiceEnabled(micEnabledSetting.trim().toLowerCase() !== "false");
      }
      if (typeof autoSpeakSetting === "string") {
        setAutoSpeak(autoSpeakSetting.trim().toLowerCase() === "true");
      }
    }).catch(() => {});
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (typeof window === "undefined" || !window.__ARUVI_E2E__) {
      return;
    }
    window.__ARUVI_E2E__.runPlannerVoiceTranscript = async (transcript: string) => {
      const handled = await handleVoiceTranscript(transcript);
      if (!handled) {
        setDraft((current) => (current ? `${current.trim()} ${transcript.trim()}` : transcript.trim()));
      }
    };
    return () => {
      if (window.__ARUVI_E2E__) {
        delete window.__ARUVI_E2E__.runPlannerVoiceTranscript;
      }
    };
  }, [draftTreeNodes, handleVoiceTranscript, selectedDraftNodeId, latestTraceEvents, pendingPlan, autoSpeak]);

  useEffect(() => {
    let cancelled = false;
    const ensureSession = async () => {
      if (sessionId) {
        return;
      }
      try {
        const session = await createPlannerSession({
          providerId: providerId || undefined,
          modelName: modelName || undefined,
        });
        if (!cancelled) {
          setSessionId(session.session_id);
        }
      } catch (error) {
        if (!cancelled) {
          setMessages((current) => [
            ...current,
            { id: makeId(), role: "assistant", content: String(error), meta: "Planner error", kind: "error" },
          ]);
        }
      }
    };
    void ensureSession();
    return () => {
      cancelled = true;
    };
  }, [modelName, providerId, sessionId]);

  useEffect(() => {
    if (!providerId) {
      return;
    }
    if (!modelName || !modelOptions.some((entry) => entry.name === modelName)) {
      setModelName(modelOptions[0]?.name ?? "");
    }
  }, [modelName, modelOptions, providerId]);

  useEffect(() => {
    if (!sessionId) {
      return;
    }
    void updatePlannerSession({
      sessionId,
      providerId: providerId || undefined,
      modelName: modelName || undefined,
    });
  }, [modelName, providerId, sessionId]);

  useEffect(() => {
    transcriptRef.current?.scrollTo({ top: transcriptRef.current.scrollHeight });
  }, [messages]);

  useEffect(() => {
    if (draftTreeNodes.length === 0 && plannerView === "draft") {
      setPlannerView("conversation");
    }
  }, [draftTreeNodes.length, plannerView]);

  useEffect(() => {
    if (!isCompactScreen) {
      setShowCompactTools(false);
    }
  }, [isCompactScreen]);

  useEffect(() => {
    const allNodeIds = collectTreeNodeIds(draftTreeNodes);
    if (allNodeIds.length === 0) {
      setExpandedDraftNodeIds([]);
      return;
    }
    setExpandedDraftNodeIds((current) => {
      const currentSet = new Set(current.filter((nodeId) => allNodeIds.includes(nodeId)));
      if (currentSet.size === 0) {
        return allNodeIds;
      }
      if (selectedDraftNodeId && !currentSet.has(selectedDraftNodeId)) {
        currentSet.add(selectedDraftNodeId);
      }
      return Array.from(currentSet);
    });
  }, [draftTreeNodes, selectedDraftNodeId]);

  useEffect(() => {
    setRenameDraftName(selectedDraftNode?.label ?? "");
    setDraftEditError(null);
    setDraftEditMessage(null);
  }, [selectedDraftNodeId, selectedDraftNode?.label]);

  useEffect(() => {
    if (allowedDraftChildTypes.length === 0) {
      return;
    }
    if (!allowedDraftChildTypes.includes(draftChildType)) {
      setDraftChildType(allowedDraftChildTypes[0]);
    }
  }, [allowedDraftChildTypes, draftChildType]);

  useEffect(() => {
    if (!voiceEnabled) {
      void stopVoiceCapture(false);
      setIsListening(false);
      return;
    }
    return () => {
      void stopVoiceCapture(false);
    };
  }, [voiceEnabled]);

  const speakAssistantReply = async (text: string) => {
    const trimmed = text.trim();
    if (!trimmed) {
      return;
    }
    try {
      await speakTextNatively({
        text: trimmed,
        voice: speechNativeVoiceSetting || undefined,
        locale: speechLocaleSetting || "en-US",
      });
    } catch {
      speakInBrowser(trimmed);
    }
  };

  const mapPlannerResponseToMutationResult = (
    response: PlannerTurnResponse,
    userInput: string,
  ): PlannerMutationResult => {
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
  };

  const handlePlannerMutationSuccess = (result: PlannerMutationResult) => {
    setPendingVoiceTranscript(null);
    setEditableVoiceTranscript("");
    setVoiceActivity(null);
    setIsVoiceSubmitting(false);
    setLatestTraceEvents(result.traceEvents ?? []);
    setMessages((current) => {
      const next: PlannerMessage[] = [...current, { id: makeId(), role: "user", content: result.userInput, kind: "text" }];
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
        const output = [
          result.plan.assistant_response,
          ...(result.execution?.lines ?? []),
        ].join("\n");
        next.push({
          id: makeId(),
          role: "assistant",
          content: output,
          meta: "Draft updated",
          kind: "proposal",
          plan: result.plan,
          treeNodes: result.treeNodes,
          traceEvents: result.traceEvents,
        });
        return next;
      }
      if (result.mode === "confirmed") {
        const output = [
          "Executed pending plan.",
          ...(result.execution?.lines ?? []),
          ...(result.execution?.errors.length ? [`Errors: ${result.execution.errors.join(" | ")}`] : []),
        ].join("\n");
        next.push({
          id: makeId(),
          role: "assistant",
          content: output,
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
        const output = [
          result.plan.assistant_response,
          ...(result.execution?.lines ?? []),
          ...(result.execution?.errors.length ? [`Errors: ${result.execution.errors.join(" | ")}`] : []),
        ].join("\n");
        next.push({
          id: makeId(),
          role: "assistant",
          content: output,
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
          content: [result.plan.assistant_response, ...(result.execution.errors.length ? [`Errors: ${result.execution.errors.join(" | ")}`] : [])].join("\n"),
          meta: "Planner error",
          kind: "error",
          traceEvents: result.traceEvents,
        });
        return next;
      }
      const output = [
        result.plan.assistant_response,
        ...(result.execution?.lines ?? []),
        ...(result.execution?.errors.length ? [`Errors: ${result.execution.errors.join(" | ")}`] : []),
      ].join("\n");
      next.push({
        id: makeId(),
        role: "assistant",
        content: output,
        meta: isInformationalOnly(result.plan) ? "Status report" : "Planner execution",
        kind: result.treeNodes ? "tree" : isInformationalOnly(result.plan) ? "report" : "execution",
        treeNodes: result.treeNodes,
        plan: result.plan,
        traceEvents: result.traceEvents,
      });
      return next;
    });

    if (result.draftTreeNodes) {
      setDraftTreeNodes(result.draftTreeNodes);
      if (result.draftTreeNodes.length > 0) {
        setPlannerView("draft");
      }
    }
    if (result.selectedDraftNodeId !== undefined) {
      setSelectedDraftNodeId(result.selectedDraftNodeId ?? null);
      const treeForPath = result.draftTreeNodes ?? draftTreeNodes;
      if (result.selectedDraftNodeId && treeForPath.length > 0) {
        const pathIds = findTreeNodePath(treeForPath, result.selectedDraftNodeId).map((node) => node.id);
        setExpandedDraftNodeIds((current) => Array.from(new Set([...current, ...pathIds])));
      }
    }

    if (result.mode === "confirmation_required") {
      setPendingPlan({ sourceText: result.userInput, plan: result.plan });
    } else if (result.mode === "draft_updated") {
      setPendingPlan(null);
    } else if (result.mode === "session_updated") {
      // Preserve the currently staged plan while updating draft selection or voice-driven session state.
    } else if (result.mode === "failed") {
      setPendingPlan(null);
      setPlannerView("trace");
    } else {
      setPendingPlan(null);
      if (result.mode === "executed" && !result.draftTreeNodes?.length) {
        setDraftTreeNodes([]);
        setSelectedDraftNodeId(null);
      }
      void queryClient.invalidateQueries({ queryKey: ["products"] });
      void queryClient.invalidateQueries({ queryKey: ["plannerWorkItems"] });
      void queryClient.invalidateQueries({ queryKey: ["sidebarWorkItems"] });
      void queryClient.invalidateQueries({ queryKey: ["productTree"] });
      void queryClient.invalidateQueries({ queryKey: ["plannerProductTree"] });
    }

    if (autoSpeak) {
      const lastAssistant = result.mode === "clarification"
        ? result.plan.clarification_question ?? result.plan.assistant_response
        : result.mode === "confirmation_required"
          ? `${result.plan.assistant_response}. Say confirm to apply the proposal.`
          : result.mode === "draft_updated"
            ? `${result.plan.assistant_response}. The draft tree has been updated.`
            : result.mode === "session_updated"
              ? result.plan.assistant_response
            : result.mode === "confirmed"
              ? "Executed the pending planner actions."
              : result.plan.assistant_response;
      void speakAssistantReply(lastAssistant);
    }
  };

  const processMutation = useMutation<PlannerMutationResult, Error, string>({
    mutationFn: async (input: string) => {
      const userInput = input.trim();
      let activeSessionId = sessionId;
      if (!activeSessionId) {
        const session = await createPlannerSession({
          providerId: providerId || undefined,
          modelName: modelName || undefined,
        });
        activeSessionId = session.session_id;
        setSessionId(session.session_id);
      }

      const response = await submitPlannerTurn({
        sessionId: activeSessionId,
        userInput,
        selectedDraftNodeId,
      });

      return mapPlannerResponseToMutationResult(response, userInput);
    },
    onSuccess: handlePlannerMutationSuccess,
    onError: (error, userInput) => {
      setPendingVoiceTranscript(null);
      setEditableVoiceTranscript("");
      setVoiceActivity(null);
      setIsVoiceSubmitting(false);
      setLatestTraceEvents([]);
      setMessages((current) => [
        ...current,
        { id: makeId(), role: "user", content: userInput, kind: "text" },
        { id: makeId(), role: "assistant", content: error instanceof Error ? error.message : String(error), meta: "Planner error", kind: "error" },
      ]);
    },
  });

  const draftEditMutation = useMutation<PlannerMutationResult, Error, DraftEditOperation>({
    mutationFn: async (operation) => {
      if (!sessionId) {
        throw new Error("Planner session is not ready.");
      }
      switch (operation.kind) {
        case "rename": {
          const response = await renamePlannerDraftNode({
            sessionId,
            nodeId: operation.nodeId,
            name: operation.name,
          });
          return mapPlannerResponseToMutationResult(
            response,
            `Rename this node to "${operation.name}".`,
          );
        }
        case "add_child": {
          const response = await addPlannerDraftChild({
            sessionId,
            parentNodeId: operation.parentNodeId,
            childType: operation.childType,
            name: operation.name,
            summary: operation.summary,
          });
          return mapPlannerResponseToMutationResult(
            response,
            `Add a ${formatDraftChildTypeLabel(operation.childType).toLowerCase()} called "${operation.name}".`,
          );
        }
        case "delete": {
          const response = await deletePlannerDraftNode({
            sessionId,
            nodeId: operation.nodeId,
          });
          return mapPlannerResponseToMutationResult(
            response,
            "Delete this node from the draft.",
          );
        }
      }
    },
    onSuccess: handlePlannerMutationSuccess,
    onError: (error) => {
      setDraftEditError(error instanceof Error ? error.message : String(error));
      setDraftEditMessage(null);
    },
  });

  const repositoryAnalysisMutation = useMutation<PlannerMutationResult, Error, string>({
    mutationFn: async (repositoryId: string) => {
      if (!sessionId) {
        throw new Error("Planner session is not ready.");
      }
      const response = await analyzeRepositoryForPlanner({
        sessionId,
        repositoryId,
        selectedDraftNodeId,
      });
      return mapPlannerResponseToMutationResult(
        response,
        `Analyze repository ${repositoryId} into a draft plan.`,
      );
    },
    onSuccess: handlePlannerMutationSuccess,
    onError: (error) => {
      setRepoAnalysisError(error instanceof Error ? error.message : String(error));
      setRepoAnalysisMessage(null);
    },
  });

  const transcribeAudioMutation = useMutation<string, Error, { audioBytesBase64: string; mimeType: string }>({
    mutationFn: async ({ audioBytesBase64, mimeType }) => {
      if (!speechModelSelection) {
        throw new Error("Configure a speech transcription provider or model before using voice input.");
      }
      const response = await transcribeAudio({
        providerId: speechModelSelection.providerId,
        modelName: speechModelSelection.modelName,
        audioBytesBase64,
        mimeType,
        locale: speechLocaleSetting || "en-US",
      });
      return response.transcript;
    },
    onError: (error) => {
      setSpeechError(error instanceof Error ? error.message : String(error));
    },
  });

  const isPlannerBusy =
    processMutation.isPending ||
    draftEditMutation.isPending ||
    repositoryAnalysisMutation.isPending ||
    transcribeAudioMutation.isPending ||
    isVoiceSubmitting;

  useEffect(() => {
    if (!isListening || !voiceCaptureStartedAt) {
      return undefined;
    }
    const interval = window.setInterval(() => {
      setVoiceElapsedMs(Date.now() - voiceCaptureStartedAt);
    }, 250);
    return () => window.clearInterval(interval);
  }, [isListening, voiceCaptureStartedAt]);

  const stopVoiceCapture = async (shouldTranscribe: boolean) => {
    const capture = audioCaptureRef.current;
    if (!capture) {
      return;
    }

    audioCaptureRef.current = null;
    mediaStreamRef.current = null;
    setIsListening(false);
    setVoiceCaptureStartedAt(null);

    try {
      const blob = await capture.stop();
      if (!shouldTranscribe || blob.size === 0) {
        setVoiceActivity(null);
        return;
      }

      setVoiceActivity("Transcribing audio...");
      setIsTranscribing(true);
      const audioBytesBase64 = await blobToBase64(blob);
      const transcript = await transcribeAudioMutation.mutateAsync({
        audioBytesBase64,
        mimeType: blob.type || "audio/wav",
      });
      const trimmedTranscript = transcript.trim();
      setIsTranscribing(false);
      if (!trimmedTranscript) {
        setVoiceActivity("No speech detected.");
        return;
      }
      setPendingVoiceTranscript(trimmedTranscript);
      setEditableVoiceTranscript(trimmedTranscript);
      setVoiceActivity("Speech recognized. Review or edit before sending.");
    } catch (error) {
      if (shouldTranscribe) {
        setSpeechError(error instanceof Error ? error.message : String(error));
      }
      setIsTranscribing(false);
      setIsVoiceSubmitting(false);
      setVoiceActivity(null);
      setPendingVoiceTranscript(null);
      setEditableVoiceTranscript("");
    } finally {
      setIsTranscribing(false);
    }
  };

  const send = async () => {
    const content = draft.trim();
    if (!content || isPlannerBusy) {
      return;
    }
    setDraft("");
    await processMutation.mutateAsync(content);
  };

  const clearPendingVoiceReview = () => {
    setPendingVoiceTranscript(null);
    setEditableVoiceTranscript("");
    setVoiceActivity(null);
    setVoiceElapsedMs(0);
  };

  const submitPendingVoiceTranscript = async () => {
    const transcript = editableVoiceTranscript.trim();
    if (!transcript || isPlannerBusy) {
      return;
    }
    setPendingVoiceTranscript(transcript);
    setVoiceActivity("Sending voice input to the planner...");
    setIsVoiceSubmitting(true);
    try {
      const handledAsVoiceCommand = await handleVoiceTranscript(transcript);
      if (!handledAsVoiceCommand) {
        setDraft((current) => (current ? `${current.trim()} ${transcript}` : transcript));
        composerRef.current?.focus();
        setVoiceActivity("Speech recognized and added to the composer.");
      }
    } catch (error) {
      setSpeechError(error instanceof Error ? error.message : String(error));
    } finally {
      setIsVoiceSubmitting(false);
      clearPendingVoiceReview();
    }
  };

  const retryVoiceCapture = async () => {
    clearPendingVoiceReview();
    await toggleListening();
  };

  const toggleListening = async () => {
    if (!voiceEnabled) {
      setSpeechError("Voice input is disabled.");
      return;
    }
    if (isListening) {
      await stopVoiceCapture(true);
      return;
    }

    if (typeof navigator === "undefined" || !navigator.mediaDevices?.getUserMedia) {
      setSpeechError("Microphone access is not available in this runtime.");
      return;
    }
    if (typeof window === "undefined" || (!window.AudioContext && !(window as typeof window & { webkitAudioContext?: typeof AudioContext }).webkitAudioContext)) {
      setSpeechError("PCM audio capture is not supported in this runtime.");
      return;
    }

    try {
      setSpeechError(null);
      setVoiceActivity("Listening...");
      setVoiceElapsedMs(0);
      setVoiceCaptureStartedAt(Date.now());
      setPendingVoiceTranscript(null);
      setEditableVoiceTranscript("");
      const capture = await startWavCapture();
      audioCaptureRef.current = capture;
      mediaStreamRef.current = capture.stream;
      setIsListening(true);
    } catch (error) {
      setSpeechError(error instanceof Error ? error.message : String(error));
      setIsListening(false);
      setVoiceActivity(null);
      setVoiceCaptureStartedAt(null);
      mediaStreamRef.current?.getTracks().forEach((track) => track.stop());
      mediaStreamRef.current = null;
      audioCaptureRef.current = null;
    }
  };

  const confirmPendingPlan = () => {
    if ((!pendingPlan && draftTreeNodes.length === 0) || isPlannerBusy || !sessionId) {
      return;
    }
    void (async () => {
      const response = await confirmPlannerPlan(sessionId);
      const execution: ExecutionResult = {
        lines: response.execution_lines,
        errors: response.execution_errors,
      };
      const plan = pendingPlan?.plan ?? {
        assistant_response: "Committed draft plan.",
        needs_confirmation: false,
        clarification_question: null,
        actions: [],
      };
      const treeNodes = (response.tree_nodes as unknown as PlannerTreeNode[] | undefined) ?? undefined;
      setLatestTraceEvents(response.trace_events ?? []);
      setMessages((current) => [
        ...current,
        { id: makeId(), role: "user", content: "confirm", kind: "text" },
        {
          id: makeId(),
          role: "assistant",
          content: ["Committed draft plan.", ...execution.lines, ...(execution.errors.length ? [`Errors: ${execution.errors.join(" | ")}`] : [])].join("\n"),
          meta: "Planner execution",
          kind: treeNodes ? "tree" : "execution",
          treeNodes,
          plan,
          traceEvents: response.trace_events ?? [],
        },
      ]);
      setPendingPlan(null);
      setDraftTreeNodes([]);
      setSelectedDraftNodeId(null);
      setPlannerView("conversation");
      void queryClient.invalidateQueries({ queryKey: ["products"] });
      void queryClient.invalidateQueries({ queryKey: ["plannerWorkItems"] });
      void queryClient.invalidateQueries({ queryKey: ["sidebarWorkItems"] });
      void queryClient.invalidateQueries({ queryKey: ["productTree"] });
      void queryClient.invalidateQueries({ queryKey: ["plannerProductTree"] });
    })().catch((error) => {
      setMessages((current) => [
        ...current,
        { id: makeId(), role: "assistant", content: String(error), meta: "Planner error", kind: "error" },
      ]);
    });
  };

  const dismissPendingPlan = () => {
    if (!pendingPlan && draftTreeNodes.length === 0) {
      return;
    }
    if (sessionId) {
      void clearPlannerPending(sessionId).catch(() => {});
    }
    setPendingPlan(null);
    setDraftTreeNodes([]);
    setSelectedDraftNodeId(null);
    setPlannerView("conversation");
  };

  const sendWhatsapp = async () => {
    try {
      setContactError(null);
      setContactMsg(null);
      await sendTwilioWhatsappMessage({ to: contactTarget.trim(), content: contactDraft.trim() });
      setContactMsg("WhatsApp message queued through Twilio.");
    } catch (error) {
      setContactError(String(error));
    }
  };

  const autoRouteContact = async () => {
    try {
      setContactError(null);
      setContactMsg(null);
      const result = await routePlannerContact({
        to: contactTarget.trim(),
        content: contactDraft.trim(),
      });
      const channelLabel = result.channel === "voice" ? "voice call" : "WhatsApp";
      if (result.status === "blocked") {
        setContactError(`Auto-routing blocked: ${result.reason}`);
        return;
      }
      setContactMsg(`Auto-routed to ${channelLabel}. ${result.reason}`);
    } catch (error) {
      setContactError(String(error));
    }
  };

  const startVoiceCall = async () => {
    try {
      setContactError(null);
      setContactMsg(null);
      await startTwilioVoiceCall({ to: contactTarget.trim(), initialPrompt: contactDraft.trim() || undefined });
      setContactMsg("Voice call requested through Twilio.");
    } catch (error) {
      setContactError(String(error));
    }
  };

  const browseRepositoryPathForPlanner = async () => {
    try {
      setRepoAnalysisError(null);
      const selectedPath = await browseForRepositoryPath();
      if (selectedPath) {
        setRepositoryPathDraft(selectedPath);
      }
    } catch (error) {
      setRepoAnalysisError(String(error));
    }
  };

  const registerRepositoryForPlanner = async () => {
    const localPath = repositoryPathDraft.trim();
    if (!localPath) {
      return;
    }
    try {
      setRepoAnalysisError(null);
      setRepoAnalysisMessage(null);
      const segments = localPath.split(/[\\/]/).filter(Boolean);
      const inferredName = segments[segments.length - 1] ?? "repository";
      const repository = await registerRepository({
        name: inferredName,
        localPath,
        remoteUrl: "",
        defaultBranch: "main",
      });
      setSelectedRepositoryId(repository.id);
      setRepositoryPathDraft("");
      setRepoAnalysisMessage(`Registered repository "${repository.name}".`);
      void queryClient.invalidateQueries({ queryKey: ["plannerRepositories"] });
    } catch (error) {
      setRepoAnalysisError(String(error));
    }
  };

  const analyzeSelectedRepository = async () => {
    if (!selectedRepositoryId || isPlannerBusy) {
      return;
    }
    try {
      setRepoAnalysisError(null);
      setRepoAnalysisMessage(null);
      await repositoryAnalysisMutation.mutateAsync(selectedRepositoryId);
      setRepoAnalysisMessage("Repository analysis staged a draft update.");
    } catch {
      // Error state is handled by the mutation.
    }
  };

  const toggleDraftNodeExpanded = (nodeId: string) => {
    setExpandedDraftNodeIds((current) =>
      current.includes(nodeId) ? current.filter((value) => value !== nodeId) : [...current, nodeId],
    );
  };

  const expandAllDraftNodes = () => {
    setExpandedDraftNodeIds(collectTreeNodeIds(draftTreeNodes));
  };

  const collapseAllDraftNodes = () => {
    setExpandedDraftNodeIds([]);
  };

  function appendVoiceCommandFeedback(transcript: string, reply: string) {
    setPendingVoiceTranscript(null);
    setEditableVoiceTranscript("");
    setVoiceActivity(null);
    setMessages((current) => [
      ...current,
      { id: makeId(), role: "user", content: transcript, kind: "text" },
      { id: makeId(), role: "assistant", content: reply, meta: "Voice command", kind: "text" },
    ]);
    if (autoSpeak) {
      void speakAssistantReply(reply);
    }
  }

  function parseVoiceNodeReference(
    spokenRemainder: string,
  ): { explicitType?: string; reference: string } {
    const trimmed = spokenRemainder.trim();
    const prefixes: Array<{ prefix: string; type: string }> = [
      { prefix: "work item ", type: "work item" },
      { prefix: "work-item ", type: "work item" },
      { prefix: "capability ", type: "capability" },
      { prefix: "module ", type: "module" },
      { prefix: "product ", type: "product" },
      { prefix: "node ", type: "node" },
    ];
    for (const option of prefixes) {
      if (trimmed === option.prefix.trim()) {
        return { explicitType: option.type, reference: `selected ${option.type}` };
      }
      if (trimmed.startsWith(option.prefix)) {
        return { explicitType: option.type, reference: trimmed.slice(option.prefix.length).trim() };
      }
    }
    return { reference: trimmed };
  }

  async function handleVoiceTranscript(transcript: string) {
    const spoken = transcript.trim();
    if (!spoken) {
      return true;
    }
    const normalizedTranscript = normalize(spoken);

    if (["view draft", "open draft", "show draft", "show draft tree", "view draft tree", "open workspace", "show workspace"].includes(normalizedTranscript)) {
      if (draftTreeNodes.length === 0) {
        appendVoiceCommandFeedback(spoken, "There is no staged draft tree yet.");
      } else {
        setPlannerView("draft");
        appendVoiceCommandFeedback(spoken, "Opened the draft workspace.");
      }
      return true;
    }

    if (["view trace", "show trace", "open trace"].includes(normalizedTranscript)) {
      if (latestTraceEvents.length === 0) {
        appendVoiceCommandFeedback(spoken, "There is no planner trace available yet.");
      } else {
        setPlannerView("trace");
        appendVoiceCommandFeedback(spoken, "Opened the latest planner trace.");
      }
      return true;
    }

    if (["view conversation", "open conversation", "show conversation", "back to chat", "view chat"].includes(normalizedTranscript)) {
      setPlannerView("conversation");
      appendVoiceCommandFeedback(spoken, "Switched back to the planner conversation.");
      return true;
    }

    if (["expand draft", "expand the draft", "expand tree", "expand all", "open all branches"].includes(normalizedTranscript)) {
      setPlannerView("draft");
      expandAllDraftNodes();
      appendVoiceCommandFeedback(spoken, "Expanded the staged draft tree.");
      return true;
    }

    if (["collapse draft", "collapse the draft", "collapse tree", "collapse all"].includes(normalizedTranscript)) {
      collapseAllDraftNodes();
      appendVoiceCommandFeedback(spoken, "Collapsed the staged draft tree.");
      return true;
    }

    const collapseMatch = normalizedTranscript.match(/^(collapse|close)\s+(.+)$/);
    if (normalizedTranscript.startsWith("expand ") || normalizedTranscript.startsWith("open ")) {
      const targetText = spoken.replace(/^(expand|open)\s+/i, "").trim();
      if (["draft", "tree", "all"].includes(normalize(targetText))) {
        setPlannerView("draft");
        expandAllDraftNodes();
        appendVoiceCommandFeedback(spoken, "Expanded the staged draft tree.");
        return true;
      }
    }

    if (collapseMatch) {
      const targetText = collapseMatch[2];
      if (["draft", "tree", "all"].includes(normalize(targetText))) {
        collapseAllDraftNodes();
        appendVoiceCommandFeedback(spoken, "Collapsed the staged draft tree.");
        return true;
      }
      const { explicitType, reference } = parseVoiceNodeReference(targetText);
      const targetNode = resolveVoiceNodeReference(draftTreeNodes, selectedDraftNodePath, reference, explicitType);
      if (!targetNode) {
        appendVoiceCommandFeedback(spoken, `I could not find a draft node matching "${targetText}".`);
        return true;
      }
      setExpandedDraftNodeIds((current) => current.filter((nodeId) => nodeId !== targetNode.id));
      appendVoiceCommandFeedback(spoken, `Collapsed ${getPlannerNodeType(targetNode)} "${targetNode.label}".`);
      return true;
    }

    let activeSessionId = sessionId;
    if (!activeSessionId) {
      const session = await createPlannerSession({
        providerId: providerId || undefined,
        modelName: modelName || undefined,
      });
      activeSessionId = session.session_id;
      setSessionId(session.session_id);
    }

    const response = await submitPlannerVoiceTurn({
      sessionId: activeSessionId,
      transcript: spoken,
      selectedDraftNodeId,
    });
    handlePlannerMutationSuccess(mapPlannerResponseToMutationResult(response, spoken));
    return true;
  }

  const applyPromptSuggestion = (prompt: string) => {
    setDraft(prompt);
    composerRef.current?.focus();
  };

  const renameSelectedDraftNode = async () => {
    if (!selectedDraftNode || !renameDraftName.trim() || isPlannerBusy) {
      return;
    }
    setDraftEditError(null);
    setDraftEditMessage(null);
    try {
      await draftEditMutation.mutateAsync({
        kind: "rename",
        nodeId: selectedDraftNode.id,
        name: renameDraftName.trim(),
      });
      setDraftEditMessage(`Renamed to "${renameDraftName.trim()}".`);
    } catch {
      // Error state is handled by the mutation.
    }
  };

  const addChildToSelectedDraftNode = async () => {
    if (!selectedDraftNode || !draftChildName.trim() || allowedDraftChildTypes.length === 0 || isPlannerBusy) {
      return;
    }
    setDraftEditError(null);
    setDraftEditMessage(null);
    try {
      await draftEditMutation.mutateAsync({
        kind: "add_child",
        parentNodeId: selectedDraftNode.id,
        childType: draftChildType,
        name: draftChildName.trim(),
        summary: draftChildSummary.trim() || undefined,
      });
      setDraftChildName("");
      setDraftChildSummary("");
      setDraftEditMessage(`Added ${formatDraftChildTypeLabel(draftChildType).toLowerCase()} "${draftChildName.trim()}".`);
    } catch {
      // Error state is handled by the mutation.
    }
  };

  const deleteSelectedDraftNode = async () => {
    if (!selectedDraftNode || isPlannerBusy) {
      return;
    }
    setDraftEditError(null);
    setDraftEditMessage(null);
    const deletedLabel = selectedDraftNode.label;
    try {
      await draftEditMutation.mutateAsync({
        kind: "delete",
        nodeId: selectedDraftNode.id,
      });
      setDraftEditMessage(`Removed "${deletedLabel}" from the draft.`);
    } catch {
      // Error state is handled by the mutation.
    }
  };

  const renderAssistantMessage = (message: PlannerMessage) => {
    if (message.kind === "proposal" && message.plan) {
      const proposalTreeNodes = buildProposalTreeNodes(message.plan);
      return (
        <>
          <div>{message.content}</div>
          <div style={styles.card}>
            <div style={styles.cardTitle}>Proposed Changes</div>
            {message.plan.actions.map((action, index) => {
              const summary = summarizeAction(action);
              const symbolStyle = summary.tone === "add"
                ? styles.diffSymbolAdd
                : summary.tone === "update"
                  ? styles.diffSymbolUpdate
                  : styles.diffSymbolWarn;
              return (
                <div key={`${action.type}-${index}`} style={styles.diffRow}>
                  <div style={symbolStyle}>{summary.symbol}</div>
                  <div>
                    <div style={styles.diffPrimary}>{summary.title}</div>
                    {summary.detail ? <div style={styles.diffSecondary}>{summary.detail}</div> : null}
                  </div>
                </div>
              );
            })}
            {proposalTreeNodes.length > 0 ? (
              <div style={styles.cardSection}>
                <div style={styles.cardTitle}>Proposed Structure</div>
                <div style={styles.treePanel}>
                  {proposalTreeNodes.map((node) => (
                    <TreeNodeView key={node.id} node={node} />
                  ))}
                </div>
              </div>
            ) : null}
            {message.treeNodes && message.treeNodes.length > 0 ? (
              <div style={styles.cardSection}>
                <div style={styles.cardTitle}>Current Structure</div>
                <div style={styles.treePanel}>
                  {message.treeNodes.map((node) => (
                    <TreeNodeView key={node.id} node={node} />
                  ))}
                </div>
              </div>
            ) : null}
            <div style={styles.inlineButtonRow}>
              <button style={styles.btn} onClick={confirmPendingPlan} disabled={isPlannerBusy || (!pendingPlan && draftTreeNodes.length === 0)}>
                {draftTreeNodes.length > 0 ? "Commit Draft" : "Confirm Proposal"}
              </button>
              <button style={styles.btnGhost} onClick={dismissPendingPlan} disabled={!pendingPlan && draftTreeNodes.length === 0}>
                {draftTreeNodes.length > 0 ? "Clear Draft" : "Dismiss"}
              </button>
            </div>
          </div>
        </>
      );
    }

    if (message.kind === "tree" && message.treeNodes) {
      return (
        <>
          <div>{message.content}</div>
          <div style={styles.treePanel}>
            {message.treeNodes.map((node) => (
              <TreeNodeView key={node.id} node={node} />
            ))}
          </div>
        </>
      );
    }

    return <div>{message.content}</div>;
  };

  const renderPlannerSidebar = () => (
    <div style={styles.panel}>
      <div style={isCompactScreen ? styles.compactPanelBody : styles.panelBody}>
        <div style={styles.sectionTitle}>Planner Controls</div>
        <div style={styles.sideCard}>
          <div style={styles.helper}>
            {hasTreeData ? "Tree rendering is active for work-item structure questions." : "Tree rendering will activate once product structure finishes loading."}
          </div>
          <div style={styles.inlineButtonRow}>
            <button style={styles.btnGhost} onClick={() => setShowAdvancedPlannerControls((value) => !value)}>
              {showAdvancedPlannerControls ? "Hide Advanced Tools" : "Show Advanced Tools"}
            </button>
          </div>
        </div>

        {showAdvancedPlannerControls ? (
          <>
            <div style={styles.sideCard}>
              <div style={styles.label}>Reverse Engineer Repo</div>
              <div style={styles.helper}>
                Point the planner at an existing repository and let the model infer a staged product, module, capability, and work-item tree from the codebase.
              </div>
              <div style={{ height: 10 }} />
              <label style={styles.label}>Registered Repository</label>
              <select
                style={styles.select}
                value={selectedRepositoryId}
                onChange={(event) => setSelectedRepositoryId(event.target.value)}
              >
                <option value="">Select a repository</option>
                {repositories.map((repository) => (
                  <option key={repository.id} value={repository.id}>
                    {repository.name}
                  </option>
                ))}
              </select>
              <div style={{ height: 10 }} />
              <label style={styles.label}>Add Existing Repo Path</label>
              <input
                style={styles.input}
                value={repositoryPathDraft}
                onChange={(event) => setRepositoryPathDraft(event.target.value)}
                placeholder="/absolute/path/to/repository"
              />
              <div style={styles.inlineButtonRow}>
                <button style={styles.btnGhost} onClick={() => void browseRepositoryPathForPlanner()}>
                  Browse Path
                </button>
                <button
                  style={styles.btnGhost}
                  onClick={() => void registerRepositoryForPlanner()}
                  disabled={!repositoryPathDraft.trim()}
                >
                  Register Repo
                </button>
              </div>
              <div style={styles.inlineButtonRow}>
                <button
                  style={styles.btn}
                  onClick={() => void analyzeSelectedRepository()}
                  disabled={!selectedRepositoryId || isPlannerBusy || !providerId || !modelName}
                >
                  Analyze Repo Into Draft
                </button>
              </div>
              {!providerId || !modelName ? (
                <div style={{ ...styles.helper, marginTop: 10 }}>
                  Configure a planner model first. Repository reverse engineering depends on the selected LLM.
                </div>
              ) : null}
              {repoAnalysisMessage ? <div style={{ ...styles.success, marginTop: 10 }}>{repoAnalysisMessage}</div> : null}
              {repoAnalysisError ? <div style={{ ...styles.error, marginTop: 10 }}>{repoAnalysisError}</div> : null}
            </div>

            <div style={styles.sideCard}>
              <div style={styles.label}>Contact Me</div>
              <div style={styles.helper}>Use Auto Route to follow the planner channel policy: routine updates stay on WhatsApp, while ambiguous planning can escalate to a call. Manual buttons still override the policy.</div>
              <div style={{ height: 10 }} />
              <label style={styles.label}>Destination</label>
              <input
                style={styles.input}
                value={contactTarget}
                onChange={(event) => setContactTarget(event.target.value)}
                placeholder="whatsapp:+15551234567 or +15551234567"
              />
              <div style={{ height: 10 }} />
              <label style={styles.label}>Opening Message</label>
              <textarea
                style={{ ...styles.textarea, minHeight: 84 }}
                value={contactDraft}
                onChange={(event) => setContactDraft(event.target.value)}
                placeholder="Tell the planner what the outbound contact should say first."
              />
              <div style={styles.actionRow}>
                <button style={styles.btn} onClick={() => void autoRouteContact()} disabled={!contactTarget.trim() || !contactDraft.trim()}>
                  Auto Route
                </button>
                <button style={styles.btnGhost} onClick={() => void sendWhatsapp()} disabled={!contactTarget.trim()}>
                  Send WhatsApp
                </button>
                <button style={styles.btnGhost} onClick={() => void startVoiceCall()} disabled={!contactTarget.trim()}>
                  Start Voice Call
                </button>
              </div>
              {contactMsg ? <div style={{ ...styles.success, marginTop: 10 }}>{contactMsg}</div> : null}
              {contactError ? <div style={{ ...styles.error, marginTop: 10 }}>{contactError}</div> : null}
            </div>
          </>
        ) : null}

        <div style={styles.sideCard}>
          <div style={styles.label}>Draft Tree</div>
          <div style={styles.helper}>
            Build the plan here first. Select a node, then ask follow-up questions like “expand this capability” or “add work items under this module.”
          </div>
          <div style={{ height: 10 }} />
          {draftTreeNodes.length > 0 ? (
            <div style={styles.treePanel}>
              <div style={styles.treeExplorer}>
                {draftTreeNodes.map((node) => (
                  <SelectableTreeNodeView
                    key={node.id}
                    node={node}
                    selectedNodeId={selectedDraftNodeId}
                    onSelect={setSelectedDraftNodeId}
                    expandedNodeIds={expandedDraftNodeIdSet}
                    onToggle={toggleDraftNodeExpanded}
                  />
                ))}
              </div>
            </div>
          ) : (
            <div style={styles.helper}>No staged draft yet. Ask the planner to sketch a product tree and it will appear here.</div>
          )}
          <div style={styles.inlineButtonRow}>
            <button style={styles.btnGhost} onClick={() => setPlannerView("draft")} disabled={draftTreeNodes.length === 0}>
              Open Workspace
            </button>
            <button style={styles.btn} onClick={confirmPendingPlan} disabled={draftTreeNodes.length === 0 || isPlannerBusy}>
              Commit Draft
            </button>
            <button style={styles.btnGhost} onClick={dismissPendingPlan} disabled={draftTreeNodes.length === 0}>
              Clear Draft
            </button>
          </div>
        </div>

        <div style={styles.sideCard}>
          <div style={styles.label}>Current Scope</div>
          <div style={styles.chipRow}>
            {selectedDraftNodeId ? <div style={styles.chip}>draft node selected</div> : null}
            {activeProductId ? <div style={styles.chip}>product selected</div> : null}
            {activeModuleId ? <div style={styles.chip}>module selected</div> : null}
            {activeCapabilityId ? <div style={styles.chip}>capability selected</div> : null}
            {activeWorkItemId ? <div style={styles.chip}>work item selected</div> : null}
          </div>
          <div style={{ ...styles.helper, marginTop: 10 }}>
            If you omit names, the planner first tries the selected draft node, then the selected workspace scope, then asks follow-up questions if it still cannot resolve the target cleanly.
          </div>
        </div>

        {pendingPlan || draftTreeNodes.length > 0 ? (
          <div style={styles.sideCard}>
            <div style={styles.label}>Draft Snapshot</div>
            <div style={styles.helper}>
              The planner stages structure here first. Keep refining the tree, then commit when the draft looks right.
            </div>
            {pendingPlan ? (
              <div style={styles.list}>
                {pendingPlan.plan.actions.map((action, index) => (
                  <div key={`${action.type}-${index}`} style={styles.listItem}>
                    <div style={styles.listItemTitle}>{action.type}</div>
                    <div style={styles.listItemMeta}>{JSON.stringify(action, null, 2)}</div>
                  </div>
                ))}
              </div>
            ) : (
              <div style={{ ...styles.helper, marginTop: 10 }}>
                The current staged draft is active in the tree above. Select a node and keep iterating, or commit to persist it.
              </div>
            )}
          </div>
        ) : null}
      </div>
    </div>
  );

  return (
    <div style={styles.page}>
      <div
        style={
          isCompactScreen
            ? styles.compactStack
            : {
                ...styles.topGrid,
                gridTemplateColumns: isFocusedWorkspaceView ? "minmax(0, 1fr)" : styles.topGrid.gridTemplateColumns,
              }
        }
      >
        <div style={styles.panel}>
          <div style={{ ...(isCompactScreen ? styles.compactPanelBody : styles.panelBody), display: "flex", flexDirection: "column" }}>
            <div style={styles.sectionHeader}>
              <div style={styles.sectionTitle}>
                {plannerView === "draft" ? "Draft Workspace" : plannerView === "trace" ? "Planner Trace" : "Conversation"}
              </div>
              <div style={styles.viewToggleRow}>
                <select
                  aria-label="Planner model"
                  style={{ ...styles.select, width: 260 }}
                  value={plannerModelPickerValue}
                  onChange={(event) => {
                    const [nextProviderId, nextModelName] = event.target.value.split("::");
                    setProviderId(nextProviderId ?? "");
                    setModelName(nextModelName ?? "");
                  }}
                >
                  <option value="">Select model</option>
                  {plannerModelPickerOptions.map((option) => (
                    <option key={option.value} value={option.value}>
                      {option.label}
                    </option>
                  ))}
                </select>
                <button
                  data-testid="planner-view-conversation"
                  style={plannerView === "conversation" ? styles.btn : styles.btnGhost}
                  onClick={() => setPlannerView("conversation")}
                >
                  Conversation
                </button>
                <button
                  data-testid="planner-view-draft"
                  style={plannerView === "draft" ? styles.btn : styles.btnGhost}
                  onClick={() => setPlannerView("draft")}
                  disabled={draftTreeNodes.length === 0}
                >
                  View Draft Tree
                </button>
                <button
                  data-testid="planner-view-trace"
                  style={plannerView === "trace" ? styles.btn : styles.btnGhost}
                  onClick={() => setPlannerView("trace")}
                  disabled={latestTraceEvents.length === 0}
                >
                  View Trace
                </button>
              </div>
            </div>

            <div style={styles.statusBanner}>
              <div>
                <div style={styles.statusBannerStrong}>{plannerStatusSummary.title}</div>
                <div style={styles.statusBannerMeta}>{plannerStatusSummary.detail}</div>
              </div>
              <div style={styles.chipRow}>
                {providerId ? <div style={styles.chip}>{providers.find((provider) => provider.id === providerId)?.name ?? "provider selected"}</div> : null}
                {modelName ? <div style={styles.chip}>{modelName}</div> : null}
                {selectedDraftNode ? <div style={styles.chip}>selected: {selectedDraftNode.label}</div> : null}
              </div>
            </div>

            {isCompactScreen && plannerView === "conversation" ? (
              <>
                <div style={styles.compactControlStrip}>
                  <button style={styles.btnGhost} onClick={() => setShowCompactTools((value) => !value)}>
                    {showCompactTools ? "Hide Tools" : "Show Tools"}
                  </button>
                  <button style={styles.btnGhost} onClick={() => setPlannerView("draft")} disabled={draftTreeNodes.length === 0}>
                    Open Draft
                  </button>
                  <button style={styles.btnGhost} onClick={() => setPlannerView("trace")} disabled={latestTraceEvents.length === 0}>
                    Open Trace
                  </button>
                </div>
                <div style={styles.compactSummaryCard}>
                  <div style={styles.compactSummaryGrid}>
                    <div style={styles.compactSummaryItem}>
                      <div style={styles.compactSummaryLabel}>Draft</div>
                      <div style={styles.compactSummaryValue}>{draftTreeNodes.length > 0 ? `${draftValidation.counts.module} modules staged` : "No active draft"}</div>
                    </div>
                    <div style={styles.compactSummaryItem}>
                      <div style={styles.compactSummaryLabel}>Selection</div>
                      <div style={styles.compactSummaryValue}>{selectedDraftNode?.label ?? "None"}</div>
                    </div>
                    <div style={styles.compactSummaryItem}>
                      <div style={styles.compactSummaryLabel}>Readiness</div>
                      <div style={styles.compactSummaryValue}>{draftTreeNodes.length > 0 ? `${draftValidation.score}` : "n/a"}</div>
                    </div>
                    <div style={styles.compactSummaryItem}>
                      <div style={styles.compactSummaryLabel}>State</div>
                      <div style={styles.compactSummaryValue}>
                        {pendingVoiceTranscript ? "Review transcript" : isPlannerBusy ? "Working" : pendingPlan ? "Need confirm" : "Ready"}
                      </div>
                    </div>
                  </div>
                </div>
              </>
            ) : null}

            {plannerView === "draft" ? (
              <div style={styles.draftWorkspace}>
                <div style={styles.draftWorkspaceMain}>
                  <div style={styles.draftCanvas}>
                    <div style={styles.draftCanvasHeader}>
                      <div>
                        <div style={styles.draftCanvasTitle}>Staged Plan Tree</div>
                        <div style={styles.helper}>
                          Select a node, then refine it in natural language. The composer below will use the selected draft node as planning context.
                        </div>
                      </div>
                      <div style={styles.chipRow}>
                        {selectedDraftNode ? <div style={styles.chip}>selected: {selectedDraftNode.label}</div> : null}
                        <div style={styles.chip}>{draftTreeNodes.length} root {draftTreeNodes.length === 1 ? "node" : "nodes"}</div>
                      </div>
                    </div>
                    <div style={styles.readinessBanner}>
                      <div>
                        <div style={styles.label}>Commit Readiness</div>
                        <div style={styles.readinessMeta}>
                          {draftValidation.issues.filter((issue) => issue.tone === "warn").length === 0
                            ? "This draft is structurally solid enough to commit."
                            : "There are still weak spots in the staged tree. Fix them before commit if you want a cleaner catalog."}
                        </div>
                      </div>
                      <div style={styles.readinessScore}>{draftValidation.score}</div>
                    </div>
                    <div style={styles.metricGrid}>
                      <div style={styles.metricCard}>
                        <div style={styles.metricLabel}>Products</div>
                        <div style={styles.metricValue}>{draftValidation.counts.product}</div>
                      </div>
                      <div style={styles.metricCard}>
                        <div style={styles.metricLabel}>Modules</div>
                        <div style={styles.metricValue}>{draftValidation.counts.module}</div>
                      </div>
                      <div style={styles.metricCard}>
                        <div style={styles.metricLabel}>Capabilities</div>
                        <div style={styles.metricValue}>{draftValidation.counts.capability}</div>
                      </div>
                      <div style={styles.metricCard}>
                        <div style={styles.metricLabel}>Work Items</div>
                        <div style={styles.metricValue}>{draftValidation.counts["work item"]}</div>
                      </div>
                    </div>
                    <div style={styles.treeToolbar}>
                      <button data-testid="draft-expand-all" style={styles.btnGhost} onClick={expandAllDraftNodes} disabled={draftTreeNodes.length === 0}>
                        Expand All
                      </button>
                      <button data-testid="draft-collapse-all" style={styles.btnGhost} onClick={collapseAllDraftNodes} disabled={draftTreeNodes.length === 0}>
                        Collapse All
                      </button>
                      <div style={styles.treeToolbarSpacer} />
                      <div style={styles.helper}>
                        Select a node to scope prompts. Expand branches to inspect the staged structure.
                      </div>
                    </div>
                    {draftTreeNodes.length > 0 ? (
                      <div style={styles.treeExplorer}>
                        {draftTreeNodes.map((node) => (
                          <SelectableTreeNodeView
                            key={node.id}
                            node={node}
                            selectedNodeId={selectedDraftNodeId}
                            onSelect={setSelectedDraftNodeId}
                            expandedNodeIds={expandedDraftNodeIdSet}
                            onToggle={toggleDraftNodeExpanded}
                          />
                        ))}
                      </div>
                    ) : (
                      <div style={styles.emptyState}>
                        No staged draft yet. Ask the planner to design a product, module tree, capabilities, or work items, then switch back here to inspect and refine the draft.
                      </div>
                    )}
                  </div>
                  <PlannerComposer
                    draft={draft}
                    onDraftChange={setDraft}
                    onSend={() => {
                      void send();
                    }}
                    onToggleListening={() => {
                      void toggleListening();
                    }}
                    onConfirm={() => setDraft("confirm")}
                    onDismiss={dismissPendingPlan}
                    isPlannerBusy={isPlannerBusy}
                    voiceEnabled={voiceEnabled}
                    isListening={isListening}
                    isTranscribing={isTranscribing}
                    isVoiceSubmitting={isVoiceSubmitting}
                    pendingVoiceTranscript={pendingVoiceTranscript}
                    draftTreeNodesLength={draftTreeNodes.length}
                    pendingPlan={pendingPlan}
                    voiceActivity={voiceActivity}
                    composerRef={composerRef}
                  />
                </div>

                <div style={styles.draftWorkspaceSide}>
                  <div style={styles.sideCard}>
                    <div style={styles.label}>Selected Node</div>
                    {selectedDraftNode ? (
                      <>
                        <div style={styles.cardTitle}>{selectedDraftNode.label}</div>
                        <div style={styles.helper}>Type: {getPlannerNodeType(selectedDraftNode)}</div>
                        {selectedDraftNode.summary ? <div style={{ ...styles.helper, marginTop: 8 }}>{selectedDraftNode.summary}</div> : null}
                        {selectedDraftNode.source || selectedDraftNode.confidence ? (
                          <div style={styles.chipRow}>
                            {selectedDraftNode.source ? <div style={styles.chip}>source: {selectedDraftNode.source.replace("_", " ")}</div> : null}
                            {selectedDraftNode.confidence ? <div style={styles.chip}>{selectedDraftNode.confidence} confidence</div> : null}
                          </div>
                        ) : null}
                        {selectedDraftNodePath.length > 0 ? (
                          <div style={styles.pathText}>
                            Path: {selectedDraftNodePath.map((node) => node.label).join(" / ")}
                          </div>
                        ) : null}
                        {selectedDraftNode.evidence && selectedDraftNode.evidence.length > 0 ? (
                          <>
                            <div style={styles.sectionDivider} />
                            <div style={styles.label}>Evidence</div>
                            <div style={styles.list}>
                              {selectedDraftNode.evidence.map((line: string) => (
                                <div key={line} style={styles.listItem}>
                                  <div style={styles.listItemMeta}>{line}</div>
                                </div>
                              ))}
                            </div>
                          </>
                        ) : null}
                        <div style={styles.sectionDivider} />
                        <div style={styles.label}>Edit Node</div>
                        <div style={styles.fieldGroup}>
                          <input
                            data-testid="draft-node-rename-input"
                            style={styles.input}
                            value={renameDraftName}
                            onChange={(event) => setRenameDraftName(event.target.value)}
                            placeholder="Rename this node"
                          />
                          <div style={styles.inlineButtonRow}>
                            <button
                              data-testid="draft-node-rename-save"
                              style={styles.mutedButton}
                              onClick={() => {
                                void renameSelectedDraftNode();
                              }}
                              disabled={!renameDraftName.trim() || isPlannerBusy}
                            >
                              Save Name
                            </button>
                            <button
                              data-testid="draft-node-delete"
                              style={styles.btnDanger}
                              onClick={() => {
                                void deleteSelectedDraftNode();
                              }}
                              disabled={isPlannerBusy}
                            >
                              Delete Node
                            </button>
                          </div>
                        </div>
                        <div style={styles.sectionDivider} />
                        <div style={styles.label}>Add Child</div>
                        {allowedDraftChildTypes.length > 0 ? (
                          <div style={styles.fieldGroup}>
                            <select
                              data-testid="draft-node-add-child-type"
                              style={styles.select}
                              value={draftChildType}
                              onChange={(event) => setDraftChildType(event.target.value as PlannerDraftChildType)}
                            >
                              {allowedDraftChildTypes.map((option: PlannerDraftChildType) => (
                                <option key={option} value={option}>
                                  {formatDraftChildTypeLabel(option)}
                                </option>
                              ))}
                            </select>
                            <input
                              data-testid="draft-node-add-child-name"
                              style={styles.input}
                              value={draftChildName}
                              onChange={(event) => setDraftChildName(event.target.value)}
                              placeholder={`Name the new ${formatDraftChildTypeLabel(draftChildType).toLowerCase()}`}
                            />
                            <textarea
                              data-testid="draft-node-add-child-summary"
                              style={styles.compactTextarea}
                              value={draftChildSummary}
                              onChange={(event) => setDraftChildSummary(event.target.value)}
                              placeholder="Optional summary or brief description"
                            />
                            <button
                              data-testid="draft-node-add-child-save"
                              style={styles.btnGhost}
                              onClick={() => {
                                void addChildToSelectedDraftNode();
                              }}
                              disabled={!draftChildName.trim() || isPlannerBusy}
                            >
                              Add {formatDraftChildTypeLabel(draftChildType)}
                            </button>
                          </div>
                        ) : (
                          <div style={styles.helper}>
                            This node is a leaf in the staged hierarchy. Use rename or delete, or select a higher branch to add more structure.
                          </div>
                        )}
                        {draftEditMessage ? <div style={styles.success}>{draftEditMessage}</div> : null}
                        {draftEditError ? <div style={styles.error}>{draftEditError}</div> : null}
                        <div style={styles.sectionDivider} />
                        <div style={styles.label}>Suggested Next Prompts</div>
                        <div style={styles.promptList}>
                          {selectedDraftNodePrompts.map((prompt) => (
                            <button key={prompt} style={styles.promptButton} onClick={() => applyPromptSuggestion(prompt)}>
                              {prompt}
                            </button>
                          ))}
                        </div>
                        <div style={styles.sectionDivider} />
                        <div style={styles.label}>Recent AI Changes For This Node</div>
                        {selectedNodeRecentActions.length > 0 ? (
                          <div style={styles.list}>
                            {selectedNodeRecentActions.map((action, index) => {
                              const summary = summarizeAction(action);
                              const symbolStyle = summary.tone === "add"
                                ? styles.diffSymbolAdd
                                : summary.tone === "update"
                                  ? styles.diffSymbolUpdate
                                  : styles.diffSymbolWarn;
                              return (
                                <div key={`${action.type}-${index}`} style={styles.diffRow}>
                                  <div style={symbolStyle}>{summary.symbol}</div>
                                  <div>
                                    <div style={styles.diffPrimary}>{summary.title}</div>
                                    <div style={styles.diffSecondary}>{summary.detail}</div>
                                  </div>
                                </div>
                              );
                            })}
                          </div>
                        ) : (
                          <div style={styles.helper}>
                            No recent planner operations are directly tied to this node yet.
                          </div>
                        )}
                      </>
                    ) : (
                      <div style={styles.helper}>
                        Select a node in the tree to anchor follow-up planning turns to that part of the draft.
                      </div>
                    )}
                  </div>

                  <div style={styles.sideCard}>
                    <div style={styles.label}>Draft Validation</div>
                    <div style={styles.helper}>
                      Structural checks for the staged tree before you commit it into the real catalog.
                    </div>
                    <div style={styles.issueList}>
                      {draftValidation.issues.slice(0, 6).map((issue, index) => {
                        const issueStyle = issue.tone === "ok"
                          ? styles.issueCardOk
                          : styles.issueCardWarn;
                        return (
                          <div key={`${issue.title}-${index}`} style={issueStyle}>
                            <div style={styles.issueTitle}>{issue.title}</div>
                            <div style={styles.issueDetail}>{issue.detail}</div>
                          </div>
                        );
                      })}
                    </div>
                  </div>

                  <div style={styles.sideCard}>
                    <div style={styles.label}>Draft Controls</div>
                    <div style={styles.helper}>
                      Keep the tree staged until the structure looks right. Commit only when you want to persist it to products, modules, capabilities, and work items.
                    </div>
                    <div style={styles.inlineButtonRow}>
                      <button data-testid="draft-commit" style={styles.btn} onClick={confirmPendingPlan} disabled={draftTreeNodes.length === 0 || isPlannerBusy}>
                        Commit Draft
                      </button>
                      <button style={styles.btnGhost} onClick={() => setPlannerView("conversation")}>
                        Back to Chat
                      </button>
                      <button style={styles.btnDanger} onClick={dismissPendingPlan} disabled={draftTreeNodes.length === 0}>
                        Clear Draft
                      </button>
                    </div>
                  </div>

                  <div style={styles.sideCard}>
                    <div style={styles.label}>Latest Draft Ops</div>
                    {latestDraftPlan ? (
                      <>
                        <div style={styles.helper}>{latestDraftPlan.assistant_response}</div>
                        <div style={styles.list}>
                          {latestDraftPlan.actions.slice(0, 8).map((action, index) => {
                            const summary = summarizeAction(action);
                            const symbolStyle = summary.tone === "add"
                              ? styles.diffSymbolAdd
                              : summary.tone === "update"
                                ? styles.diffSymbolUpdate
                                : styles.diffSymbolWarn;
                            return (
                              <div key={`${action.type}-${index}`} style={styles.diffRow}>
                                <div style={symbolStyle}>{summary.symbol}</div>
                                <div>
                                  <div style={styles.diffPrimary}>{summary.title}</div>
                                  <div style={styles.diffSecondary}>{summary.detail}</div>
                                </div>
                              </div>
                            );
                          })}
                        </div>
                      </>
                    ) : (
                      <div style={styles.helper}>
                        No pending proposal snapshot. Use the chat to add structure, then review and keep refining the staged tree here.
                      </div>
                    )}
                  </div>
                </div>
              </div>
            ) : plannerView === "trace" ? (
              <div style={styles.draftWorkspaceMain}>
                <div style={styles.draftCanvas}>
                  <div style={styles.draftCanvasHeader}>
                    <div>
                      <div style={styles.draftCanvasTitle}>Latest Planner Turn Trace</div>
                      <div style={styles.helper}>
                        Inspect the raw planning flow: input context, model completions, tool calls, parsed plan, and any backend validation failure.
                      </div>
                    </div>
                    <div style={styles.chipRow}>
                      <div style={styles.chip}>{latestTraceEvents.length} events</div>
                    </div>
                  </div>
                  {latestTraceEvents.length > 0 ? (
                    <div style={styles.list}>
                      {latestTraceEvents.map((event) => (
                        <div key={`${event.step}-${event.title}`} style={styles.listItem}>
                          <div style={styles.listItemTitle}>
                            {event.step}. {event.title}
                          </div>
                          <div style={styles.helper}>{event.stage}</div>
                          <div style={{ ...styles.listItemMeta, marginTop: 8 }}>{event.detail}</div>
                        </div>
                      ))}
                    </div>
                  ) : (
                    <div style={styles.emptyState}>
                      No trace captured yet. Send a planner turn, then open this view to inspect the latest model/tool trace.
                    </div>
                  )}
                </div>
              </div>
            ) : (
              <div ref={transcriptRef} style={{ ...styles.transcript, flex: 1, minHeight: 0, overflow: "auto" }}>
                {pendingVoiceTranscript ? (
                  <div style={styles.voiceReviewCard}>
                    <div style={styles.voiceReviewHeader}>
                      <div>
                        <div style={styles.voiceReviewTitle}>Voice Transcript Preview</div>
                        <div style={styles.helper}>
                          Review or edit the recognized speech before sending it to the planner.
                        </div>
                      </div>
                      <div style={styles.chipRow}>
                        <div style={styles.chip}>elapsed {formatElapsedMs(voiceElapsedMs)}</div>
                        <div style={styles.chip}>{isVoiceSubmitting ? "sending" : "ready to send"}</div>
                      </div>
                    </div>
                    <textarea
                      style={{ ...styles.compactTextarea, minHeight: 88 }}
                      value={editableVoiceTranscript}
                      onChange={(event) => setEditableVoiceTranscript(event.target.value)}
                    />
                    <div style={styles.inlineButtonRow}>
                      <button style={styles.btn} onClick={() => void submitPendingVoiceTranscript()} disabled={!editableVoiceTranscript.trim() || isPlannerBusy}>
                        Send Transcript
                      </button>
                      <button style={styles.btnGhost} onClick={() => void retryVoiceCapture()} disabled={isPlannerBusy}>
                        Retry
                      </button>
                      <button style={styles.btnDanger} onClick={clearPendingVoiceReview} disabled={isPlannerBusy}>
                        Cancel
                      </button>
                    </div>
                  </div>
                ) : null}
                {messages.map((message) => (
                  <div key={message.id} style={message.role === "user" ? styles.bubbleUser : styles.bubbleAssistant}>
                    {message.role === "assistant" ? renderAssistantMessage(message) : message.content}
                    {message.meta ? <span style={styles.bubbleMeta}>{message.meta}</span> : null}
                  </div>
                ))}
              </div>
            )}
            {plannerView !== "draft" ? (
              <PlannerComposer
                draft={draft}
                onDraftChange={setDraft}
                onSend={() => {
                  void send();
                }}
                onToggleListening={() => {
                  void toggleListening();
                }}
                onConfirm={() => setDraft("confirm")}
                onDismiss={dismissPendingPlan}
                isPlannerBusy={isPlannerBusy}
                voiceEnabled={voiceEnabled}
                isListening={isListening}
                isTranscribing={isTranscribing}
                isVoiceSubmitting={isVoiceSubmitting}
                pendingVoiceTranscript={pendingVoiceTranscript}
                draftTreeNodesLength={draftTreeNodes.length}
                pendingPlan={pendingPlan}
                voiceActivity={voiceActivity}
                composerRef={composerRef}
              />
            ) : null}
          </div>
        </div>

        {!isFocusedWorkspaceView && !isCompactScreen ? renderPlannerSidebar() : null}
        {!isFocusedWorkspaceView && isCompactScreen && showCompactTools ? renderPlannerSidebar() : null}
      </div>
    </div>
  );
}
