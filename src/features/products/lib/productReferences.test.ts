import { describe, expect, it } from "vitest";

import {
  filterReferencesForProductBook,
  filterReferencesForScope,
  getCapabilityReferenceScope,
  getProductAreaReferenceScope,
  getReferenceKindLabel,
} from "./productReferences";
import type { Capability, ProductReference, ProductTree } from "../../../lib/types";

function makeReference(overrides: Partial<ProductReference> = {}): ProductReference {
  return {
    id: "reference-1",
    scope_type: "product",
    scope_id: "product-1",
    title: "Reference",
    reference_kind: "external_doc",
    uri: "",
    content: "",
    created_at: "",
    updated_at: "",
    ...overrides,
  };
}

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
  roots: [
    {
      id: "product_area-1",
      node_type: "product_area",
      node_kind: "product_area",
      product_area_id: "product_area-1",
      capability_id: null,
      parent_node_id: null,
      parent_node_type: null,
      depth: 0,
      name: "Area One",
      description: "",
      summary: "",
      path: ["Area One"],
      allowed_child_kinds: ["capability"],
      children: [
        {
          id: "capability-1",
          node_type: "capability",
          node_kind: "capability",
          product_area_id: "product_area-1",
          capability_id: "capability-1",
          parent_node_id: "product_area-1",
          parent_node_type: "product_area",
          depth: 1,
          name: "Capability One",
          description: "",
          summary: "",
          path: ["Area One", "Capability One"],
          allowed_child_kinds: ["feature"],
          children: [
            {
              id: "feature-1",
              node_type: "capability",
              node_kind: "feature",
              product_area_id: "product_area-1",
              capability_id: "feature-1",
              parent_node_id: "capability-1",
              parent_node_type: "capability",
              depth: 2,
              name: "Feature One",
              description: "",
              summary: "",
              path: ["Area One", "Capability One", "Feature One"],
              allowed_child_kinds: [],
              children: [],
            },
          ],
        },
      ],
    },
  ],
};

describe("productReferences", () => {
  it("derives product-area and capability scopes from frontend entities", () => {
    const capability: Capability = {
      id: "capability-1",
      product_area_id: "product_area-1",
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
      created_at: "",
      updated_at: "",
    };

    expect(getProductAreaReferenceScope("product_area-1")).toEqual({
      scopeType: "product_area",
      scopeId: "product_area-1",
    });
    expect(getCapabilityReferenceScope(capability)).toEqual({
      scopeType: "capability",
      scopeId: "capability-1",
    });
    expect(getCapabilityReferenceScope({ ...capability, id: "feature-1", node_kind: "feature" })).toEqual({
      scopeType: "feature",
      scopeId: "feature-1",
    });
  });

  it("filters references for exact scopes and product-book inclusion", () => {
    const references = [
      makeReference({ id: "product-ref", scope_type: "product", scope_id: "product-1" }),
      makeReference({ id: "product_area-ref", scope_type: "product_area", scope_id: "product_area-1" }),
      makeReference({ id: "cap-ref", scope_type: "capability", scope_id: "capability-1" }),
      makeReference({ id: "feature-ref", scope_type: "feature", scope_id: "feature-1" }),
      makeReference({ id: "other-product-ref", scope_type: "product", scope_id: "product-2" }),
    ];

    expect(
      filterReferencesForScope(references, { scopeType: "product_area", scopeId: "product_area-1" }).map(
        (reference) => reference.id,
      ),
    ).toEqual(["product_area-ref"]);
    expect(filterReferencesForProductBook("product-1", productTree, references).map((reference) => reference.id)).toEqual([
      "product-ref",
      "product_area-ref",
      "cap-ref",
      "feature-ref",
    ]);
  });

  it("formats human-readable labels from enum-like kinds", () => {
    expect(getReferenceKindLabel("customer_evidence")).toBe("Customer Evidence");
    expect(getReferenceKindLabel("external_doc")).toBe("External Doc");
  });
});
