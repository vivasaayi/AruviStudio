import { describe, expect, it } from "vitest";

import {
  PRODUCT_DELIVERY_ID,
  PRODUCT_OVERVIEW_TOP_ID,
  buildProductOverviewToc,
  buildScopedWorkItemTree,
  buildWorkItemMetrics,
  countCapabilities,
  countCapabilityTree,
  getCapabilitySectionId,
  getModuleSectionId,
  getWorkItemPresentation,
  sortWorkItems,
} from "./productOverview";
import type { Capability, CapabilityTree, Module, ModuleTree, ProductTree, WorkItem } from "../../../lib/types";

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

function makeCapability(overrides: Partial<Capability> = {}): Capability {
  return {
    id: "capability-1",
    module_id: "module-1",
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
    ...overrides,
  };
}

const moduleRecord: Module = {
  id: "module-1",
  product_id: "product-1",
  node_kind: "area",
  name: "Area One",
  description: "",
  purpose: "",
  explanation: "",
  examples: "",
  implementation_notes: "",
  test_guidance: "",
  sort_order: 0,
  created_at: "",
  updated_at: "",
};

const nestedCapabilityTree: CapabilityTree = {
  capability: makeCapability({ id: "capability-root" }),
  children: [
    {
      capability: makeCapability({
        id: "capability-child",
        parent_capability_id: "capability-root",
        level: 1,
        node_kind: "feature",
      }),
      children: [],
    },
  ],
};

const moduleTree: ModuleTree = {
  module: moduleRecord,
  features: [nestedCapabilityTree],
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
  modules: [moduleTree],
  roots: [
    {
      id: "module-1",
      node_type: "module",
      node_kind: "area",
      module_id: "module-1",
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
          id: "capability-root",
          node_type: "capability",
          node_kind: "capability",
          module_id: "module-1",
          capability_id: "capability-root",
          parent_node_id: "module-1",
          parent_node_type: "module",
          depth: 1,
          name: "Capability Root",
          description: "",
          summary: "",
          path: ["Area One", "Capability Root"],
          allowed_child_kinds: ["feature"],
          children: [],
        },
      ],
    },
  ],
};

describe("productOverview helpers", () => {
  it("sorts and nests work items deterministically", () => {
    const root = makeWorkItem({ id: "root", title: "B task", sort_order: 1 });
    const earlier = makeWorkItem({
      id: "earlier",
      title: "A task",
      sort_order: 1,
      created_at: "2024-01-01T00:00:00Z",
    });
    const child = makeWorkItem({
      id: "child",
      title: "Child",
      sort_order: 0,
      parent_work_item_id: "root",
    });

    expect(sortWorkItems([root, earlier]).map((item) => item.id)).toEqual(["earlier", "root"]);
    expect(buildScopedWorkItemTree([child, root, earlier])).toEqual([
      { workItem: earlier, children: [] },
      { workItem: root, children: [{ workItem: child, children: [] }] },
    ]);
  });

  it("computes hierarchy and work-item metrics for overview summaries", () => {
    expect(countCapabilityTree(nestedCapabilityTree)).toBe(2);
    expect(countCapabilities([moduleTree])).toBe(2);
    expect(
      buildWorkItemMetrics([
        makeWorkItem({ status: "done" }),
        makeWorkItem({ id: "2", status: "in_progress" }),
        makeWorkItem({ id: "3", status: "blocked" }),
        makeWorkItem({ id: "4", status: "draft" }),
      ]),
    ).toEqual({
      total: 4,
      done: 1,
      wip: 1,
      tbd: 1,
      blocked: 1,
      completion: 25,
    });
  });

  it("maps statuses and section identifiers consistently", () => {
    expect(getWorkItemPresentation("failed").bucket).toBe("blocked");
    expect(getWorkItemPresentation("ready_for_review").label).toBe("WIP");
    expect(getWorkItemPresentation("approved").toneClass).toBe("is-tbd");
    expect(getModuleSectionId(moduleRecord)).toBe("module-module-1");
    expect(getCapabilitySectionId(makeCapability({ id: "capability-root" }))).toBe(
      "capability-capability-root",
    );
  });

  it("builds a toc with overview, delivery, and hierarchy sections", () => {
    expect(buildProductOverviewToc(productTree, true)).toEqual([
      { id: PRODUCT_OVERVIEW_TOP_ID, title: "Overview", level: 0 },
      { id: PRODUCT_DELIVERY_ID, title: "Product Delivery", level: 0 },
      { id: "module-module-1", title: "1. Area One", level: 0 },
      { id: "capability-capability-root", title: "1.1. Capability Root", level: 1 },
    ]);
  });
});
