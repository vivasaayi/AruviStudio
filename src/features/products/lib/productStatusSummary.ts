import {
  countDescendantNodes,
  countHierarchyNodes,
  countLeafNodes,
  flattenHierarchyNodes,
  getHierarchyNodeKey,
} from "../../../lib/hierarchyTree";
import { getHierarchyNodeKindLabel } from "../../../lib/hierarchyLabels";
import type {
  HierarchyTreeNode,
  Product,
  ProductTree,
  ProductWorkItemSummary,
  WorkItemScopeSummary,
} from "../../../lib/types";

export interface ProgressSummary {
  total: number;
  done: number;
  percent: number;
}

export interface ProductStatusSummary {
  productCount: number;
  nodeCount: number;
  leafCount: number;
  workItemCount: number;
  activeWorkItemCount: number;
  doneWorkItemCount: number;
  progress: ProgressSummary;
}

export interface StatusRow {
  id: string;
  productId: string | null;
  nodeId?: string;
  nodeType?: HierarchyTreeNode["node_type"];
  productAreaId?: string;
  capabilityId?: string | null;
  level: number;
  name: string;
  subtitle: string;
  kind: string;
  childCount: number;
  nodeCount: number;
  workItemCount: number;
  activeWorkItemCount: number;
  progress: ProgressSummary;
}

export interface WorkItemCountSummary {
  total: number;
  topLevel: number;
  active: number;
  done: number;
  blocked: number;
}

export interface WorkItemScopeSummaryIndex {
  rows: WorkItemScopeSummary[];
  byOwner: Map<string, WorkItemCountSummary>;
  byProduct: Map<string, WorkItemCountSummary>;
  byStatus: Map<string, WorkItemCountSummary>;
}

export function getProgressSummaryFromCounts(total: number, done: number): ProgressSummary {
  return {
    total,
    done,
    percent: total > 0 ? Math.round((done / total) * 100) : 0,
  };
}

export function buildProductStatusSummary(
  products: Product[],
  productTreeById: Map<string, ProductTree>,
  productSummaryById: Map<string, ProductWorkItemSummary>,
): ProductStatusSummary {
  const totals = products.reduce(
    (summary, product) => addProductSummaryCounts(summary, productSummaryById.get(product.id)),
    emptyWorkItemCountSummary(),
  );
  return {
    productCount: products.length,
    nodeCount: products.reduce((total, product) => total + countHierarchyNodes(productTreeById.get(product.id)?.roots ?? []), 0),
    leafCount: products.reduce((total, product) => total + countLeafNodes(productTreeById.get(product.id)?.roots ?? []), 0),
    workItemCount: totals.total,
    activeWorkItemCount: totals.active,
    doneWorkItemCount: totals.done,
    progress: getProgressSummaryFromCounts(totals.total, totals.done),
  };
}

export function buildStatusRows(
  products: Product[],
  productTreeById: Map<string, ProductTree>,
  scopeSummaryIndex: WorkItemScopeSummaryIndex,
  maxDepth: number,
  groupBy: "node" | "kind" | "work_status",
): StatusRow[] {
  if (groupBy === "kind") {
    return buildKindPivotRows(products, productTreeById, scopeSummaryIndex, maxDepth);
  }
  if (groupBy === "work_status") {
    return buildWorkStatusPivotRows(products, scopeSummaryIndex);
  }

  const rows: StatusRow[] = [];
  const includeProductRows = products.length !== 1;
  products.forEach((product) => {
    const tree = productTreeById.get(product.id);
    const productCounts = scopeSummaryIndex.byProduct.get(product.id) ?? emptyWorkItemCountSummary();
    if (includeProductRows || !tree?.roots.length) {
      rows.push({
        id: `product:${product.id}`,
        productId: product.id,
        level: 0,
        name: product.name,
        subtitle: product.description || product.vision || "Product summary",
        kind: "Product",
        childCount: tree?.roots.length ?? 0,
        nodeCount: tree ? countHierarchyNodes(tree.roots) : 0,
        workItemCount: productCounts.total,
        activeWorkItemCount: productCounts.active,
        progress: getProgressSummaryFromCounts(productCounts.total, productCounts.done),
      });
    }
    if (tree) {
      tree.roots.forEach((node) => pushNodeStatusRows(rows, product, node, scopeSummaryIndex, maxDepth));
    }
  });
  return rows;
}

