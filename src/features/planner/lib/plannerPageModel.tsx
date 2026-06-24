import React from "react";

import {
  approveWorkItem,
  approveWorkItemPlan,
  approveWorkItemTestReview,
  applySemanticTemplate,
  convertCapabilityKind,
  createCapability,
  createProductArea,
  createWorkItem,
  deleteCapability,
  deleteProductArea,
  deleteWorkItem,
  getLatestWorkflowRunForWorkItem,
  handleWorkflowUserAction,
  rejectWorkItem,
  rejectWorkItemPlan,
  startWorkItemWorkflow,
  updateCapability,
  updateProduct,
  updateProductArea,
  updateWorkItem,
} from "../../../lib/tauri";
import type {
  CapabilityTree,
  PlannerDraftChildType,
  PlannerTraceEvent,
  Product,
  ProductArea,
  ProductTree,
  WorkItem,
} from "../../../lib/types";
import { styles } from "./plannerPageStyles";

export const PLANNER_WORK_ITEM_PAGE_SIZE = 500;

export type PlannerMessageKind = "text" | "proposal" | "tree" | "report" | "execution" | "error";

export type PlannerTreeNode = {
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

export type PlannerMessage = {
  id: string;
  role: "user" | "assistant";
  content: string;
  meta?: string;
  kind?: PlannerMessageKind;
  plan?: PlannerPlan;
  treeNodes?: PlannerTreeNode[];
  traceEvents?: PlannerTraceEvent[];
};

export type PlannerAction =
  | {
      type: "update_product";
      target?: { productName?: string };
      fields: { name?: string; description?: string; vision?: string; goals?: string[]; tags?: string[] };
    }
  | {
      type: "create_product_area";
      target?: { productName?: string };
      name: string;
      description?: string;
      purpose?: string;
    }
  | {
      type: "update_product_area";
      target?: { productName?: string; productAreaName?: string };
      fields: { name?: string; description?: string; purpose?: string };
    }
  | { type: "delete_product_area"; target?: { productName?: string; productAreaName?: string } }
  | {
      type: "create_capability";
      target?: { productName?: string; productAreaName?: string; capabilityName?: string };
      name: string;
      description?: string;
      acceptanceCriteria?: string;
      technicalNotes?: string;
      priority?: "critical" | "high" | "medium" | "low";
      risk?: "high" | "medium" | "low";
    }
  | {
      type: "apply_capability_template";
      target?: { productName?: string; productAreaName?: string; capabilityName?: string };
      templateKind: "operator_chapter" | "technical_topic_book";
      name: string;
      description?: string;
      priority?: "critical" | "high" | "medium" | "low";
      risk?: "high" | "medium" | "low";
      explanation?: string;
      examples?: string;
      implementationNotes?: string;
      testGuidance?: string;
    }
  | {
      type: "convert_capability_kind";
      target?: { productName?: string; productAreaName?: string; capabilityName?: string };
      nodeKind: string;
      childStrategy?: "reject" | "reparent_to_parent";
    }
  | {
      type: "update_capability";
      target?: { productName?: string; productAreaName?: string; capabilityName?: string };
      fields: { name?: string; description?: string; acceptanceCriteria?: string; technicalNotes?: string; priority?: "critical" | "high" | "medium" | "low"; risk?: "high" | "medium" | "low" };
    }
  | { type: "delete_capability"; target?: { productName?: string; productAreaName?: string; capabilityName?: string } }
  | {
      type: "create_work_item";
      target?: { productName?: string; productAreaName?: string; capabilityName?: string };
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

export type PlannerPlan = {
  assistant_response: string;
  needs_confirmation: boolean;
  clarification_question: string | null;
  actions: PlannerAction[];
};

export type PendingPlan = {
  sourceText: string;
  plan: PlannerPlan;
};

export type ResolverContext = {
  products: Product[];
  productTrees: ProductTree[];
  workItems: WorkItem[];
  activeProductId: string | null;
  activeProductAreaId: string | null;
  activeCapabilityId: string | null;
  activeWorkItemId: string | null;
};

export type ExecutionResult = {
  lines: string[];
  errors: string[];
};

export type PlannerMutationResult =
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

export type DraftEditOperation =
  | { kind: "rename"; nodeId: string; name: string }
  | { kind: "add_child"; parentNodeId: string; childType: PlannerDraftChildType; name: string; summary?: string }
  | { kind: "delete"; nodeId: string };

export const DEFAULT_ASSISTANT_OPENING =
  "Select a product first, then describe the product area, capability, feature, story, task, or design change you want to explore. Planning stays inside the selected product until you switch products.";

export const SPEECH_PROVIDER_KEY = "speech.transcription_provider_id";
export const SPEECH_MODEL_KEY = "speech.transcription_model_name";
export const SPEECH_LOCALE_KEY = "speech.locale";
export const SPEECH_NATIVE_VOICE_KEY = "speech.native_voice";
export const SPEECH_ENABLE_MIC_KEY = "speech.enable_mic";
export const SPEECH_AUTO_SPEAK_REPLIES_KEY = "speech.auto_speak_replies";
export const SPEECH_REVIEW_BEFORE_SEND_KEY = "speech.review_before_send";

export function normalize(value?: string | null) {
  return (value ?? "").trim().toLowerCase();
}

export function makeId() {
  return crypto.randomUUID();
}

export function isInformationalOnly(plan: PlannerPlan) {
  return plan.actions.length > 0 && plan.actions.every((action) => action.type === "report_status" || action.type === "report_tree");
}

export function findProduct(context: ResolverContext, productName?: string) {
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

export function formatElapsedMs(value: number) {
  const totalSeconds = Math.max(0, Math.round(value / 1000));
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  if (minutes > 0) {
    return `${minutes}:${String(seconds).padStart(2, "0")}`;
  }
  return `${seconds}s`;
}

export function buildProductAreaOnlyTree(product: Product, productAreas: ProductArea[]): ProductTree {
  const sortedProductAreas = [...productAreas].sort((left, right) => left.sort_order - right.sort_order || left.name.localeCompare(right.name));
  return {
    product,
    product_areas: sortedProductAreas.map((product_area) => ({
      product_area,
      features: [],
    })),
    roots: sortedProductAreas.map((product_area) => ({
      id: product_area.id,
      node_type: "product_area",
      node_kind: product_area.node_kind,
      product_area_id: product_area.id,
      capability_id: null,
      parent_node_id: null,
      parent_node_type: null,
      depth: 0,
      name: product_area.name,
      description: product_area.description,
      summary: product_area.purpose,
      path: [product_area.name],
      allowed_child_kinds: ["capability"],
      children: [],
    })),
  };
}

export function findTree(context: ResolverContext, product: Product) {
  const tree = context.productTrees.find((entry) => entry.product.id === product.id);
  if (!tree) {
    throw new Error(`Product tree for "${product.name}" is not loaded.`);
  }
  return tree;
}

export function findProductArea(context: ResolverContext, product: Product, productAreaName?: string) {
  const tree = findTree(context, product);
  if (productAreaName) {
    const normalized = normalize(productAreaName);
    const exact = tree.product_areas.find((entry) => normalize(entry.product_area.name) === normalized);
    if (exact) {
      return exact.product_area;
    }
    const partial = tree.product_areas.filter((entry) => normalize(entry.product_area.name).includes(normalized));
    if (partial.length === 1) {
      return partial[0].product_area;
    }
    if (partial.length > 1) {
      throw new Error(`Multiple product_areas match "${productAreaName}" in "${product.name}".`);
    }
    throw new Error(`No product_area matches "${productAreaName}" in "${product.name}".`);
  }
  if (context.activeProductAreaId) {
    const active = tree.product_areas.find((entry) => entry.product_area.id === context.activeProductAreaId);
    if (active) {
      return active.product_area;
    }
  }
  if (tree.product_areas.length === 1) {
    return tree.product_areas[0].product_area;
  }
  throw new Error("Product Area is required.");
}

export function flattenCapabilities(tree: CapabilityTree[], bucket: CapabilityTree[] = []) {
  tree.forEach((node) => {
    bucket.push(node);
    flattenCapabilities(node.children, bucket);
  });
  return bucket;
}

export function findCapability(context: ResolverContext, product: Product, productAreaName?: string, capabilityName?: string) {
  const product_area = findProductArea(context, product, productAreaName);
  const tree = findTree(context, product);
  const productAreaTree = tree.product_areas.find((entry) => entry.product_area.id === product_area.id);
  if (!productAreaTree) {
    throw new Error(`Product Area "${product_area.name}" has no capability tree.`);
  }
  const capabilities = flattenCapabilities(productAreaTree.features);
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
      throw new Error(`Multiple capabilities match "${capabilityName}" in "${product_area.name}".`);
    }
    throw new Error(`No capability matches "${capabilityName}" in "${product_area.name}".`);
  }
  if (context.activeCapabilityId) {
    const active = capabilities.find((entry) => entry.capability.id === context.activeCapabilityId);
    if (active) {
      return active.capability;
    }
  }
  throw new Error("Capability is required.");
}

export function findWorkItem(context: ResolverContext, workItemTitle?: string, productName?: string) {
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

export function formatArrayField(values?: string[]) {
  return values?.join(", ") ?? "";
}

export function formatWorkItemLine(workItem: WorkItem, indent: string) {
  return `${indent}- ${workItem.title} [${workItem.status}]`;
}

export function appendWorkItemHierarchy(lines: string[], items: WorkItem[], parentId: string | null, indent: string) {
  const children = items
    .filter((item) => (item.parent_work_item_id ?? null) === parentId)
    .sort((left, right) => left.sort_order - right.sort_order || left.title.localeCompare(right.title));
  children.forEach((child) => {
    lines.push(formatWorkItemLine(child, indent));
    appendWorkItemHierarchy(lines, items, child.id, `${indent}  `);
  });
}

export function buildWorkItemTreeReport(context: ResolverContext, productName?: string) {
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

    tree.product_areas.forEach((productAreaTree) => {
      lines.push(`  ${productAreaTree.product_area.name}`);

      const productAreaDirectItems = productItems.filter(
        (item) => item.product_area_id === productAreaTree.product_area.id && !item.capability_id,
      );
      if (productAreaDirectItems.length > 0) {
        lines.push("    direct stories/tasks");
        appendWorkItemHierarchy(lines, productAreaDirectItems, null, "      ");
        productAreaDirectItems.forEach((item) => includedWorkItemIds.add(item.id));
      }

      const flattenedCapabilities = flattenCapabilities(productAreaTree.features);
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
      lines.push("  no stories/tasks");
    }
    lines.push("");
  });

  return lines.join("\n").trim();
}

export function buildWorkItemTreeNodes(context: ResolverContext, productName?: string): PlannerTreeNode[] {
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
    const productAreaNodes: PlannerTreeNode[] = [];

    if (tree) {
      tree.product_areas.forEach((productAreaTree) => {
        const productAreaChildren: PlannerTreeNode[] = [];
        const productAreaDirectItems = productItems.filter(
          (item) => item.product_area_id === productAreaTree.product_area.id && !item.capability_id,
        );
        if (productAreaDirectItems.length > 0) {
          productAreaChildren.push({
            id: `${productAreaTree.product_area.id}-direct`,
            label: "Direct Delivery Items",
            children: buildWorkItemNodes(productAreaDirectItems, null),
          });
          productAreaDirectItems.forEach((item) => includedWorkItemIds.add(item.id));
        }

        flattenCapabilities(productAreaTree.features).forEach((capabilityTree) => {
          const capabilityItems = productItems.filter((item) => item.capability_id === capabilityTree.capability.id);
          if (capabilityItems.length === 0) {
            return;
          }
          productAreaChildren.push({
            id: capabilityTree.capability.id,
            label: capabilityTree.capability.name,
            children: buildWorkItemNodes(capabilityItems, null),
          });
          capabilityItems.forEach((item) => includedWorkItemIds.add(item.id));
        });

        productAreaNodes.push({
          id: productAreaTree.product_area.id,
          label: productAreaTree.product_area.name,
          children: productAreaChildren,
        });
      });
    }

    const unscopedItems = productItems.filter(
      (item) => !includedWorkItemIds.has(item.id) && !item.parent_work_item_id,
    );
    if (unscopedItems.length > 0) {
      productAreaNodes.push({
        id: `${product.id}-unscoped`,
        label: "Unscoped",
        children: buildWorkItemNodes(unscopedItems, null),
      });
    }

    if (productAreaNodes.length === 0) {
      productAreaNodes.push({
        id: `${product.id}-empty`,
        label: "No stories/tasks",
        meta: "empty",
        children: [],
      });
    }

    return {
      id: product.id,
      label: product.name,
      children: productAreaNodes,
    };
  });
}

