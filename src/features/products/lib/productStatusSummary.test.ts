import { describe, expect, it } from "vitest";

import {
  buildProductStatusSummary,
  buildStatusRows,
  buildWorkItemScopeSummaryIndex,
  getDirectWorkItemCounts,
  getSubtreeWorkItemCounts,
} from "./productStatusSummary";
import type {
  HierarchyNodeKind,
  HierarchyNodeType,
  HierarchyTreeNode,
  Product,
  ProductTree,
  ProductWorkItemSummary,
  WorkItem,
  WorkItemScopeSummary,
} from "../../../lib/types";

function product(overrides: Partial<Product> = {}): Product {
  return {
    id: "product-1",
    name: "Mayyam",
    description: "Operations platform",
    vision: "",
    goals: [],
    tags: [],
    status: "active",
    lifecycle: "active",
    health: "healthy",
    owner_label: "",
    investment_status: "invest",
    roadmap: "",
    evidence: "",
    created_at: "2026-01-01T00:00:00Z",
    updated_at: "2026-01-01T00:00:00Z",
    ...overrides,
  };
}

function node(overrides: Partial<HierarchyTreeNode>): HierarchyTreeNode {
  const nodeKind = overrides.node_kind ?? "capability";
  const nodeType = overrides.node_type ?? (nodeKind === "product_area" ? "product_area" : "capability");
  const id = overrides.id ?? nodeKind;

  return {
    id,
    node_type: nodeType,
    node_kind: nodeKind,
    product_area_id: overrides.product_area_id ?? (nodeType === "product_area" ? id : "area-1"),
    capability_id: overrides.capability_id ?? (nodeType === "capability" ? id : null),
    parent_node_id: null,
    parent_node_type: null,
    depth: 0,
    name: id,
    description: "",
    summary: "",
    path: [id],
    allowed_child_kinds: [],
    children: [],
    ...overrides,
  };
}

function tree(productModel: Product, roots: HierarchyTreeNode[]): ProductTree {
  return {
    product: productModel,
    product_areas: [],
    roots,
  };
}

function scopeSummary(overrides: Partial<WorkItemScopeSummary>): WorkItemScopeSummary {
  return {
    product_id: "product-1",
    product_area_id: null,
    capability_id: null,
    source_node_id: null,
    source_node_type: null,
    status: "in_progress" as WorkItem["status"],
    total_count: 0,
    top_level_count: 0,
    active_count: 0,
    done_count: 0,
    blocked_count: 0,
    ...overrides,
  };
}