function pushNodeStatusRows(
  rows: StatusRow[],
  product: Product,
  node: HierarchyTreeNode,
  scopeSummaryIndex: WorkItemScopeSummaryIndex,
  maxDepth: number,
) {
  const level = node.depth + 1;
  if (level > maxDepth) {
    return;
  }
  const subtreeCounts = getSubtreeWorkItemCounts(node, product.id, scopeSummaryIndex);
  rows.push({
    id: `${product.id}:${node.node_type}:${node.id}`,
    productId: product.id,
    nodeId: node.id,
    nodeType: node.node_type,
    productAreaId: node.product_area_id,
    capabilityId: node.capability_id,
    level,
    name: node.name,
    subtitle: node.path.join(" / ") || product.name,
    kind: getHierarchyNodeKindLabel(node.node_kind),
    childCount: node.children.length,
    nodeCount: countDescendantNodes(node) + 1,
    workItemCount: subtreeCounts.total,
    activeWorkItemCount: subtreeCounts.active,
    progress: getProgressSummaryFromCounts(subtreeCounts.total, subtreeCounts.done),
  });
  node.children.forEach((child) => pushNodeStatusRows(rows, product, child, scopeSummaryIndex, maxDepth));
}

function buildKindPivotRows(
  products: Product[],
  productTreeById: Map<string, ProductTree>,
  scopeSummaryIndex: WorkItemScopeSummaryIndex,
  maxDepth: number,
): StatusRow[] {
  const groups = new Map<string, StatusRow>();
  products.forEach((product) => {
    const tree = productTreeById.get(product.id);
    (tree?.roots ?? []).forEach((node) => collectKindPivot(node, product, scopeSummaryIndex, maxDepth, groups));
  });
  return Array.from(groups.values()).sort((a, b) => a.kind.localeCompare(b.kind));
}

function collectKindPivot(
  node: HierarchyTreeNode,
  product: Product,
  scopeSummaryIndex: WorkItemScopeSummaryIndex,
  maxDepth: number,
  groups: Map<string, StatusRow>,
) {
  const level = node.depth + 1;
  if (level > maxDepth) {
    return;
  }
  const kind = getHierarchyNodeKindLabel(node.node_kind);
  const directCounts = getDirectWorkItemCounts(node, product.id, scopeSummaryIndex);
  const existing = groups.get(kind) ?? {
    id: `kind:${kind}`,
    productId: product.id,
    level: 0,
    name: kind,
    subtitle: "Pivoted across matching node kinds",
    kind,
    childCount: 0,
    nodeCount: 0,
    workItemCount: 0,
    activeWorkItemCount: 0,
    progress: getProgressSummaryFromCounts(0, 0),
  };
  const nextTotal = existing.workItemCount + directCounts.total;
  const nextDone = existing.progress.done + directCounts.done;
  groups.set(kind, {
    ...existing,
    childCount: existing.childCount + node.children.length,
    nodeCount: existing.nodeCount + 1,
    workItemCount: nextTotal,
    activeWorkItemCount: existing.activeWorkItemCount + directCounts.active,
    progress: getProgressSummaryFromCounts(nextTotal, nextDone),
  });
  node.children.forEach((child) => collectKindPivot(child, product, scopeSummaryIndex, maxDepth, groups));
}

