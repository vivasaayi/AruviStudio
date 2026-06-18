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
  module_id: "module-a",
  capability_id: "capability-a",
  parent_node_id: "module-a",
  parent_node_type: "module",
  depth: 1,
  name: "Capability A",
  description: "",
  summary: "",
  path: ["Area A", "Capability A"],
  allowed_child_kinds: ["feature"],
  children: [],
};

const moduleNode: HierarchyTreeNode = {
  id: "module-a",
  node_type: "module",
  node_kind: "area",
  module_id: "module-a",
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

const secondModuleNode: HierarchyTreeNode = {
  ...moduleNode,
  id: "module-b",
  module_id: "module-b",
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
  modules: [],
  roots: [moduleNode, secondModuleNode],
};

function makeWorkItem(overrides: Partial<WorkItem> = {}): WorkItem {
  return {
    id: "work-item",
    product_id: "product-1",
    module_id: null,
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
    expect(getWorkItemOwnerKey(makeWorkItem({ module_id: "module-a" }))).toBe("module:module-a");
    expect(getWorkItemOwnerKey(makeWorkItem({ capability_id: "capability-a" }))).toBe(
      "capability:capability-a",
    );
    expect(
      getWorkItemOwnerKey(
        makeWorkItem({ source_node_id: "module-a", source_node_type: "module" }),
      ),
    ).toBe("module:module-a");
  });

  it("finds nodes, paths, and counts across a nested tree", () => {
    expect(getHierarchyNodeKey(moduleNode)).toBe("module:module-a");
    expect(countHierarchyNodes(productTree.roots)).toBe(3);
    expect(countLeafNodes(productTree.roots)).toBe(2);
    expect(countDescendantNodes(moduleNode)).toBe(1);
    expect(findHierarchyNode(productTree.roots, "capability-a", "capability")).toEqual(
      capabilityNode,
    );
    expect(findHierarchyNode(productTree.roots, "missing")).toBeNull();
    expect(
      findHierarchyNodePath(productTree.roots, "capability-a", "capability").map(
        (node) => node.id,
      ),
    ).toEqual(["module-a", "capability-a"]);
    expect(findHierarchyNodePath(productTree.roots, "missing")).toEqual([]);
  });

  it("groups work items by direct and subtree ownership", () => {
    const productItem = makeWorkItem({ id: "product-item" });
    const moduleItem = makeWorkItem({
      id: "module-item",
      source_node_id: "module-a",
      source_node_type: "module",
    });
    const capabilityItem = makeWorkItem({
      id: "capability-item",
      source_node_id: "capability-a",
      source_node_type: "capability",
    });
    const workItems = [productItem, moduleItem, capabilityItem];

    expect(isDirectProductWorkItem(productItem)).toBe(true);
    expect(getProductDirectWorkItems(workItems).map((item) => item.id)).toEqual(["product-item"]);
    expect(getDirectWorkItemsForNode(moduleNode, workItems).map((item) => item.id)).toEqual([
      "module-item",
    ]);
    expect(getSubtreeWorkItemsForNode(moduleNode, workItems).map((item) => item.id)).toEqual([
      "module-item",
      "capability-item",
    ]);
  });

  it("resolves section ids and direct child collections", () => {
    expect(getHierarchyNodeSectionId(null)).toBe("product-overview-top");
    expect(getHierarchyNodeSectionId(capabilityNode)).toBe("capability-capability-a");
    expect(getDirectChildNodes(productTree, null).map((node) => node.id)).toEqual([
      "module-a",
      "module-b",
    ]);
    expect(getDirectChildNodes(productTree, moduleNode).map((node) => node.id)).toEqual([
      "capability-a",
    ]);
    expect(getDirectChildNodes(undefined, moduleNode)).toEqual([]);
  });
});
