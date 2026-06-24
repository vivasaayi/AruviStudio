import { getDirectWorkItemsForNode } from "../../../lib/hierarchyTree";
import type { Capability, CapabilityTree, HierarchyTreeNode, ProductArea, ProductAreaTree, WorkItem } from "../../../lib/types";
import {
  findCapabilityTree,
  flattenCapabilityTreeList,
  getCapabilityOrderKey,
  getOrderedCapabilityTrees,
} from "./productHierarchyHelpers";

export type ProductManagementFeatureEntry = {
  capabilityTree: CapabilityTree;
  productArea: ProductArea;
  parentCapability: Capability | null;
};

export function selectManagementCapabilityTree(
  managementCapabilities: CapabilityTree[],
  activeCapabilityId: string | null,
  selectedCapabilityParentId: string | null | undefined,
) {
  const selectedTopLevelCapability = managementCapabilities.find((capabilityTree) => capabilityTree.capability.id === activeCapabilityId);
  if (selectedTopLevelCapability) {
    return selectedTopLevelCapability;
  }
  if (selectedCapabilityParentId) {
    return managementCapabilities.find((capabilityTree) => capabilityTree.capability.id === selectedCapabilityParentId)
      ?? managementCapabilities[0]
      ?? null;
  }
  return managementCapabilities[0] ?? null;
}

export function buildManagementFeatures(
  selectedManagementCapabilityTree: CapabilityTree | null,
  capabilityOrderMap: Record<string, string[]>,
) {
  return selectedManagementCapabilityTree
    ? getOrderedCapabilityTrees(
        selectedManagementCapabilityTree.children,
        capabilityOrderMap[getCapabilityOrderKey(
          selectedManagementCapabilityTree.capability.product_area_id,
          selectedManagementCapabilityTree.capability.id,
        )],
      ).filter((capabilityTree) => capabilityTree.capability.node_kind === "feature")
    : [];
}

export function buildAllManagementFeatures(productAreaProductAreas: ProductAreaTree[]): ProductManagementFeatureEntry[] {
  return productAreaProductAreas.flatMap((productAreaTree) =>
    flattenCapabilityTreeList(productAreaTree.features)
      .filter((capabilityTree) => capabilityTree.capability.node_kind === "feature")
      .map((capabilityTree) => ({
        capabilityTree,
        productArea: productAreaTree.product_area,
        parentCapability: capabilityTree.capability.parent_capability_id
          ? findCapabilityTree(productAreaProductAreas, capabilityTree.capability.parent_capability_id)?.capability ?? null
          : null,
      })),
  );
}

export function selectManagementFeature(
  allManagementFeatures: ProductManagementFeatureEntry[],
  activeCapabilityId: string | null,
) {
  return allManagementFeatures.find((entry) => entry.capabilityTree.capability.id === activeCapabilityId)
    ?? allManagementFeatures[0]
    ?? null;
}

export function buildFeatureStories(
  selectedManagementFeatureNode: HierarchyTreeNode | null,
  managementFeatureWorkItems: WorkItem[],
) {
  if (!selectedManagementFeatureNode) {
    return [];
  }
  return getDirectWorkItemsForNode(selectedManagementFeatureNode, managementFeatureWorkItems)
    .filter((workItem) => !workItem.parent_work_item_id);
}

export function selectManagementStory(
  featureStories: WorkItem[],
  selectedManagementStoryId: string | null,
  activeWorkItemId: string | null,
) {
  return featureStories.find((workItem) => workItem.id === selectedManagementStoryId)
    ?? featureStories.find((workItem) => workItem.id === activeWorkItemId)
    ?? featureStories[0]
    ?? null;
}

export function buildSelectedManagementTasks(
  selectedManagementStory: WorkItem | null,
  selectedManagementStoryTasks: WorkItem[],
  managementFeatureWorkItems: WorkItem[],
) {
  if (!selectedManagementStory) {
    return [];
  }
  const taskMap = new Map<string, WorkItem>();
  selectedManagementStoryTasks
    .filter((workItem) => workItem.parent_work_item_id === selectedManagementStory.id)
    .forEach((workItem) => taskMap.set(workItem.id, workItem));
  managementFeatureWorkItems
    .filter((workItem) => workItem.parent_work_item_id === selectedManagementStory.id)
    .forEach((workItem) => taskMap.set(workItem.id, workItem));
  return Array.from(taskMap.values()).sort((a, b) => a.sort_order - b.sort_order || a.title.localeCompare(b.title));
}