function buildWorkStatusPivotRows(
  products: Product[],
  scopeSummaryIndex: WorkItemScopeSummaryIndex,
): StatusRow[] {
  const productIds = new Set(products.map((product) => product.id));
  const groups = new Map<string, WorkItemCountSummary>();
  scopeSummaryIndex.rows
    .filter((summary) => productIds.has(summary.product_id))
    .forEach((summary) => {
      groups.set(summary.status, addScopeSummaryCounts(groups.get(summary.status), summary));
    });

  return Array.from(groups.entries())
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([status, counts]) => ({
      id: `work-status:${status}`,
      productId: products.length === 1 ? products[0].id : null,
      level: 0,
      name: status.replace(/_/g, " "),
      subtitle: "Pivoted across stories with this status",
      kind: "Work Status",
      childCount: 0,
      nodeCount: 0,
      workItemCount: counts.total,
      activeWorkItemCount: counts.active,
      progress: getProgressSummaryFromCounts(counts.total, counts.done),
    }));
}

export function buildWorkItemScopeSummaryIndex(summaries: WorkItemScopeSummary[]): WorkItemScopeSummaryIndex {
  const byOwner = new Map<string, WorkItemCountSummary>();
  const byProduct = new Map<string, WorkItemCountSummary>();
  const byStatus = new Map<string, WorkItemCountSummary>();

  summaries.forEach((summary) => {
    const ownerKey = getScopeSummaryOwnerKey(summary);
    byOwner.set(ownerKey, addScopeSummaryCounts(byOwner.get(ownerKey), summary));
    byProduct.set(summary.product_id, addScopeSummaryCounts(byProduct.get(summary.product_id), summary));
    byStatus.set(summary.status, addScopeSummaryCounts(byStatus.get(summary.status), summary));
  });

  return { rows: summaries, byOwner, byProduct, byStatus };
}

function getScopeSummaryOwnerKey(summary: WorkItemScopeSummary) {
  if (summary.source_node_id && summary.source_node_type) {
    return `${summary.product_id}:${summary.source_node_type}:${summary.source_node_id}`;
  }
  if (summary.capability_id) {
    return `${summary.product_id}:capability:${summary.capability_id}`;
  }
  if (summary.product_area_id) {
    return `${summary.product_id}:product_area:${summary.product_area_id}`;
  }
  return `${summary.product_id}:product`;
}

export function getDirectWorkItemCounts(
  node: HierarchyTreeNode,
  productId: string,
  scopeSummaryIndex: WorkItemScopeSummaryIndex,
): WorkItemCountSummary {
  return scopeSummaryIndex.byOwner.get(`${productId}:${getHierarchyNodeKey(node)}`) ?? emptyWorkItemCountSummary();
}

export function getSubtreeWorkItemCounts(
  node: HierarchyTreeNode,
  productId: string,
  scopeSummaryIndex: WorkItemScopeSummaryIndex,
): WorkItemCountSummary {
  return flattenHierarchyNodes([node]).reduce(
    (summary, child) => addCountSummaries(summary, getDirectWorkItemCounts(child, productId, scopeSummaryIndex)),
    emptyWorkItemCountSummary(),
  );
}

function addProductSummaryCounts(
  current: WorkItemCountSummary,
  summary: ProductWorkItemSummary | undefined,
): WorkItemCountSummary {
  if (!summary) {
    return current;
  }
  return {
    total: current.total + summary.total_count,
    topLevel: current.topLevel + summary.total_count,
    active: current.active + summary.active_count,
    done: current.done + summary.done_count,
    blocked: current.blocked + summary.blocked_count,
  };
}

function addScopeSummaryCounts(
  current: WorkItemCountSummary | undefined,
  summary: WorkItemScopeSummary,
): WorkItemCountSummary {
  return addCountSummaries(current ?? emptyWorkItemCountSummary(), {
    total: summary.total_count,
    topLevel: summary.top_level_count,
    active: summary.active_count,
    done: summary.done_count,
    blocked: summary.blocked_count,
  });
}

function addCountSummaries(left: WorkItemCountSummary, right: WorkItemCountSummary): WorkItemCountSummary {
  return {
    total: left.total + right.total,
    topLevel: left.topLevel + right.topLevel,
    active: left.active + right.active,
    done: left.done + right.done,
    blocked: left.blocked + right.blocked,
  };
}

function emptyWorkItemCountSummary(): WorkItemCountSummary {
  return {
    total: 0,
    topLevel: 0,
    active: 0,
    done: 0,
    blocked: 0,
  };
}
