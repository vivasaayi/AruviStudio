import { describe, expect, it } from "vitest";

import {
  countDescendantNodes,
  countHierarchyNodes,
  countLeafNodes,
  findHierarchyNode,
  findHierarchyNodePath,
  getDirectChildNodes,
  getDirectWorkItemsForNode,
  getHierarchyNodeKey,
  getHierarchyNodeSectionId,
  getProductDirectWorkItems,
  getSubtreeWorkItemsForNode,
  getWorkItemOwnerKey,
  isDirectProductWorkItem,
} from "./hierarchyTree";
import type { HierarchyTreeNode, ProductTree, WorkItem } from "./types";

const capabilityNode: HierarchyTreeNode = {
  id: "capability-a",
  node_type: "capability",
  node_kind: "capability",
  product_area_id: "product_area-a",
  capability_id: "capability-a",
  parent_node_id: "product_area-a",
  parent_node_type: "product_area",
  depth: 1,
  name: "Capability A",
  description: "",
  summary: "",
  path: ["Area A", "Capability A"],
  allowed_child_kinds: ["feature"],
  children: [],
};

const productAreaNode: HierarchyTreeNode = {
  id: "product_area-a",
  node_type: "product_area",
  node_kind: "product_area",
  product_area_id: "product_area-a",
  capability_id: null,
  parent_node_id: null,
  parent_node_type: null,
  depth: 0,
  name: "Area A",
  description: "",
  summary: "",
  path: ["Area A"],
  allowed_child_kinds: ["capability"],
  children: [capabilityNode],
};

const secondProductAreaNode: HierarchyTreeNode = {
  ...productAreaNode,
  id: "product_area-b",
  product_area_id: "product_area-b",
  name: "Area B",
  path: ["Area B"],
  children: [],
};

const productTree: ProductTree = {
  product: {
    id: "product-1",
    name: "Product",
    description: "",
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
    created_at: "",
    updated_at: "",
  },
  product_areas: [],
  roots: [productAreaNode, secondProductAreaNode],
};

function makeWorkItem(overrides: Partial<WorkItem> = {}): WorkItem {
  return {
    id: "work-item",
    product_id: "product-1",
    product_area_id: null,
    capability_id: null,
    source_node_id: null,
    source_node_type: null,
    parent_work_item_id: null,
    title: "Work item",
    problem_statement: "",
    description: "",
    acceptance_criteria: "",
    constraints: "",
    work_item_type: "story",
    priority: "medium",
    complexity: "medium",
    status: "draft",
    repo_override_id: null,
    active_repo_id: null,
    branch_name: null,
    sort_order: 0,
    created_at: "2025-01-01T00:00:00Z",
    updated_at: "2025-01-01T00:00:00Z",
    ...overrides,
  };
}

describe("hierarchyTree", () => {
  it("derives stable owner keys from explicit and legacy scope fields", () => {
    expect(getWorkItemOwnerKey(makeWorkItem())).toBe("product");
    expect(getWorkItemOwnerKey(makeWorkItem({ product_area_id: "product_area-a" }))).toBe("product_area:product_area-a");
    expect(getWorkItemOwnerKey(makeWorkItem({ capability_id: "capability-a" }))).toBe(
      "capability:capability-a",
    );
    expect(
      getWorkItemOwnerKey(
        makeWorkItem({ source_node_id: "product_area-a", source_node_type: "product_area" }),
      ),
    ).toBe("product_area:product_area-a");
  });

  it("finds nodes, paths, and counts across a nested tree", () => {
    expect(getHierarchyNodeKey(productAreaNode)).toBe("product_area:product_area-a");
    expect(countHierarchyNodes(productTree.roots)).toBe(3);
    expect(countLeafNodes(productTree.roots)).toBe(2);
    expect(countDescendantNodes(productAreaNode)).toBe(1);
    expect(findHierarchyNode(productTree.roots, "capability-a", "capability")).toEqual(
      capabilityNode,
    );
    expect(findHierarchyNode(productTree.roots, "missing")).toBeNull();
    expect(
      findHierarchyNodePath(productTree.roots, "capability-a", "capability").map(
        (node) => node.id,
      ),
    ).toEqual(["product_area-a", "capability-a"]);
    expect(findHierarchyNodePath(productTree.roots, "missing")).toEqual([]);
  });

  it("groups work items by direct and subtree ownership", () => {
    const productItem = makeWorkItem({ id: "product-item" });
    const productAreaItem = makeWorkItem({
      id: "product_area-item",
      source_node_id: "product_area-a",
      source_node_type: "product_area",
    });
    const capabilityItem = makeWorkItem({
      id: "capability-item",
      source_node_id: "capability-a",
      source_node_type: "capability",
    });
    const workItems = [productItem, productAreaItem, capabilityItem];

    expect(isDirectProductWorkItem(productItem)).toBe(true);
    expect(getProductDirectWorkItems(workItems).map((item) => item.id)).toEqual(["product-item"]);
    expect(getDirectWorkItemsForNode(productAreaNode, workItems).map((item) => item.id)).toEqual([
      "product_area-item",
    ]);
    expect(getSubtreeWorkItemsForNode(productAreaNode, workItems).map((item) => item.id)).toEqual([
      "product_area-item",
      "capability-item",
    ]);
  });

  it("resolves section ids and direct child collections", () => {
    expect(getHierarchyNodeSectionId(null)).toBe("product-overview-top");
    expect(getHierarchyNodeSectionId(productAreaNode)).toBe("product-area-product_area-a");
    expect(getHierarchyNodeSectionId(capabilityNode)).toBe("capability-capability-a");
    expect(getDirectChildNodes(productTree, null).map((node) => node.id)).toEqual([
      "product_area-a",
      "product_area-b",
    ]);
    expect(getDirectChildNodes(productTree, productAreaNode).map((node) => node.id)).toEqual([
      "capability-a",
    ]);
    expect(getDirectChildNodes(undefined, productAreaNode)).toEqual([]);
  });
});
