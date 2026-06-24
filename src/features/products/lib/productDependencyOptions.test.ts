import { describe, expect, it } from "vitest";

import {
  buildCapabilityLabelById,
  buildCapabilityOptionsFromNodes,
  buildDependencyTargetCapabilityOptions,
} from "./productDependencyOptions";
import type { HierarchyTreeNode, Product, ProductTree } from "../../../lib/types";

function node(
  id: string,
  name: string,
  nodeType: HierarchyTreeNode["node_type"],
  children: HierarchyTreeNode[] = [],
): HierarchyTreeNode {
  return {
    id,
    name,
    node_type: nodeType,
    node_kind: nodeType === "product_area" ? "product_area" : "capability",
    path: ["Product", name],
    depth: 1,
    product_area_id: nodeType === "product_area" ? id : "area-1",
    capability_id: nodeType === "capability" ? id : null,
    parent_node_id: null,
    parent_node_type: null,
    description: "",
    summary: "",
    allowed_child_kinds: [],
    children,
  };
}

function product(id: string): Product {
  return {
    id,
    name: `Product ${id}`,
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
    created_at: "2026-01-01T00:00:00Z",
    updated_at: "2026-01-01T00:00:00Z",
  };
}

function productTree(id: string, roots: HierarchyTreeNode[]): ProductTree {
  return {
    product: product(id),
    product_areas: [],
    roots,
  };
}

describe("productDependencyOptions", () => {
  it("builds capability options from flattened hierarchy nodes", () => {
    expect(buildCapabilityOptionsFromNodes([
      node("area-1", "Commerce", "product_area"),
      node("capability-1", "Checkout", "capability"),
    ])).toEqual([
      { id: "capability-1", label: "Product / Checkout" },
    ]);
  });

  it("builds target capability options for the selected dependency product", () => {
    const targetTree = productTree("product-2", [
      node("area-2", "Platform", "product_area", [
        node("capability-2", "Identity", "capability"),
      ]),
    ]);
    const productTreeById = new Map([["product-2", targetTree]]);

    expect(buildDependencyTargetCapabilityOptions(productTreeById, "product-2")).toEqual([
      { id: "capability-2", label: "Product / Identity" },
    ]);
    expect(buildDependencyTargetCapabilityOptions(productTreeById, "missing")).toEqual([]);
  });

  it("builds capability labels with current tree nodes taking precedence", () => {
    const productTreeById = new Map([
      ["product-1", productTree("product-1", [node("capability-1", "Old Checkout", "capability")])],
    ]);
    const labels = buildCapabilityLabelById(productTreeById, [
      node("capability-1", "Checkout", "capability"),
      node("capability-2", "Payments", "capability"),
    ]);

    expect(labels.get("capability-1")).toBe("Product / Checkout");
    expect(labels.get("capability-2")).toBe("Product / Payments");
  });
});
