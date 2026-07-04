import { describe, expect, it } from "vitest";

import type { ProductStrategyLink, StrategyNode, StrategyNodeKind } from "../../../lib/types";
import {
  buildStrategyTree,
  collectDescendantIds,
  collectStrategySubtreeIds,
  countProductsForStrategy,
  findTreeNode,
  getChildKind,
} from "./portfolioStrategyTree";

const node = (
  id: string,
  parentNodeId: string | null,
  nodeKind: StrategyNodeKind,
  name: string,
  sortOrder: number,
): StrategyNode => ({
  id,
  parent_node_id: parentNodeId,
  node_kind: nodeKind,
  name,
  description: "",
  owner_label: "",
  sort_order: sortOrder,
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-01T00:00:00Z",
});

const link = (id: string, productId: string, strategyNodeId: string): ProductStrategyLink => ({
  id,
  product_id: productId,
  strategy_node_id: strategyNodeId,
  is_primary: true,
  created_at: "2026-01-01T00:00:00Z",
});

describe("portfolioStrategyTree", () => {
  it("builds a sorted hierarchy from flat strategy nodes", () => {
    const tree = buildStrategyTree([
      node("domain-b", "root", "domain", "Beta", 2),
      node("sub-a", "domain-a", "sub_domain", "Checkout", 1),
      node("root", null, "strategic_product_area", "Commerce", 1),
      node("domain-a", "root", "domain", "Alpha", 1),
    ]);

    expect(tree.map((item) => item.id)).toEqual(["root"]);
    expect(tree[0].children.map((item) => item.id)).toEqual(["domain-a", "domain-b"]);
    expect(tree[0].children[0].children.map((item) => item.id)).toEqual(["sub-a"]);
  });

  it("collects subtree and flat descendant ids", () => {
    const nodes = [
      node("root", null, "strategic_product_area", "Commerce", 1),
      node("domain-a", "root", "domain", "Alpha", 1),
      node("sub-a", "domain-a", "sub_domain", "Checkout", 1),
      node("domain-b", "root", "domain", "Beta", 2),
    ];
    const tree = buildStrategyTree(nodes);

    expect(collectStrategySubtreeIds(tree, "domain-a")).toEqual(["domain-a", "sub-a"]);
    expect(collectDescendantIds(nodes, "root")).toEqual(["domain-a", "sub-a", "domain-b"]);
  });

  it("counts products linked anywhere in a strategy branch", () => {
    const tree = buildStrategyTree([
      node("root", null, "strategic_product_area", "Commerce", 1),
      node("domain-a", "root", "domain", "Alpha", 1),
      node("sub-a", "domain-a", "sub_domain", "Checkout", 1),
    ]);

    expect(countProductsForStrategy(findTreeNode(tree, "root"), [
      link("link-1", "product-1", "root"),
      link("link-2", "product-2", "sub-a"),
      link("link-3", "product-3", "elsewhere"),
    ])).toBe(2);
  });

  it("maps allowed child node kinds", () => {
    expect(getChildKind("strategic_product_area")).toBe("domain");
    expect(getChildKind("domain")).toBe("sub_domain");
    expect(getChildKind("sub_domain")).toBeNull();
  });
});
