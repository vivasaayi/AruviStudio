import React, { useEffect, useMemo, useRef, useState } from "react";
import { useMutation, useQueries, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  approveWorkItem,
  approveWorkItemPlan,
  approveWorkItemTestReview,
  archiveProduct,
  createCapability,
  createModule,
  createProduct,
  createWorkItem,
  deleteCapability,
  deleteModule,
  deleteWorkItem,
  getLatestWorkflowRunForWorkItem,
  getProductTree,
  handleWorkflowUserAction,
  listModelDefinitions,
  listProducts,
  listProviders,
  listWorkItems,
  rejectWorkItem,
  rejectWorkItemPlan,
  runModelChatCompletion,
  startWorkItemWorkflow,
  updateCapability,
  updateModule,
  updateProduct,
  updateWorkItem,
} from "../../../lib/tauri";
import { useWorkspaceStore } from "../../../state/workspaceStore";
import type { CapabilityTree, ModelDefinition, Product, ProductTree, WorkItem } from "../../../lib/types";

const styles: Record<string, React.CSSProperties> = {
  page: { display: "flex", flexDirection: "column", gap: 12, height: "100%" },
  header: { display: "flex", justifyContent: "space-between", gap: 12, alignItems: "flex-start" },
  titleWrap: { display: "flex", flexDirection: "column", gap: 4 },
  title: { fontSize: 22, fontWeight: 800, color: "#f5f7fb", margin: 0 },
  subtitle: { fontSize: 13, color: "#9da7b5", maxWidth: 760, lineHeight: 1.45 },
  topGrid: { display: "grid", gridTemplateColumns: "minmax(0, 1fr) 320px", gap: 12, minHeight: 0, flex: 1 },
  panel: { backgroundColor: "#212327", border: "1px solid #32353d", borderRadius: 14, minHeight: 0, overflow: "hidden" },
  panelBody: { padding: 16, height: "100%", overflow: "auto" },
  sectionTitle: { fontSize: 11, fontWeight: 800, letterSpacing: 0.8, textTransform: "uppercase" as const, color: "#8f96a3", marginBottom: 10 },
  transcript: { display: "flex", flexDirection: "column", gap: 10 },
  bubbleUser: { alignSelf: "flex-end", maxWidth: "80%", backgroundColor: "#0e639c", color: "#fff", borderRadius: 14, padding: "12px 14px", whiteSpace: "pre-wrap" as const },
  bubbleAssistant: { alignSelf: "flex-start", maxWidth: "84%", backgroundColor: "#2c3139", color: "#edf1f8", borderRadius: 14, padding: "12px 14px", whiteSpace: "pre-wrap" as const },
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
};

