import type { PlannerDraftChildType } from "../../../lib/types";
import type { PlannerPlan, PlannerTreeNode } from "./plannerPageTypes";

export function parseDraftNodeType(meta?: string | null) {
  if (!meta) {
    return "node";
  }
  if (meta.includes("product area") || meta.includes("product_area")) {
    return "product area";
  }
  if (meta.includes("product")) {
    return "product";
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
