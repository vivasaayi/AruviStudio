import { describe, expect, it } from "vitest";

import {
  buildAllManagementFeatures,
  buildFeatureStories,
  buildManagementFeatures,
  buildSelectedManagementTasks,
  selectManagementCapabilityTree,
  selectManagementFeature,
  selectManagementStory,
} from "./productManagementSelection";
import type { Capability, CapabilityTree, HierarchyTreeNode, ProductArea, ProductAreaTree, WorkItem } from "../../../lib/types";

function capability(
  id: string,
  name: string,
  nodeKind: Capability["node_kind"] = "capability",
  parentCapabilityId: string | null = null,
): Capability {
  return {
    id,
    product_area_id: "area-1",
    parent_capability_id: parentCapabilityId,
    level: parentCapabilityId ? 2 : 1,
    node_kind: nodeKind,
    sort_order: 0,
    name,
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
  };
}

function capabilityTree(capabilityNode: Capability, children: CapabilityTree[] = []): CapabilityTree {
  return { capability: capabilityNode, children };
}

function productArea(): ProductArea {
  return {
    id: "area-1",
    product_id: "product-1",
    node_kind: "product_area",
    name: "Commerce",
    description: "",
    purpose: "",
    explanation: "",
    examples: "",
    implementation_notes: "",
    test_guidance: "",
    sort_order: 0,
    created_at: "2026-01-01T00:00:00Z",
    updated_at: "2026-01-01T00:00:00Z",
  };
}

function hierarchyNode(id: string): HierarchyTreeNode {
  return {
    id,
    node_type: "capability",
    node_kind: "feature",
    product_area_id: "area-1",
    capability_id: id,
    parent_node_id: "capability-1",
    parent_node_type: "capability",
    depth: 2,
    name: "Checkout",
    description: "",
    summary: "",
    path: ["Commerce", "Checkout"],
    allowed_child_kinds: [],
    children: [],
  };
}

function workItem(id: string, parentWorkItemId: string | null, sortOrder: number, title = id): WorkItem {
  return {
    id,
    product_id: "product-1",
    product_area_id: "area-1",
    capability_id: "feature-1",
    source_node_id: "feature-1",
    source_node_type: "capability",
    parent_work_item_id: parentWorkItemId,
    title,
    problem_statement: "",
    description: "",
    acceptance_criteria: "",
    constraints: "",
    work_item_type: parentWorkItemId ? "task" : "story",
    priority: "medium",
    complexity: "medium",
    status: "draft",
    repo_override_id: null,
    active_repo_id: null,
    branch_name: null,
    sort_order: sortOrder,
    created_at: "2026-01-01T00:00:00Z",
    updated_at: "2026-01-01T00:00:00Z",
  };
}

describe("productManagementSelection", () => {
  it("selects the active top-level capability before falling back to a parent or first capability", () => {
    const first = capabilityTree(capability("capability-1", "Checkout"));
    const second = capabilityTree(capability("capability-2", "Identity"));
    const capabilities = [first, second];

    expect(selectManagementCapabilityTree(capabilities, "capability-2", null)).toBe(second);
    expect(selectManagementCapabilityTree(capabilities, "feature-1", "capability-1")).toBe(first);
    expect(selectManagementCapabilityTree(capabilities, "missing", null)).toBe(first);
    expect(selectManagementCapabilityTree([], "missing", null)).toBeNull();
  });

  it("builds ordered feature lists for the selected capability", () => {
    const featureA = capabilityTree(capability("feature-a", "A", "feature", "capability-1"));
    const featureB = capabilityTree(capability("feature-b", "B", "feature", "capability-1"));
    const selectedCapability = capabilityTree(capability("capability-1", "Checkout"), [featureA, featureB]);

    expect(buildManagementFeatures(selectedCapability, {
      "area-1:capability-1": ["feature-b", "feature-a"],
    }).map((entry) => entry.capability.id)).toEqual(["feature-b", "feature-a"]);
    expect(buildManagementFeatures(null, {})).toEqual([]);
  });

  it("flattens all features with product area and parent capability context", () => {
    const feature = capabilityTree(capability("feature-1", "Checkout", "feature", "capability-1"));
    const parent = capabilityTree(capability("capability-1", "Commerce"), [feature]);
    const productAreaTree: ProductAreaTree = { product_area: productArea(), features: [parent] };

    const entries = buildAllManagementFeatures([productAreaTree]);

    expect(entries).toHaveLength(1);
    expect(entries[0].capabilityTree).toBe(feature);
    expect(entries[0].productArea.id).toBe("area-1");
    expect(entries[0].parentCapability?.id).toBe("capability-1");
    expect(selectManagementFeature(entries, "feature-1")).toBe(entries[0]);
    expect(selectManagementFeature(entries, "missing")).toBe(entries[0]);
  });

  it("filters stories and merges task sources for the selected story", () => {
    const selectedNode = hierarchyNode("feature-1");
    const storyA = workItem("story-a", null, 0);
    const storyB = workItem("story-b", null, 1);
    const taskA = workItem("task-a", "story-b", 2, "B task");
    const taskB = workItem("task-b", "story-b", 1, "A task");
    const duplicateTask = workItem("task-a", "story-b", 0, "Updated task");

    const stories = buildFeatureStories(selectedNode, [storyA, storyB, taskA]);
    const selectedStory = selectManagementStory(stories, "story-b", "story-a");
    const tasks = buildSelectedManagementTasks(selectedStory, [duplicateTask], [taskA, taskB]);

    expect(stories.map((item) => item.id)).toEqual(["story-a", "story-b"]);
    expect(selectedStory?.id).toBe("story-b");
    expect(tasks.map((item) => item.id)).toEqual(["task-b", "task-a"]);
    expect(tasks.find((item) => item.id === "task-a")?.title).toBe("B task");
    expect(buildFeatureStories(null, [storyA])).toEqual([]);
    expect(buildSelectedManagementTasks(null, [taskA], [taskB])).toEqual([]);
  });
});