type PlannerMessage = {
  id: string;
  role: "user" | "assistant";
  content: string;
  meta?: string;
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
  | { type: "report_status"; target?: { productName?: string; workItemTitle?: string } };

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

type SpeechRecognitionLike = {
  continuous: boolean;
  interimResults: boolean;
  lang: string;
  onstart: (() => void) | null;
  onend: (() => void) | null;
  onerror: ((event: { error: string }) => void) | null;
  onresult: ((event: SpeechRecognitionEventLike) => void) | null;
  start: () => void;
  stop: () => void;
};

type SpeechRecognitionAlternativeLike = {
  transcript: string;
};

type SpeechRecognitionResultLike = {
  isFinal: boolean;
  0: SpeechRecognitionAlternativeLike;
};

type SpeechRecognitionEventLike = {
  resultIndex: number;
  results: SpeechRecognitionResultLike[];
};

declare global {
  interface Window {
    SpeechRecognition?: new () => SpeechRecognitionLike;
    webkitSpeechRecognition?: new () => SpeechRecognitionLike;
  }
}

const DEFAULT_ASSISTANT_OPENING =
  "Tell me what to do in plain English. Example: create a product called Studio Ops, add a module named Voice Planner, create a work item to wire approvals into speech, and start the workflow.";

const PLANNER_SYSTEM_PROMPT = `You are an operations planner for a product-management desktop app.
Convert the user's request into a JSON object only, with this exact shape:
{
  "assistant_response": "brief natural-language reply",
  "needs_confirmation": true,
  "clarification_question": null,
  "actions": []
}

Rules:
- Output valid JSON only. No markdown.
- Prefer actions over prose when the user's intent is actionable.
- If the request is destructive (delete, archive, reject, cancel), set needs_confirmation=true.
- If the request is ambiguous, set actions=[] and put the missing detail in clarification_question.
- Use these action types only:
create_product, update_product, archive_product,
create_module, update_module, delete_module,
create_capability, update_capability, delete_capability,
create_work_item, update_work_item, delete_work_item,
approve_work_item, reject_work_item, approve_work_item_plan, reject_work_item_plan, approve_work_item_test_review,
start_workflow, workflow_action, report_status.
- Use product/module/capability/work item names in target fields, never IDs.
- For create_work_item defaults when omitted: workItemType=feature, priority=medium, complexity=medium.
- For create_capability defaults when omitted: priority=medium, risk=medium.
- For workflow_action action must be one of approve,reject,pause,resume,cancel.
- assistant_response should be concise and operational.`;

function normalize(value?: string | null) {
  return (value ?? "").trim().toLowerCase();
}

function makeId() {
  return crypto.randomUUID();
}

function summarizeContext(context: ResolverContext) {
  const productLines = context.productTrees.map((tree) => {
    const modules = tree.modules.map((moduleTree) => {
      const capabilities: string[] = [];
      const visit = (node: CapabilityTree) => {
        capabilities.push(node.capability.name);
        node.children.forEach(visit);
      };
      moduleTree.features.forEach(visit);
      return `${moduleTree.module.name}${capabilities.length ? ` [${capabilities.join(", ")}]` : ""}`;
    });
    return `${tree.product.name}: ${modules.join(" | ") || "no modules"}`;
  });
  const workItemLines = context.workItems.slice(0, 120).map((item) => `${item.title} [${item.status}]${item.product_id ? ` product=${item.product_id}` : ""}`);
  return [
    "Products and structure:",
    ...productLines,
    "Work items:",
    ...workItemLines,
  ].join("\n");
}

function extractJsonObject(raw: string) {
  const trimmed = raw.trim();
  if (trimmed.startsWith("{") && trimmed.endsWith("}")) {
    return trimmed;
  }
  const start = trimmed.indexOf("{");
  const end = trimmed.lastIndexOf("}");
  if (start >= 0 && end > start) {
    return trimmed.slice(start, end + 1);
  }
  throw new Error("Planner model did not return JSON.");
}

function parsePlannerResponse(raw: string): PlannerPlan {
  const parsed = JSON.parse(extractJsonObject(raw)) as Partial<PlannerPlan>;
  return {
    assistant_response: typeof parsed.assistant_response === "string" ? parsed.assistant_response : "I translated that into planner actions.",
    needs_confirmation: Boolean(parsed.needs_confirmation),
    clarification_question: typeof parsed.clarification_question === "string" ? parsed.clarification_question : null,
    actions: Array.isArray(parsed.actions) ? (parsed.actions as PlannerAction[]) : [],
  };
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
      assistant_response: "I’ll approve the referenced work item if I can resolve it.",
      needs_confirmation: false,
      clarification_question: null,
      actions: [{ type: "approve_work_item" }],
    };
  }
  return {
    assistant_response: "I could not safely infer a structured action from that without a model.",
    needs_confirmation: false,
    clarification_question: "Configure a model, or be more explicit about the exact product, module, capability, or work item.",
    actions: [],
  };
}