export function summarizeAction(action: PlannerAction | Record<string, unknown> | null | undefined) {
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
  const target = raw.target as { productName?: string; productAreaName?: string; capabilityName?: string; workItemTitle?: string } | undefined;
  const name = typeof raw.name === "string" ? raw.name : undefined;
  const title = typeof raw.title === "string" ? raw.title : undefined;
  const description = typeof raw.description === "string" ? raw.description : undefined;
  const vision = typeof raw.vision === "string" ? raw.vision : undefined;
  const fields = raw.fields ?? undefined;
  switch (actionType) {
    case "create_product_area":
      return { symbol: "+", tone: "add", title: `Create product area ${name ?? target?.productAreaName ?? "unnamed product area"}`, detail: target?.productName ? `Product: ${target.productName}` : "Attach to selected product." };
    case "create_capability":
      return { symbol: "+", tone: "add", title: `Create capability ${name ?? target?.capabilityName ?? "unnamed capability"}`, detail: [target?.productName, target?.productAreaName].filter(Boolean).join(" / ") || "Attach to selected scope." };
    case "apply_capability_template":
      return {
        symbol: "+",
        tone: "add",
        title: `Apply template ${String(raw.templateKind ?? "chapter")} to ${name ?? "unnamed topic"}`,
        detail: [target?.productName, target?.productAreaName, target?.capabilityName].filter(Boolean).join(" / ") || description || "Create a product-design scaffold.",
      };
    case "convert_capability_kind":
      return {
        symbol: "~",
        tone: "update",
        title: `Convert capability ${target?.capabilityName ?? ""}`.trim(),
        detail: `nodeKind=${String(raw.nodeKind ?? "unknown")} childStrategy=${String(raw.childStrategy ?? "reject")}`,
      };
    case "create_work_item":
      return { symbol: "+", tone: "add", title: `Create story/task ${title ?? target?.workItemTitle ?? "untitled work item"}`, detail: [target?.productName, target?.productAreaName, target?.capabilityName].filter(Boolean).join(" / ") || description || "New delivery work proposal." };
    case "update_product":
      return { symbol: "~", tone: "update", title: `Update product ${target?.productName ?? ""}`.trim(), detail: JSON.stringify(fields, null, 2) };
    case "update_product_area":
      return { symbol: "~", tone: "update", title: `Update product area ${target?.productAreaName ?? ""}`.trim(), detail: JSON.stringify(fields, null, 2) };
    case "update_capability":
      return { symbol: "~", tone: "update", title: `Update capability ${target?.capabilityName ?? ""}`.trim(), detail: JSON.stringify(fields, null, 2) };
    case "update_work_item":
      return { symbol: "~", tone: "update", title: `Update story/task ${target?.workItemTitle ?? ""}`.trim(), detail: JSON.stringify(fields, null, 2) };
    case "approve_work_item":
    case "approve_work_item_plan":
    case "approve_work_item_test_review":
    case "start_workflow":
    case "workflow_action":
    case "reject_work_item":
    case "reject_work_item_plan":
    case "archive_product":
    case "delete_product_area":
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

export function getReportTreeProductName(plan: PlannerPlan) {
  const treeAction = plan.actions.find((action): action is Extract<PlannerAction, { type: "report_tree" }> => action.type === "report_tree");
  return treeAction?.target?.productName;
}

export function TreeNodeView({ node }: { node: PlannerTreeNode }) {
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

export function FormattedPlannerText({ content }: { content: string }) {
  const lines = content.split("\n");

  return (
    <div style={styles.messageText}>
      {lines.map((line, lineIndex) => (
        <React.Fragment key={`${line}-${lineIndex}`}>
          {renderInlinePlannerMarkdown(line)}
          {lineIndex < lines.length - 1 ? <br /> : null}
        </React.Fragment>
      ))}
    </div>
  );
}

export function renderInlinePlannerMarkdown(line: string) {
  return line.split(/(\*\*[^*]+\*\*)/g).map((part, index) => {
    if (part.startsWith("**") && part.endsWith("**") && part.length > 4) {
      return <strong key={`${part}-${index}`}>{part.slice(2, -2)}</strong>;
    }
    return <React.Fragment key={`${part}-${index}`}>{part}</React.Fragment>;
  });
}

export function PlannerComposer({
  draft,
  onDraftChange,
  onSend,
  onToggleListening,
  onOpenDraftWorkspace,
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
  scopeChips,
  scopeHint,
  isProductSelected,
}: {
  draft: string;
  onDraftChange: (value: string) => void;
  onSend: () => void;
  onToggleListening: () => void;
  onOpenDraftWorkspace: () => void;
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
  scopeChips: string[];
  scopeHint: string;
  isProductSelected: boolean;
}) {
  return (
    <div style={styles.composerWrap}>
      <textarea
        ref={composerRef}
        data-testid="planner-input"
        style={styles.textarea}
        value={draft}
        onChange={(event) => onDraftChange(event.target.value)}
        placeholder={isProductSelected
          ? "Describe what to design inside the selected product. Example: Add reporting capabilities, features, and starter stories."
          : "Select a product before planning. Create products in the Products page."}
      />
      {scopeChips.length > 0 ? (
        <div style={styles.composerScopeCard}>
          <div style={styles.composerScopeTitle}>Current Scope</div>
          <div style={{ ...styles.chipRow, marginTop: 0 }}>
            {scopeChips.map((chip) => (
              <div key={chip} style={styles.chip}>
                {chip}
              </div>
            ))}
          </div>
          <div style={{ ...styles.helper, marginTop: 8 }}>{scopeHint}</div>
        </div>
      ) : null}
      {draftTreeNodesLength > 0 ? (
        <div style={styles.inlineButtonRow}>
          <button style={styles.btnGhost} onClick={onOpenDraftWorkspace}>
            Open Review
          </button>
          <button style={styles.btnGhost} onClick={onConfirm} disabled={isPlannerBusy}>
            Apply Design
          </button>
          <button style={styles.btnDanger} onClick={onDismiss} disabled={isPlannerBusy}>
            Clear Design
          </button>
        </div>
      ) : null}
      <div style={styles.actionRow}>
        <button data-testid="planner-send" style={styles.btn} onClick={onSend} disabled={isPlannerBusy || !isProductSelected}>
          {isPlannerBusy ? "Working..." : isProductSelected ? "Send" : "Select Product"}
        </button>
        <button style={styles.btnGhost} onClick={onToggleListening} disabled={!isProductSelected || !voiceEnabled || isTranscribing || isVoiceSubmitting || Boolean(pendingVoiceTranscript)}>
          {isListening
            ? "Stop Recording"
            : isTranscribing
              ? "Transcribing..."
              : isVoiceSubmitting
                ? "Sending Voice..."
                : "Start Voice Input"}
        </button>
        {draftTreeNodesLength === 0 ? (
          <>
            <button style={styles.btnGhost} onClick={onConfirm} disabled={!pendingPlan}>
              Apply Proposal
            </button>
            <button style={styles.btnDanger} onClick={onDismiss} disabled={!pendingPlan}>
              Clear Pending
            </button>
          </>
        ) : null}
        <span style={styles.status}>
          {voiceActivity
            ? voiceActivity
            : pendingVoiceTranscript
              ? "Voice transcript is ready. Review, edit, send, retry, or cancel."
              : draftTreeNodesLength > 0
                ? "A staged design is active. Review it, export the packet, then apply when ready."
                : pendingPlan
                  ? "A proposed plan is waiting for confirmation."
                  : isProductSelected
                    ? "No pending proposal."
                    : "Select a product to begin planning."}
        </span>
      </div>
    </div>
  );
}

export function parseDraftNodeType(meta?: string | null) {
  if (!meta) {
    return "node";
  }
  if (meta.includes("product")) {
    return "product";
  }
  if (meta.includes("product area") || meta.includes("product_area")) {
    return "product area";
  }
  if (meta.includes("capability")) {
    return "capability";
  }
  if (meta.includes("work item")) {
    return "work item";
  }
  return "node";
}

export function getPlannerNodeType(node: PlannerTreeNode | null | undefined) {
  if (!node) {
    return "node";
  }
  if (node.node_type) {
    return node.node_type.replace("_", " ");
  }
  return parseDraftNodeType(node.meta);
}

export function collectTreeNodeIds(nodes: PlannerTreeNode[]): string[] {
  return nodes.flatMap((node) => [node.id, ...collectTreeNodeIds(node.children)]);
}

export function SelectableTreeNodeView({
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

export function buildProposalTreeNodes(plan: PlannerPlan): PlannerTreeNode[] {
  const productNodes = new Map<string, PlannerTreeNode>();
  const productAreaNodes = new Map<string, PlannerTreeNode>();
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

  const ensureProductArea = (productName?: string | null, productAreaName?: string | null) => {
    const product = ensureProduct(productName);
    const label = productAreaName?.trim() || "Proposed capability";
    const key = `${product.label}::${label}`;
    let node = productAreaNodes.get(key);
    if (!node) {
      node = { id: `proposal-product-area-${key}`, label, meta: "proposed product area", node_type: "product_area", evidence: [], children: [] };
      productAreaNodes.set(key, node);
      product.children.push(node);
    }
    return node!;
  };

  const ensureCapability = (productName?: string | null, productAreaName?: string | null, capabilityName?: string | null) => {
    const product_area = ensureProductArea(productName, productAreaName);
    const label = capabilityName?.trim() || "Proposed capability";
    const key = `${product_area.id}::${label}`;
    let node = capabilityNodes.get(key);
    if (!node) {
      node = { id: `proposal-capability-${key}`, label, meta: "proposed capability", node_type: "capability", evidence: [], children: [] };
      capabilityNodes.set(key, node);
      product_area.children.push(node);
    }
    return node!;
  };

  for (const action of plan.actions) {
    const target = (action as { target?: { productName?: string; productAreaName?: string; capabilityName?: string; workItemTitle?: string } }).target;
    switch (action.type) {
      case "create_product_area":
        ensureProductArea(target?.productName, action.name ?? target?.productAreaName ?? null);
        break;
      case "create_capability":
      case "apply_capability_template":
        ensureCapability(target?.productName, target?.productAreaName, action.name ?? target?.capabilityName ?? null);
        break;
      case "create_work_item": {
        const capability = ensureCapability(target?.productName, target?.productAreaName, target?.capabilityName ?? null);
        capability.children.push({
          id: `proposal-work-item-${capability.id}-${action.title ?? target?.workItemTitle ?? capability.children.length}`,
          label: action.title ?? target?.workItemTitle ?? "Proposed story/task",
          meta: "proposed story/task",
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

export function findTreeNodeById(nodes: PlannerTreeNode[], nodeId: string | null): PlannerTreeNode | null {
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

export function findTreeNodePath(nodes: PlannerTreeNode[], nodeId: string | null, trail: PlannerTreeNode[] = []): PlannerTreeNode[] {
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

export function flattenTreeNodes(nodes: PlannerTreeNode[]): PlannerTreeNode[] {
  return nodes.flatMap((node) => [node, ...flattenTreeNodes(node.children)]);
}

export function findAncestorNodeByType(path: PlannerTreeNode[], nodeType: string) {
  for (let index = path.length - 1; index >= 0; index -= 1) {
    if (getPlannerNodeType(path[index]) === nodeType) {
      return path[index];
    }
  }
  return null;
}

export function resolveVoiceNodeReference(
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
  if (["this product_area", "selected product_area"].includes(reference)) {
    return findAncestorNodeByType(selectedPath, "product area");
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

export type DraftValidationIssue = {
  tone: "ok" | "warn";
  title: string;
  detail: string;
};

export type DraftValidationSummary = {
  score: number;
  counts: Record<"product" | "product area" | "capability" | "work item", number>;
  issues: DraftValidationIssue[];
};

export function buildDraftValidation(nodes: PlannerTreeNode[]): DraftValidationSummary {
  const counts: DraftValidationSummary["counts"] = {
    product: 0,
    "product area": 0,
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
        title: `${node.label} needs product areas`,
        detail: "Products should usually have at least one product area before the design is applied.",
      });
    }
    if (nodeType === "product area" && node.children.length === 0) {
      issues.push({
        tone: "warn",
        title: `${node.label} is empty`,
        detail: "Product areas should contain capabilities, features, and starter stories so the plan is actionable.",
      });
    }
    if (nodeType === "capability" && node.children.length === 0) {
      issues.push({
        tone: "warn",
        title: `${node.label} has no features or stories`,
        detail: "Capabilities are stronger when they break down into concrete features with implementation-ready stories.",
      });
    }

    node.children.forEach(visit);
  }

  nodes.forEach(visit);

  if (counts.product === 0) {
    issues.push({
      tone: "warn",
      title: "No staged product root",
      detail: "The design needs a product root before it can be applied to the catalog.",
    });
  } else {
    issues.unshift({
      tone: "ok",
      title: "Design tree is structurally valid",
      detail: "A product root exists and the planner can keep refining the staged hierarchy before apply.",
    });
  }

  const warningCount = issues.filter((issue) => issue.tone === "warn").length;
  const score = Math.max(35, 100 - warningCount * 12);
  return { score, counts, issues };
}

export function buildSuggestedPrompts(node: PlannerTreeNode | null): string[] {
  if (!node) {
    return [
      "Design the selected product's product areas, capabilities, features, and starter stories in one review packet.",
      "Show me what is missing in this design before I apply it.",
    ];
  }
  const resolvedNodeType = getPlannerNodeType(node);
  switch (resolvedNodeType) {
    case "product":
      return [
        `Expand ${node.label} with missing capabilities and operational areas.`,
        `What is missing under ${node.label} before I apply it?`,
        `Add notification, reporting, and integration capabilities under ${node.label}.`,
      ];
    case "product area":
      return [
        `Enhance ${node.label} with 3 concrete capabilities.`,
        `Break ${node.label} into capabilities, features, and starter stories.`,
        `What risks or missing features exist under ${node.label}?`,
      ];
    case "capability":
      return [
        `Add implementation stories and tasks under ${node.label}.`,
        `Revise ${node.label} to be more concrete and execution-ready.`,
        `What acceptance criteria or technical notes are missing for ${node.label}?`,
      ];
    case "work item":
      return [
        `Revise ${node.label} to be more specific and testable.`,
        `Split ${node.label} into smaller stories or tasks if needed.`,
        `Add risks, constraints, and acceptance criteria to ${node.label}.`,
      ];
    default:
      return [
        `Expand ${node.label}.`,
        `What is missing under ${node.label}?`,
      ];
  }
}

export function getAllowedDraftChildTypes(node: PlannerTreeNode | null): PlannerDraftChildType[] {
  const nodeType = getPlannerNodeType(node);
  switch (nodeType) {
    case "product":
      return ["product_area", "work_item"];
    case "product area":
      return ["capability", "work_item"];
    case "capability":
      return ["capability", "work_item"];
    default:
      return [];
  }
}

export function formatDraftChildTypeLabel(type: PlannerDraftChildType) {
  switch (type) {
    case "work_item":
      return "Work Item";
    case "product_area":
      return "Product Area";
    case "capability":
      return "Capability";
  }
}

export function findRelevantPlanActions(plan: PlannerPlan | null, node: PlannerTreeNode | null) {
  if (!plan || !node) {
    return [];
  }

  const nodeType = getPlannerNodeType(node);
  return plan.actions.filter((action) => {
    const target = (action as { target?: { productName?: string; productAreaName?: string; capabilityName?: string; workItemTitle?: string } }).target;
    if (nodeType === "product") {
      return target?.productName === node.label;
    }
    if (nodeType === "product area") {
      return action.type === "create_product_area"
        ? action.name === node.label || target?.productAreaName === node.label
        : target?.productAreaName === node.label;
    }
    if (nodeType === "capability") {
      return action.type === "create_capability" || action.type === "apply_capability_template"
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

export async function executePlannerAction(action: PlannerAction, context: ResolverContext): Promise<string[]> {
  switch (action.type) {
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
    case "create_product_area": {
      const product = findProduct(context, action.target?.productName);
      const product_area = await createProductArea({
        productId: product.id,
        name: action.name,
        description: action.description ?? "",
        purpose: action.purpose ?? "",
        nodeKind: (action as { nodeKind?: string }).nodeKind,
        explanation: (action as { explanation?: string }).explanation,
        examples: (action as { examples?: string }).examples,
        implementationNotes: (action as { implementationNotes?: string }).implementationNotes,
        testGuidance: (action as { testGuidance?: string }).testGuidance,
      });
      return [`Created product area "${product_area.name}" in "${product.name}".`];
    }
    case "update_product_area": {
      const product = findProduct(context, action.target?.productName);
      const product_area = findProductArea(context, product, action.target?.productAreaName);
      const updated = await updateProductArea({
        id: product_area.id,
        name: action.fields.name,
        description: action.fields.description,
        purpose: action.fields.purpose,
        nodeKind: (action.fields as { nodeKind?: string }).nodeKind,
        explanation: (action.fields as { explanation?: string }).explanation,
        examples: (action.fields as { examples?: string }).examples,
        implementationNotes: (action.fields as { implementationNotes?: string }).implementationNotes,
        testGuidance: (action.fields as { testGuidance?: string }).testGuidance,
      });
      return [`Updated capability "${updated.name}" in "${product.name}".`];
    }
    case "delete_product_area": {
      const product = findProduct(context, action.target?.productName);
      const product_area = findProductArea(context, product, action.target?.productAreaName);
      await deleteProductArea(product_area.id);
      return [`Deleted product area "${product_area.name}" from "${product.name}".`];
    }
    case "create_capability": {
      const product = findProduct(context, action.target?.productName);
      const product_area = findProductArea(context, product, action.target?.productAreaName);
      const parentCapability = action.target?.capabilityName
        ? findCapability(context, product, product_area.name, action.target.capabilityName)
        : null;
      const capability = await createCapability({
        productAreaId: product_area.id,
        parentCapabilityId: parentCapability?.id,
        name: action.name,
        description: action.description ?? "",
        acceptanceCriteria: action.acceptanceCriteria ?? "",
        priority: action.priority ?? "medium",
        risk: action.risk ?? "medium",
        technicalNotes: action.technicalNotes ?? "",
        nodeKind: (action as { nodeKind?: string }).nodeKind,
        explanation: (action as { explanation?: string }).explanation,
        examples: (action as { examples?: string }).examples,
        implementationNotes: (action as { implementationNotes?: string }).implementationNotes,
        testGuidance: (action as { testGuidance?: string }).testGuidance,
      });
      return [`Created capability "${capability.name}" in "${product_area.name}".`];
    }
    case "apply_capability_template": {
      const product = findProduct(context, action.target?.productName);
      const product_area = findProductArea(context, product, action.target?.productAreaName);
      const parentCapability = action.target?.capabilityName
        ? findCapability(context, product, product_area.name, action.target.capabilityName)
        : null;
      const result = await applySemanticTemplate({
        productAreaId: product_area.id,
        parentCapabilityId: parentCapability?.id,
        templateKind: action.templateKind,
        name: action.name,
        description: action.description,
        priority: action.priority,
        risk: action.risk,
        explanation: action.explanation,
        examples: action.examples,
        implementationNotes: action.implementationNotes,
        testGuidance: action.testGuidance,
      });
      return [`Applied template ${result.template_kind} to "${result.topic_node.name}".`];
    }
    case "convert_capability_kind": {
      const product = findProduct(context, action.target?.productName);
      const capability = findCapability(context, product, action.target?.productAreaName, action.target?.capabilityName);
      const result = await convertCapabilityKind({
        id: capability.id,
        nodeKind: action.nodeKind,
        childStrategy: action.childStrategy,
      });
      return [`Converted capability "${result.capability.name}" to ${result.capability.node_kind}.`];
    }
    case "update_capability": {
      const product = findProduct(context, action.target?.productName);
      const capability = findCapability(context, product, action.target?.productAreaName, action.target?.capabilityName);
      const updated = await updateCapability({
        id: capability.id,
        name: action.fields.name,
        description: action.fields.description,
        acceptanceCriteria: action.fields.acceptanceCriteria,
        technicalNotes: action.fields.technicalNotes,
        priority: action.fields.priority,
        risk: action.fields.risk,
        nodeKind: (action.fields as { nodeKind?: string }).nodeKind,
        explanation: (action.fields as { explanation?: string }).explanation,
        examples: (action.fields as { examples?: string }).examples,
        implementationNotes: (action.fields as { implementationNotes?: string }).implementationNotes,
        testGuidance: (action.fields as { testGuidance?: string }).testGuidance,
      });
      return [`Updated capability "${updated.name}".`];
    }
    case "delete_capability": {
      const product = findProduct(context, action.target?.productName);
      const capability = findCapability(context, product, action.target?.productAreaName, action.target?.capabilityName);
      await deleteCapability(capability.id);
      return [`Deleted capability "${capability.name}".`];
    }
    case "create_work_item": {
      const product = findProduct(context, action.target?.productName);
      const product_area = action.target?.productAreaName ? findProductArea(context, product, action.target.productAreaName) : context.activeProductAreaId ? findProductArea(context, product, undefined) : null;
      const capability = action.target?.capabilityName ? findCapability(context, product, action.target?.productAreaName, action.target.capabilityName) : context.activeCapabilityId ? findCapability(context, product, product_area?.name, undefined) : null;
      const workItem = await createWorkItem({
        productId: product.id,
        productAreaId: product_area?.id,
        capabilityId: capability?.id,
        title: action.title,
        problemStatement: action.problemStatement ?? action.description ?? "",
        description: action.description ?? "",
        acceptanceCriteria: action.acceptanceCriteria ?? "",
        constraints: action.constraints ?? "",
        workItemType: action.workItemType ?? "story",
        priority: action.priority ?? "medium",
        complexity: action.complexity ?? "medium",
      });
      return [`Created story/task "${workItem.title}" in "${product.name}".`];
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
      return [`Updated story/task "${updated.title}".`];
    }
    case "delete_work_item": {
      const workItem = findWorkItem(context, action.target?.workItemTitle, action.target?.productName);
      await deleteWorkItem(workItem.id);
      return [`Deleted story/task "${workItem.title}".`];
    }
    case "approve_work_item": {
      const workItem = findWorkItem(context, action.target?.workItemTitle, action.target?.productName);
      await approveWorkItem(workItem.id, action.notes);
      return [`Approved story/task "${workItem.title}".`];
    }
    case "reject_work_item": {
      const workItem = findWorkItem(context, action.target?.workItemTitle, action.target?.productName);
      await rejectWorkItem(workItem.id, action.notes ?? "Rejected from interactive planner.");
      return [`Rejected story/task "${workItem.title}".`];
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

export type DesignReviewPacketInput = {
  title: string;
  generatedAt: string;
  activeProductName?: string | null;
  currentProducts: Product[];
  currentProductTrees: ProductTree[];
  currentWorkItems: WorkItem[];
  currentWorkItemsHasMore: boolean;
  draftTreeNodes: PlannerTreeNode[];
  plan: PlannerPlan | null;
  validation: DraftValidationSummary;
  selectedNode: PlannerTreeNode | null;
  latestAssistantText?: string | null;
};

export function buildDesignReviewPacketHtml(input: DesignReviewPacketInput) {
  const actionSummaries = (input.plan?.actions ?? []).map((action) => summarizeAction(action));
  const rootNames = input.draftTreeNodes.map((node) => node.label);
  const featureActions = actionSummaries.filter((summary) =>
    /create|update|apply|convert/i.test(summary.title),
  );
  const workActions = actionSummaries.filter((summary) =>
    /work item|task/i.test(summary.title),
  );
  const riskItems = [
    ...input.validation.issues.filter((issue) => issue.tone === "warn").map((issue) => `${issue.title}: ${issue.detail}`),
    ...(input.plan?.clarification_question ? [`Open question: ${input.plan.clarification_question}`] : []),
  ];
  const changeSetJson = JSON.stringify(
    {
      generatedAt: input.generatedAt,
      title: input.title,
      selectedNode: input.selectedNode?.label ?? null,
      draftTree: input.draftTreeNodes,
      actions: input.plan?.actions ?? [],
      validation: input.validation,
    },
    null,
    2,
  );

  return `<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>${escapeHtml(input.title)} - Design Review Packet</title>
    <script src="https://cdn.jsdelivr.net/npm/mermaid@10/dist/mermaid.min.js"></script>
    <script>window.addEventListener("DOMContentLoaded", function () { if (window.mermaid) mermaid.initialize({ startOnLoad: true, theme: "base" }); });</script>
    <style>
      :root {
        color-scheme: light;
        --bg: #f8fafc;
        --panel: #ffffff;
        --ink: #111827;
        --muted: #64748b;
        --border: #d8dee8;
        --accent: #2563eb;
        --accent-soft: #eff6ff;
        --ok: #15803d;
        --warn: #a16207;
        --danger: #b91c1c;
      }
      * { box-sizing: border-box; }
      body { margin: 0; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; background: var(--bg); color: var(--ink); line-height: 1.55; }
      .shell { max-width: 1180px; margin: 0 auto; padding: 28px; }
      .hero { background: linear-gradient(145deg, #eff6ff, #ffffff 64%); border: 1px solid #bfdbfe; border-radius: 16px; padding: 24px; margin-bottom: 18px; }
      .eyebrow { color: var(--accent); font-size: 12px; font-weight: 800; letter-spacing: .12em; text-transform: uppercase; }
      h1 { margin: 8px 0 10px; font-size: 34px; line-height: 1.05; }
      h2 { margin: 0 0 12px; font-size: 20px; }
      h3 { margin: 16px 0 8px; font-size: 15px; }
      p { margin: 0 0 10px; }
      .meta { color: var(--muted); font-size: 13px; }
      .grid { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 10px; margin-top: 16px; }
      .metric { background: var(--panel); border: 1px solid var(--border); border-radius: 12px; padding: 12px; }
      .metric-label { color: var(--muted); font-size: 11px; font-weight: 800; letter-spacing: .08em; text-transform: uppercase; }
      .metric-value { font-size: 24px; font-weight: 900; margin-top: 4px; }
      .section { background: var(--panel); border: 1px solid var(--border); border-radius: 14px; padding: 18px; margin: 14px 0; }
      .toc { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 8px; margin-top: 16px; }
      .toc a { color: #1e3a8a; background: #f8fafc; border: 1px solid var(--border); border-radius: 10px; padding: 8px 10px; text-decoration: none; font-weight: 700; font-size: 13px; }
      ul { margin: 8px 0 0 20px; padding: 0; }
      li { margin: 5px 0; }
      .badge { display: inline-flex; border-radius: 999px; padding: 4px 8px; background: var(--accent-soft); color: #1d4ed8; font-size: 12px; font-weight: 800; margin: 2px 4px 2px 0; }
      .diff { border-top: 1px solid #e2e8f0; padding: 10px 0; display: grid; grid-template-columns: 32px minmax(0, 1fr); gap: 10px; }
      .symbol { font-weight: 900; color: var(--ok); }
      .warn { color: var(--warn); }
      .danger { color: var(--danger); }
      .diagram { background: #ffffff; border: 1px solid var(--border); border-radius: 12px; padding: 12px; overflow: auto; }
      pre { white-space: pre-wrap; word-break: break-word; background: #0f172a; color: #e2e8f0; border-radius: 12px; padding: 14px; overflow: auto; }
      .approval { border-color: #bfdbfe; background: #eff6ff; }
      @media print { body { background: white; } .shell { max-width: none; padding: 0; } .section, .hero { break-inside: avoid; } }
    </style>
  </head>
  <body>
    <main class="shell">
      <section class="hero">
        <div class="eyebrow">Design Review Packet</div>
        <h1>${escapeHtml(input.title)}</h1>
        <p class="meta">Generated ${escapeHtml(input.generatedAt)}${input.activeProductName ? ` · Active product: ${escapeHtml(input.activeProductName)}` : ""}</p>
        <div class="grid">
          <div class="metric"><div class="metric-label">Current Products</div><div class="metric-value">${input.currentProducts.length}</div></div>
          <div class="metric"><div class="metric-label">Draft Roots</div><div class="metric-value">${input.draftTreeNodes.length}</div></div>
          <div class="metric"><div class="metric-label">Proposed Changes</div><div class="metric-value">${input.plan?.actions.length ?? 0}</div></div>
          <div class="metric"><div class="metric-label">Readiness</div><div class="metric-value">${input.validation.score}</div></div>
        </div>
        <nav class="toc">
          ${[
            "Executive Summary",
            "Current State",
            "Proposed Architecture",
            "Change Diff",
            "Feature Specification",
            "UX / Design Proposal",
            "Implementation Plan",
            "Risk Review",
            "Work Breakdown",
            "Approval Section",
          ].map((label, index) => `<a href="#section-${index + 1}">${index + 1}. ${label}</a>`).join("")}
        </nav>
      </section>

      ${packetSection(1, "Executive Summary", [
        paragraph(input.latestAssistantText || input.plan?.assistant_response || "This packet captures the proposed product design before the catalog is changed."),
        list([
          rootNames.length > 0 ? `Primary proposed structure: ${rootNames.join(", ")}.` : "No staged structure is currently available.",
          `The packet includes ${input.plan?.actions.length ?? 0} proposed catalog operation(s).`,
          "No catalog changes should be applied until this packet is reviewed and approved.",
        ]),
      ])}

      ${packetSection(2, "Current State", [
        list([
          `${input.currentProducts.length} product(s) exist in the current workspace.`,
          `${input.currentProductTrees.reduce((total, tree) => total + tree.roots.length, 0)} root section(s) are loaded across current products.`,
          input.currentWorkItemsHasMore
            ? `The planner context is capped at the first ${input.currentWorkItems.length} story/task item(s); use Work Items for full paged delivery browsing.`
            : `${input.currentWorkItems.length} story/task item(s) are currently visible to the planner.`,
          input.activeProductName ? `Current active product context: ${input.activeProductName}.` : "No active product context was selected.",
        ]),
      ])}

      ${packetSection(3, "Proposed Architecture", [
        `<div class="diagram"><pre class="mermaid">${escapeHtml(buildMermaidDiagram(input.draftTreeNodes))}</pre></div>`,
        paragraph("The architecture diagram is generated from the staged design tree. Use it to inspect hierarchy, ownership, and major boundaries before applying changes."),
      ])}

      ${packetSection(4, "Change Diff", [
        actionSummaries.length > 0
          ? actionSummaries.map((summary) => `<div class="diff"><div class="symbol">${escapeHtml(summary.symbol)}</div><div><strong>${escapeHtml(summary.title)}</strong>${summary.detail ? `<div class="meta">${escapeHtml(summary.detail)}</div>` : ""}</div></div>`).join("")
          : paragraph("No structured change actions are currently available."),
      ])}

      ${packetSection(5, "Feature Specification", [
        list([
          ...featureActions.map((summary) => summary.title),
          featureActions.length === 0 ? "No explicit feature additions or modifications were found in the latest plan." : "",
        ].filter((value): value is string => Boolean(value))),
        paragraph("Each feature should be reviewed for user value, acceptance criteria, edge cases, and whether it belongs in this product boundary."),
      ])}

      ${packetSection(6, "UX / Design Proposal", [
        list([
          "Identify the primary user workflows affected by this design.",
          "Review first-screen information hierarchy, navigation, empty states, loading states, error states, success states, and conflict states.",
          "Check whether the proposed screens are operationally useful, not just visually complete.",
          "Confirm accessibility expectations: keyboard access, readable contrast, clear focus states, and screen-reader labels.",
        ]),
      ])}

      ${packetSection(7, "Implementation Plan", [
        list([
          "Phase 1: Apply the approved product area/capability/feature structure.",
          "Phase 2: Generate or refine stories and tasks with acceptance criteria and dependencies.",
          "Phase 3: Build UI/data/API changes behind reviewable stories and tasks.",
          "Phase 4: Validate with focused tests, walkthroughs, and user review before release.",
        ]),
      ])}

      ${packetSection(8, "Risk Review", [
        riskItems.length > 0
          ? list(riskItems)
          : list([
              "No blocking structural risks were detected by the current validation pass.",
              "Review assumptions manually before applying the design.",
            ]),
      ])}

      ${packetSection(9, "Work Breakdown", [
        workActions.length > 0
          ? list(workActions.map((summary) => summary.title))
          : list([
              "Convert approved features into implementation-ready stories and tasks.",
              "Add priorities, dependencies, complexity, and acceptance criteria before delivery.",
            ]),
      ])}

      ${packetSection(10, "Approval Section", [
        `<div class="section approval">`,
        `<h3>Recommended approval options</h3>`,
        list([
          "Approve all proposed changes and apply them to the product catalog.",
          "Approve selected changes only.",
          "Request revision and regenerate this packet.",
          "Export/share this packet for stakeholder review.",
          "Archive this packet without applying changes.",
        ]),
        `<h3>Structured change set</h3>`,
        `<pre>${escapeHtml(changeSetJson)}</pre>`,
        `</div>`,
      ])}
    </main>
  </body>
</html>`;
}

export function packetSection(index: number, title: string, body: string[]) {
  return `<section id="section-${index}" class="section"><h2>${index}. ${escapeHtml(title)}</h2>${body.join("\n")}</section>`;
}

export function paragraph(value: string) {
  return `<p>${escapeHtml(value)}</p>`;
}

export function list(items: string[]) {
  return `<ul>${items.map((item) => `<li>${escapeHtml(item)}</li>`).join("")}</ul>`;
}

export function buildMermaidDiagram(nodes: PlannerTreeNode[]) {
  if (nodes.length === 0) {
    return "flowchart TD\n  Empty[No staged design yet]";
  }
  const lines = ["flowchart TD"];
  const seen = new Set<string>();

  const walk = (node: PlannerTreeNode, parentId?: string) => {
    const nodeId = mermaidNodeId(node.id);
    if (!seen.has(nodeId)) {
      lines.push(`  ${nodeId}["${escapeMermaidLabel(node.label)}"]`);
      seen.add(nodeId);
    }
    if (parentId) {
      lines.push(`  ${parentId} --> ${nodeId}`);
    }
    node.children.forEach((child) => walk(child, nodeId));
  };

  nodes.forEach((node) => walk(node));
  return lines.join("\n");
}

export function mermaidNodeId(value: string) {
  const cleaned = value.replace(/[^a-zA-Z0-9_]/g, "_");
  return /^[a-zA-Z_]/.test(cleaned) ? cleaned : `node_${cleaned}`;
}

export function escapeMermaidLabel(value: string) {
  return value.replace(/"/g, "'").replace(/\n/g, " ");
}

export function escapeHtml(value: string | number | null | undefined) {
  return String(value ?? "")
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

export function slugifyPacketName(value: string) {
  return value
    .toLowerCase()
    .trim()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    || "design-review-packet";
}

export async function executePlannerPlan(plan: PlannerPlan, context: ResolverContext): Promise<ExecutionResult> {
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
