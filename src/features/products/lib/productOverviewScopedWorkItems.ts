import type { CapabilityTree, ProductAreaTree, WorkItem } from "../../../lib/types";

function collectCapabilityIds(capabilities: CapabilityTree[]): Set<string> {
  const ids = new Set<string>();
  capabilities.forEach((capabilityTree) => {
    ids.add(capabilityTree.capability.id);
    collectCapabilityIds(capabilityTree.children).forEach((id) => ids.add(id));
  });
  return ids;
}

export function getProductAreaScopedWorkItems(productAreaTree: ProductAreaTree, allWorkItems: WorkItem[]) {
  const capabilityIds = collectCapabilityIds(productAreaTree.features);
  return allWorkItems.filter(
    (workItem) => workItem.product_area_id === productAreaTree.product_area.id || (workItem.capability_id ? capabilityIds.has(workItem.capability_id) : false),
  );
}

export function getCapabilityScopedWorkItems(capabilityTree: CapabilityTree, allWorkItems: WorkItem[]) {
  const capabilityIds = collectCapabilityIds([capabilityTree]);
  return allWorkItems.filter((workItem) => workItem.capability_id ? capabilityIds.has(workItem.capability_id) : false);
}
