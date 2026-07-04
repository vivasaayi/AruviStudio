import { useEffect, useMemo, useState } from "react";

import type { ProductTree } from "../../../lib/types";
import {
  findCapabilityTree,
  getCapabilityOrderKey,
  seedCapabilityOrderMap,
} from "../lib/productHierarchyHelpers";

type ProductHierarchySelectionStateInput = {
  tree: ProductTree | undefined;
  activeProductAreaId: string | null;
  activeCapabilityId: string | null;
};

export function useProductHierarchySelectionState({
  tree,
  activeProductAreaId,
  activeCapabilityId,
}: ProductHierarchySelectionStateInput) {
  const [productAreaOrderIds, setProductAreaOrderIds] = useState<string[]>([]);
  const [capabilityOrderMap, setCapabilityOrderMap] = useState<Record<string, string[]>>({});

  useEffect(() => {
    if (!tree) {
      return;
    }
    setProductAreaOrderIds(tree.product_areas.map((productAreaTree) => productAreaTree.product_area.id));
    const nextCapabilityMap: Record<string, string[]> = {};
    tree.product_areas.forEach((productAreaTree) => {
      nextCapabilityMap[getCapabilityOrderKey(productAreaTree.product_area.id, null)] = productAreaTree.features.map((capabilityTree) => capabilityTree.capability.id);
      seedCapabilityOrderMap(nextCapabilityMap, productAreaTree.features);
    });
    setCapabilityOrderMap(nextCapabilityMap);
  }, [tree]);

  const selectedProductArea = useMemo(
    () => tree?.product_areas.find((productAreaTree) => productAreaTree.product_area.id === activeProductAreaId)?.product_area ?? null,
    [tree, activeProductAreaId],
  );
  const selectedCapabilityTree = useMemo(
    () => (tree ? findCapabilityTree(tree.product_areas, activeCapabilityId) : null),
    [tree, activeCapabilityId],
  );
  const selectedCapability = selectedCapabilityTree?.capability ?? null;
  const selectedCapabilityParentKind = useMemo(() => {
    if (!selectedCapability) {
      return selectedProductArea?.node_kind ?? null;
    }
    if (!selectedCapability.parent_capability_id) {
      return selectedProductArea?.node_kind ?? null;
    }
    return findCapabilityTree(tree?.product_areas ?? [], selectedCapability.parent_capability_id)?.capability.node_kind ?? null;
  }, [selectedCapability, selectedProductArea, tree]);

  return {
    productAreaOrderIds,
    capabilityOrderMap,
    selectedProductArea,
    selectedCapabilityTree,
    selectedCapability,
    selectedCapabilityParentKind,
  };
}