describe("productStatusSummary", () => {
  it("builds product status totals from aggregate product summaries and tree counts", () => {
    const mayyam = product({ id: "product-1", name: "Mayyam" });
    const platform = product({ id: "product-2", name: "Platform" });
    const feature = node({ id: "feature-1", node_kind: "feature", depth: 2, path: ["Area", "Capability", "Feature"] });
    const capability = node({ id: "capability-1", node_kind: "capability", depth: 1, path: ["Area", "Capability"], children: [feature] });
    const area = node({ id: "area-1", node_kind: "product_area", depth: 0, path: ["Area"], children: [capability] });
    const productTreeById = new Map<string, ProductTree>([
      [mayyam.id, tree(mayyam, [area])],
    ]);
    const productSummaryById = new Map<string, ProductWorkItemSummary>([
      [mayyam.id, { product_id: mayyam.id, total_count: 10, active_count: 6, done_count: 4, blocked_count: 1 }],
      [platform.id, { product_id: platform.id, total_count: 5, active_count: 4, done_count: 1, blocked_count: 0 }],
    ]);

    expect(buildProductStatusSummary([mayyam, platform], productTreeById, productSummaryById)).toEqual({
      productCount: 2,
      nodeCount: 3,
      leafCount: 1,
      workItemCount: 15,
      activeWorkItemCount: 10,
      doneWorkItemCount: 5,
      progress: { total: 15, done: 5, percent: 33 },
    });
  });

  it("builds node status rows from aggregate scope summaries without work item rows", () => {
    const mayyam = product();
    const feature = node({ id: "feature-1", node_kind: "feature", depth: 2, path: ["Area", "Capability", "Feature"] });
    const capability = node({ id: "capability-1", node_kind: "capability", depth: 1, path: ["Area", "Capability"], children: [feature] });
    const area = node({ id: "area-1", node_kind: "product_area", depth: 0, path: ["Area"], children: [capability] });
    const index = buildWorkItemScopeSummaryIndex([
      scopeSummary({
        source_node_id: "area-1",
        source_node_type: "product_area" as HierarchyNodeType,
        status: "in_progress",
        total_count: 3,
        top_level_count: 2,
        active_count: 2,
        done_count: 1,
      }),
      scopeSummary({
        source_node_id: "feature-1",
        source_node_type: "capability" as HierarchyNodeType,
        capability_id: "feature-1",
        status: "done",
        total_count: 4,
        top_level_count: 4,
        active_count: 0,
        done_count: 4,
      }),
    ]);

    const rows = buildStatusRows(
      [mayyam],
      new Map([[mayyam.id, tree(mayyam, [area])]]),
      index,
      3,
      "node",
    );

    expect(rows.map((row) => row.name)).toEqual(["area-1", "capability-1", "feature-1"]);
    expect(rows[0]).toMatchObject({
      id: "product-1:product_area:area-1",
      kind: "Product Area",
      level: 1,
      childCount: 1,
      nodeCount: 3,
      workItemCount: 7,
      activeWorkItemCount: 2,
      progress: { total: 7, done: 5, percent: 71 },
    });
    expect(rows[2]).toMatchObject({
      kind: "Feature",
      workItemCount: 4,
      activeWorkItemCount: 0,
      progress: { total: 4, done: 4, percent: 100 },
    });
  });

  it("indexes direct and subtree counts by canonical product area and capability ownership", () => {
    const feature = node({ id: "feature-1", node_kind: "feature", depth: 1 });
    const area = node({ id: "area-1", node_kind: "product_area", children: [feature] });
    const index = buildWorkItemScopeSummaryIndex([
      scopeSummary({ product_area_id: "area-1", total_count: 2, top_level_count: 2, active_count: 1, done_count: 1 }),
      scopeSummary({ capability_id: "feature-1", total_count: 3, top_level_count: 3, active_count: 2, done_count: 1 }),
    ]);

    expect(getDirectWorkItemCounts(area, "product-1", index)).toMatchObject({ total: 2, topLevel: 2 });
    expect(getSubtreeWorkItemCounts(area, "product-1", index)).toMatchObject({ total: 5, topLevel: 5, active: 3, done: 2 });
  });

  it("builds kind and work-status pivots from aggregate summaries", () => {
    const mayyam = product();
    const feature = node({ id: "feature-1", node_kind: "feature" as HierarchyNodeKind, depth: 0 });
    const index = buildWorkItemScopeSummaryIndex([
      scopeSummary({ source_node_id: "feature-1", source_node_type: "capability", status: "in_progress", total_count: 6, top_level_count: 6, active_count: 6 }),
      scopeSummary({ source_node_id: "feature-1", source_node_type: "capability", status: "done", total_count: 4, top_level_count: 4, done_count: 4 }),
      scopeSummary({ product_id: "other-product", status: "blocked", total_count: 100, top_level_count: 100, blocked_count: 100 }),
    ]);
    const productTreeById = new Map([[mayyam.id, tree(mayyam, [feature])]]);

    expect(buildStatusRows([mayyam], productTreeById, index, 1, "kind")).toMatchObject([
      {
        name: "Feature",
        workItemCount: 10,
        activeWorkItemCount: 6,
        progress: { total: 10, done: 4, percent: 40 },
      },
    ]);
    expect(buildStatusRows([mayyam], productTreeById, index, 1, "work_status").map((row) => ({
      name: row.name,
      workItemCount: row.workItemCount,
    }))).toEqual([
      { name: "done", workItemCount: 4 },
      { name: "in progress", workItemCount: 6 },
    ]);
  });
});
