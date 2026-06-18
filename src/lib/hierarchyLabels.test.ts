import { describe, expect, it } from "vitest";

import {
  getAllowedChildNodeKinds,
  getCapabilityChildLabel,
  getCapabilityHierarchyLabel,
  getDefaultChildNodeKind,
  getHierarchyChildLabel,
  getHierarchyNodeKindGuidance,
  getHierarchyNodeKindLabel,
  groupHierarchyNodeKinds,
  isCapabilityFeatureLevel,
  orderHierarchyNodeKinds,
  supportsHierarchyChildren,
} from "./hierarchyLabels";

describe("hierarchyLabels", () => {
  it("orders and groups known node kinds using display semantics", () => {
    expect(orderHierarchyNodeKinds(["feature", "area"])).toEqual(["area", "feature"]);
    expect(groupHierarchyNodeKinds(["capability", "feature"])).toEqual([
      {
        label: "Product Management",
        kinds: ["capability", "feature"],
      },
    ]);
  });

  it("renders labels and guidance for known and fallback node kinds", () => {
    expect(getHierarchyNodeKindLabel("area")).toBe("Product Area");
    expect(getHierarchyNodeKindLabel("feature", { plural: true, lowercase: true })).toBe(
      "features",
    );
    expect(getHierarchyNodeKindLabel(null, { plural: true })).toBe("Nodes");
    expect(getHierarchyNodeKindGuidance("capability")).toContain("must be able to do");
    expect(getHierarchyNodeKindGuidance(null)).toContain("Product Area");
  });

  it("describes child semantics by parent node kind", () => {
    expect(supportsHierarchyChildren("area")).toBe(true);
    expect(supportsHierarchyChildren("feature")).toBe(false);
    expect(getAllowedChildNodeKinds("area")).toEqual(["capability"]);
    expect(getAllowedChildNodeKinds("capability")).toEqual(["feature"]);
    expect(getAllowedChildNodeKinds(undefined)).toEqual(["area"]);
    expect(getDefaultChildNodeKind("capability")).toBe("feature");
    expect(getDefaultChildNodeKind(undefined)).toBe("area");
    expect(getHierarchyChildLabel("area")).toBe("Capability");
    expect(getHierarchyChildLabel(undefined, { plural: true, lowercase: true })).toBe(
      "product areas",
    );
  });

  it("keeps legacy numeric capability levels aligned with node-kind labels", () => {
    expect(getCapabilityHierarchyLabel(0)).toBe("Capability");
    expect(getCapabilityHierarchyLabel(2, { lowercase: true })).toBe("feature");
    expect(getCapabilityChildLabel(0, { plural: true })).toBe("Features");
    expect(getCapabilityChildLabel("feature")).toBe("Child");
    expect(isCapabilityFeatureLevel(0)).toBe(false);
    expect(isCapabilityFeatureLevel(1)).toBe(true);
    expect(isCapabilityFeatureLevel("feature")).toBe(true);
  });
});