function speak(text: string) {
  if (typeof window === "undefined" || !("speechSynthesis" in window)) {
    return;
  }
  window.speechSynthesis.cancel();
  const utterance = new SpeechSynthesisUtterance(text);
  utterance.rate = 1;
  utterance.pitch = 1;
  window.speechSynthesis.speak(utterance);
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
  const [providerId, setProviderId] = useState("");
  const [modelName, setModelName] = useState("");
  const [draft, setDraft] = useState("");
  const [messages, setMessages] = useState<PlannerMessage[]>([
    { id: makeId(), role: "assistant", content: DEFAULT_ASSISTANT_OPENING },
  ]);
  const [pendingPlan, setPendingPlan] = useState<PendingPlan | null>(null);
  const [voiceEnabled, setVoiceEnabled] = useState(true);
  const [autoSpeak, setAutoSpeak] = useState(false);
  const [isListening, setIsListening] = useState(false);
  const [speechError, setSpeechError] = useState<string | null>(null);
  const recognitionRef = useRef<SpeechRecognitionLike | null>(null);
  const transcriptRef = useRef<HTMLDivElement | null>(null);

  const { data: products = [] } = useQuery({ queryKey: ["products"], queryFn: listProducts });
  const { data: providers = [] } = useQuery({ queryKey: ["plannerProviders"], queryFn: listProviders });
  const { data: models = [] } = useQuery({ queryKey: ["plannerModels"], queryFn: listModelDefinitions });
  const { data: workItems = [] } = useQuery({ queryKey: ["plannerWorkItems"], queryFn: () => listWorkItems() });

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

  const modelOptions = useMemo(
    () => models.filter((model) => model.provider_id === providerId && model.enabled),
    [models, providerId],
  );

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
    if (!providerId && providers.length > 0) {
      setProviderId(providers[0].id);
    }
  }, [providerId, providers]);

  useEffect(() => {
    if (!providerId) {
      return;
    }
    if (!modelName || !modelOptions.some((entry) => entry.name === modelName)) {
      setModelName(modelOptions[0]?.name ?? "");
    }
  }, [modelName, modelOptions, providerId]);

  useEffect(() => {
    transcriptRef.current?.scrollTo({ top: transcriptRef.current.scrollHeight });
  }, [messages]);

  useEffect(() => {
    if (!voiceEnabled) {
      recognitionRef.current?.stop();
      recognitionRef.current = null;
      setIsListening(false);
      return;
    }
    const RecognitionCtor = window.SpeechRecognition ?? window.webkitSpeechRecognition;
    if (!RecognitionCtor) {
      setSpeechError("Speech recognition is not available in this runtime.");
      return;
    }
    const recognition = new RecognitionCtor();
    recognition.continuous = false;
    recognition.interimResults = true;
    recognition.lang = "en-US";
    recognition.onstart = () => {
      setSpeechError(null);
      setIsListening(true);
    };
    recognition.onend = () => setIsListening(false);
    recognition.onerror = (event) => {
      setSpeechError(event.error);
      setIsListening(false);
    };
    recognition.onresult = (event) => {
      let finalTranscript = "";
      for (let index = event.resultIndex; index < event.results.length; index += 1) {
        const result = event.results[index];
        if (result.isFinal) {
          finalTranscript += result[0].transcript;
        }
      }
      if (finalTranscript.trim()) {
        setDraft((current) => (current ? `${current.trim()} ${finalTranscript.trim()}` : finalTranscript.trim()));
      }
    };
    recognitionRef.current = recognition;
    return () => {
      recognition.stop();
    };
  }, [voiceEnabled]);

  const processMutation = useMutation({
    mutationFn: async (input: string) => {
      const userInput = input.trim();
      const normalized = normalize(userInput);
      if ((normalized === "yes" || normalized === "confirm" || normalized === "go ahead") && pendingPlan) {
        const execution = await executePlannerPlan(pendingPlan.plan, context);
        return {
          mode: "confirmed" as const,
          userInput,
          plan: pendingPlan.plan,
          execution,
        };
      }

      let plan: PlannerPlan;
      if (providerId && modelName) {
        const contextSummary = summarizeContext(context);
        const completion = await runModelChatCompletion({
          providerId,
          model: modelName,
          temperature: 0.1,
          maxTokens: 1800,
          messages: [
            { role: "system", content: PLANNER_SYSTEM_PROMPT },
            { role: "user", content: `Current context:\n${contextSummary}\n\nUser request:\n${userInput}` },
          ],
        });
        plan = parsePlannerResponse(completion.content);
      } else {
        plan = heuristicPlan(userInput);
      }

      if (plan.needs_confirmation && plan.actions.length > 0) {
        return {
          mode: "confirmation_required" as const,
          userInput,
          plan,
          execution: null,
        };
      }

      if (plan.actions.length === 0) {
        return {
          mode: "clarification" as const,
          userInput,
          plan,
          execution: null,
        };
      }

      const execution = await executePlannerPlan(plan, context);
      return {
        mode: "executed" as const,
        userInput,
        plan,
        execution,
      };
    },
    onSuccess: (result) => {
      setMessages((current) => {
        const next: PlannerMessage[] = [...current, { id: makeId(), role: "user", content: result.userInput }];
        if (result.mode === "confirmation_required") {
          const actionSummary = result.plan.actions.map((action) => action.type).join(", ");
          next.push({
            id: makeId(),
            role: "assistant",
            content: `${result.plan.assistant_response}\n\nPending confirmation for: ${actionSummary}. Say "confirm" to execute or give a correction.`,
            meta: "Awaiting confirmation",
          });
          return next;
        }
        if (result.mode === "confirmed") {
          const output = [
            "Executed pending plan.",
            ...(result.execution?.lines ?? []),
            ...(result.execution?.errors.length ? [`Errors: ${result.execution.errors.join(" | ")}`] : []),
          ].join("\n");
          next.push({ id: makeId(), role: "assistant", content: output, meta: "Planner execution" });
          return next;
        }
        if (result.mode === "clarification") {
          next.push({
            id: makeId(),
            role: "assistant",
            content: result.plan.clarification_question ?? result.plan.assistant_response,
            meta: "Need more detail",
          });
          return next;
        }
        const output = [
          result.plan.assistant_response,
          ...(result.execution?.lines ?? []),
          ...(result.execution?.errors.length ? [`Errors: ${result.execution.errors.join(" | ")}`] : []),
        ].join("\n");
        next.push({ id: makeId(), role: "assistant", content: output, meta: "Planner execution" });
        return next;
      });

      if (result.mode === "confirmation_required") {
        setPendingPlan({ sourceText: result.userInput, plan: result.plan });
      } else {
        setPendingPlan(null);
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
            ? `${result.plan.assistant_response}. Say confirm to execute.`
            : result.mode === "confirmed"
              ? "Executed the pending planner actions."
              : result.plan.assistant_response;
        speak(lastAssistant);
      }
    },
    onError: (error, userInput) => {
      setMessages((current) => [
        ...current,
        { id: makeId(), role: "user", content: userInput },
        { id: makeId(), role: "assistant", content: error instanceof Error ? error.message : String(error), meta: "Planner error" },
      ]);
    },
  });

  const send = async () => {
    const content = draft.trim();
    if (!content || processMutation.isPending) {
      return;
    }
    setDraft("");
    await processMutation.mutateAsync(content);
  };

  const toggleListening = () => {
    if (!recognitionRef.current) {
      setSpeechError("Speech recognition is not available.");
      return;
    }
    if (isListening) {
      recognitionRef.current.stop();
      return;
    }
    recognitionRef.current.start();
  };

  return (
    <div style={styles.page}>
      <div style={styles.header}>
        <div style={styles.titleWrap}>
          <h1 style={styles.title}>Interactive Planner</h1>
          <div style={styles.subtitle}>
            This replaces button-by-button planning with a conversational command layer. You can tell it to create, update, remove, approve, reject, start workflows, and report status. Voice input and spoken responses are supported when the runtime exposes browser speech APIs.
          </div>
        </div>
      </div>

      <div style={styles.topGrid}>
        <div style={styles.panel}>
          <div style={{ ...styles.panelBody, display: "flex", flexDirection: "column" }}>
            <div style={styles.sectionTitle}>Conversation</div>
            <div ref={transcriptRef} style={{ ...styles.transcript, flex: 1, minHeight: 0, overflow: "auto" }}>
              {messages.map((message) => (
                <div key={message.id} style={message.role === "user" ? styles.bubbleUser : styles.bubbleAssistant}>
                  {message.content}
                  {message.meta ? <span style={styles.bubbleMeta}>{message.meta}</span> : null}
                </div>
              ))}
            </div>
            <div style={styles.composerWrap}>
              <textarea
                style={styles.textarea}
                value={draft}
                onChange={(event) => setDraft(event.target.value)}
                placeholder="Say or type what you need. Example: Add a work item called Build voice planner under AruviStudio, then approve it and start the workflow."
              />
              <div style={styles.actionRow}>
                <button style={styles.btn} onClick={() => void send()} disabled={processMutation.isPending}>
                  {processMutation.isPending ? "Working..." : "Send"}
                </button>
                <button style={styles.btnGhost} onClick={toggleListening} disabled={!voiceEnabled}>
                  {isListening ? "Stop Listening" : "Start Listening"}
                </button>
                <button style={styles.btnGhost} onClick={() => setDraft("confirm")} disabled={!pendingPlan}>
                  Confirm Pending
                </button>
                <button style={styles.btnDanger} onClick={() => setPendingPlan(null)} disabled={!pendingPlan}>
                  Clear Pending
                </button>
                <span style={styles.status}>
                  {pendingPlan ? "A destructive or ambiguous action is waiting for confirmation." : "No pending confirmation."}
                </span>
              </div>
            </div>
          </div>
        </div>

        <div style={styles.panel}>
          <div style={styles.panelBody}>
            <div style={styles.sectionTitle}>Planner Controls</div>

            <div style={styles.sideCard}>
              <label style={styles.label}>Provider</label>
              <select style={styles.select} value={providerId} onChange={(event) => setProviderId(event.target.value)}>
                <option value="">No provider</option>
                {providers.map((provider) => (
                  <option key={provider.id} value={provider.id}>
                    {provider.name}
                  </option>
                ))}
              </select>
              <div style={{ height: 10 }} />
              <label style={styles.label}>Model</label>
              <select style={styles.select} value={modelName} onChange={(event) => setModelName(event.target.value)}>
                <option value="">No model</option>
                {modelOptions.map((model: ModelDefinition) => (
                  <option key={model.id} value={model.name}>
                    {model.name}
                  </option>
                ))}
              </select>
              <div style={{ ...styles.helper, marginTop: 10 }}>
                With a configured model, the planner translates free-form requests into executable actions. Without one, it falls back to simple heuristics.
              </div>
            </div>

            <div style={styles.sideCard}>
              <div style={styles.label}>Voice</div>
              <div style={styles.actionRow}>
                <button style={voiceEnabled ? styles.btnGhost : styles.btn} onClick={() => setVoiceEnabled((value) => !value)}>
                  {voiceEnabled ? "Disable Mic" : "Enable Mic"}
                </button>
                <button style={autoSpeak ? styles.btn : styles.btnGhost} onClick={() => setAutoSpeak((value) => !value)}>
                  {autoSpeak ? "Voice Replies On" : "Voice Replies Off"}
                </button>
              </div>
              {speechError ? <div style={{ ...styles.error, marginTop: 10 }}>{speechError}</div> : null}
              <div style={{ ...styles.helper, marginTop: 10 }}>
                For phone or WhatsApp calls, you still need an external telephony layer such as Twilio. This page gives you the in-app conversational planner first.
              </div>
            </div>

            <div style={styles.sideCard}>
              <div style={styles.label}>Current Scope</div>
              <div style={styles.chipRow}>
                {activeProductId ? <div style={styles.chip}>product selected</div> : null}
                {activeModuleId ? <div style={styles.chip}>module selected</div> : null}
                {activeCapabilityId ? <div style={styles.chip}>capability selected</div> : null}
                {activeWorkItemId ? <div style={styles.chip}>work item selected</div> : null}
              </div>
              <div style={{ ...styles.helper, marginTop: 10 }}>
                If you omit names, the planner tries to use the currently selected scope before asking for clarification.
              </div>
            </div>

            <div style={styles.sideCard}>
              <div style={styles.label}>Examples</div>
              <div style={styles.list}>
                <div style={styles.listItem}>
                  <div style={styles.listItemTitle}>Create structure</div>
                  <div style={styles.listItemMeta}>Create a product called Growth OS, add a module called Outreach, and add a capability called Voice Campaigns.</div>
                </div>
                <div style={styles.listItem}>
                  <div style={styles.listItemTitle}>Drive execution</div>
                  <div style={styles.listItemMeta}>Create a work item called Add WhatsApp follow-up flow under Voice Campaigns, approve it, and start the workflow.</div>
                </div>
                <div style={styles.listItem}>
                  <div style={styles.listItemTitle}>Check status</div>
                  <div style={styles.listItemMeta}>What is the status of Add WhatsApp follow-up flow?</div>
                </div>
              </div>
            </div>

            {pendingPlan ? (
              <div style={styles.sideCard}>
                <div style={styles.label}>Pending Confirmation</div>
                <div style={styles.warning}>
                  The last request produced actions that require confirmation before execution.
                </div>
                <div style={styles.list}>
                  {pendingPlan.plan.actions.map((action, index) => (
                    <div key={`${action.type}-${index}`} style={styles.listItem}>
                      <div style={styles.listItemTitle}>{action.type}</div>
                      <div style={styles.listItemMeta}>{JSON.stringify(action, null, 2)}</div>
                    </div>
                  ))}
                </div>
              </div>
            ) : null}
          </div>
        </div>
      </div>
    </div>
  );
}
