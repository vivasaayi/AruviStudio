import { describe, expect, it } from "vitest";

import {
  countCapabilities,
  findCapabilityTree,
  flattenCapabilityTreeList,
  getCapabilityOrderKey,
  getHierarchyDeleteLabel,
  getOrderedCapabilityTrees,
  orderItemsByIds,
  seedCapabilityOrderMap,
} from "./productHierarchyHelpers";
import type { Capability, CapabilityTree, ProductArea, ProductAreaTree } from "../../../lib/types";

function productArea(overrides: Partial<ProductArea> = {}): ProductArea {
  return {
    id: "area-1",
    product_id: "product-1",
    node_kind: "product_area",
    name: "Area",
    description: "",
    purpose: "",
    explanation: "",
    examples: "",
    implementation_notes: "",
    test_guidance: "",
    sort_order: 0,
    created_at: "2026-01-01T00:00:00Z",
    updated_at: "2026-01-01T00:00:00Z",
    ...overrides,
  };
}

function capability(overrides: Partial<Capability> = {}): Capability {
  return {
    id: "capability-1",
    product_area_id: "area-1",
    parent_capability_id: null,
    level: 0,
    node_kind: "capability",
    sort_order: 0,
    name: "Capability",
    description: "",
    acceptance_criteria: "",
    explanation: "",
    examples: "",
    priority: "medium",
    risk: "low",
    status: "draft",
    technical_notes: "",
    implementation_notes: "",
    test_guidance: "",
    created_at: "2026-01-01T00:00:00Z",
    updated_at: "2026-01-01T00:00:00Z",
    ...overrides,
  };
}

function capabilityTree(id: string, children: CapabilityTree[] = []): CapabilityTree {
  return {
    capability: capability({ id, name: id }),
    children,
  };
}

function productAreaTree(features: CapabilityTree[]): ProductAreaTree {
  return {
    product_area: productArea(),
    features,
  };
}

describe("productHierarchyHelpers", () => {
  it("counts nested capabilities across product areas", () => {
    expect(countCapabilities([
      productAreaTree([
        capabilityTree("capability-1", [
          capabilityTree("feature-1"),
          capabilityTree("feature-2"),
        ]),
      ]),
      productAreaTree([
        capabilityTree("capability-2"),
      ]),
    ])).toBe(4);
  });

  it("finds and flattens nested capability trees in preorder", () => {
    const feature = capabilityTree("feature-1");
    const capabilityNode = capabilityTree("capability-1", [feature]);
    const productAreas = [productAreaTree([capabilityNode])];

    expect(findCapabilityTree(productAreas, "feature-1")).toBe(feature);
    expect(findCapabilityTree(productAreas, "missing")).toBeNull();
    expect(findCapabilityTree(productAreas, null)).toBeNull();
    expect(flattenCapabilityTreeList([capabilityNode]).map((node) => node.capability.id)).toEqual([
      "capability-1",
      "feature-1",
    ]);
  });

  it("orders capability trees by persisted order ids while preserving unknown tail order", () => {
    const first = capabilityTree("first");
    const second = capabilityTree("second");
    const third = capabilityTree("third");

    expect(getOrderedCapabilityTrees([first, second, third], ["third", "first"]).map((node) => node.capability.id)).toEqual([
      "third",
      "first",
      "second",
    ]);
    expect(getOrderedCapabilityTrees([first, second])).toEqual([first, second]);
  });

  it("seeds nested order map keys by product area and parent capability", () => {
    const target: Record<string, string[]> = {};
    seedCapabilityOrderMap(target, [
      capabilityTree("capability-1", [
        capabilityTree("feature-1"),
        capabilityTree("feature-2"),
      ]),
    ]);

    expect(getCapabilityOrderKey("area-1", null)).toBe("area-1:root");
    expect(target).toEqual({
      "area-1:capability-1": ["feature-1", "feature-2"],
      "area-1:feature-1": [],
      "area-1:feature-2": [],
    });
  });

  it("orders generic items and maps delete labels", () => {
    expect(orderItemsByIds([{ id: "b" }, { id: "a" }], ["a", "b"], (item) => item.id)).toEqual([
      { id: "a" },
      { id: "b" },
    ]);
    expect(getHierarchyDeleteLabel("product_area")).toBe("Product Area");
    expect(getHierarchyDeleteLabel("capability")).toBe("Capability");
    expect(getHierarchyDeleteLabel("feature")).toBe("Feature");
  });
});
