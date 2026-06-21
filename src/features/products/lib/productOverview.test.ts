import { describe, expect, it } from "vitest";

import {
  PRODUCT_DELIVERY_ID,
  PRODUCT_OVERVIEW_TOP_ID,
  buildProductOverviewBookHtml,
  buildProductOverviewHtml,
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
import type { ProductReference } from "../../../lib/types";

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
  node_kind: "product_area",
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
      node_type: "product_area",
      node_kind: "product_area",
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
          parent_node_type: "product_area",
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
    expect(getModuleSectionId(moduleRecord)).toBe("product-area-module-1");
    expect(getCapabilitySectionId(makeCapability({ id: "capability-root" }))).toBe(
      "capability-capability-root",
    );
  });

  it("builds a toc with overview, delivery, and hierarchy sections", () => {
    expect(buildProductOverviewToc(productTree, true)).toEqual([
      { id: PRODUCT_OVERVIEW_TOP_ID, title: "Overview", level: 0 },
      { id: PRODUCT_DELIVERY_ID, title: "Product Delivery", level: 0 },
      { id: "product-area-module-1", title: "1. Area One", level: 0 },
      { id: "capability-capability-root", title: "1.1. Capability Root", level: 1 },
    ]);
  });

  it("renders overview html with escaped content, references, and scoped delivery sections", () => {
    const html = buildProductOverviewHtml({
      product: {
        ...productTree.product,
        name: "Product <One>",
        description: "Line 1\nLine 2",
        vision: "Vision <script>",
        goals: ["Goal A", "Goal B"],
        tags: ["alpha", "beta"],
      },
      tree: productTree,
      workItems: [
        makeWorkItem({
          id: "product-item",
          title: "Product Story",
          description: "Ship <fast>",
          status: "in_progress",
        }),
        makeWorkItem({
          id: "module-item",
          module_id: "module-1",
          title: "Module Story",
          description: "Module summary",
          status: "done",
        }),
        makeWorkItem({
          id: "capability-item",
          capability_id: "capability-root",
          title: "Capability Story",
          problem_statement: "Capability problem",
          status: "blocked",
        }),
        makeWorkItem({
          id: "capability-child",
          capability_id: "capability-root",
          parent_work_item_id: "capability-item",
          title: "Child Task",
          acceptance_criteria: "Validate child path",
          work_item_type: "task",
          status: "draft",
        }),
      ],
      references: [
        makeReference({
          id: "product-ref",
          scope_type: "product",
          scope_id: "product-1",
          title: "Vision Doc",
          content: "Narrative <b>content</b>",
          uri: "https://example.com/doc?a=1&b=2",
        }),
        makeReference({
          id: "module-ref",
          scope_type: "product_area",
          scope_id: "module-1",
          title: "Area Spec",
          reference_kind: "architecture",
        }),
        makeReference({
          id: "cap-ref",
          scope_type: "capability",
          scope_id: "capability-root",
          title: "Capability Spec",
          reference_kind: "customer_evidence",
        }),
      ],
    });

    expect(html).toContain("Product &lt;One&gt;");
    expect(html).toContain("Line 1<br />Line 2");
    expect(html).toContain("Vision &lt;script&gt;");
    expect(html).toContain("Narrative &lt;b&gt;content&lt;/b&gt;");
    expect(html).toContain("https://example.com/doc?a=1&amp;b=2");
    expect(html).toContain("Product Delivery");
    expect(html).toContain("Chapter Delivery");
    expect(html).toContain("Stories");
    expect(html).toContain("Child Task");
    expect(html).toContain("1 sub-item");
    expect(html).toContain("1 blocked");
    expect(html).toContain("1 done");
    expect(html).toContain("status-pill is-wip");
    expect(html).toContain("Architecture");
    expect(html).toContain("Customer Evidence");
  });

  it("renders book html with fallback copy when hierarchy sections are sparse", () => {
    const html = buildProductOverviewBookHtml({
      product: {
        ...productTree.product,
        description: "",
        vision: "",
        goals: [],
      },
      tree: {
        ...productTree,
        modules: [
          {
            module: moduleRecord,
            features: [],
          },
        ],
      },
      workItems: [
        makeWorkItem({
          id: "product-item",
          title: "Cross-cutting story",
          description:
            "This is a very long description that should be summarized when rendered in book mode. "
            + "It keeps going so the excerpt path is exercised and trimmed for readability in the book export.",
          status: "waiting_human_review",
        }),
      ],
    });

    expect(html).toContain("A durable product narrative generated from Aruvi Studio.");
    expect(html).toContain("No goals recorded yet.");
    expect(html).toContain("Product Delivery Themes");
    expect(html).toContain("No capabilities are defined for this product area yet.");
    expect(html).toContain("Cross-cutting story");
    expect(html).toContain("WIP");
    expect(html).toContain("End of chapter 1.");
    expect(html).toContain("Title Page");
    expect(html).toContain("Prelude");
  });
});
