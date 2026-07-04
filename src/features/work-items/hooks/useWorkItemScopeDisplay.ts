import { useMemo } from "react";

import { getHierarchyNodeKindLabel } from "../../../lib/hierarchyLabels";
import type { Capability, Product, ProductArea, WorkItem } from "../../../lib/types";
import { buildCapabilityPath } from "../lib/workItemListPageHelpers";

type WorkItemScopeDisplayInput = {
  products: Product[] | undefined;
  activeProductId: string | null;
  activeProductAreaId: string | null;
  activeCapabilityId: string | null;
  productAreaById: Map<string, ProductArea>;
  capabilityById: Map<string, Capability>;
  filteredWorkItems: WorkItem[];
  createParentWorkItemId: string | null;
};

export function useWorkItemScopeDisplay({
  products,
  activeProductId,
  activeProductAreaId,
  activeCapabilityId,
  productAreaById,
  capabilityById,
  filteredWorkItems,
  createParentWorkItemId,
}: WorkItemScopeDisplayInput) {
  const activeProduct = useMemo(
    () => (products ?? []).find((product) => product.id === activeProductId) ?? null,
    [activeProductId, products],
  );
  const activeProductArea = useMemo(
    () => activeProductAreaId ? productAreaById.get(activeProductAreaId) ?? null : null,
    [activeProductAreaId, productAreaById],
  );
  const activeCapability = useMemo(() => {
    return activeCapabilityId ? capabilityById.get(activeCapabilityId) ?? null : null;
  }, [activeCapabilityId, capabilityById]);
  const scopeDescriptor = useMemo(() => {
    const parts: string[] = [];
    if (activeProduct?.name) {
      parts.push(activeProduct.name);
    }
    if (activeProductArea?.name) {
      parts.push(activeProductArea.name);
    }
    if (activeCapability?.name) {
      parts.push(activeCapability.name);
    }
    return parts.length > 0 ? parts.join(" / ") : "None selected";
  }, [activeCapability?.name, activeProductArea?.name, activeProduct?.name]);
  const createWorkItemScopeLabel = createParentWorkItemId
    ? "Current story"
    : activeCapability
    ? `Current ${getHierarchyNodeKindLabel(activeCapability.node_kind, { lowercase: true })}`
    : activeCapabilityId
      ? "Current node"
      : activeProductAreaId
        ? "Current product area"
        : activeProductId
          ? "Current product"
          : "No product selected";
  const workItemOwnerMap = useMemo(() => {
    const map = new Map<string, { badge: string; path: string; isRoot: boolean }>();
    if (!activeProduct) {
      return map;
    }

    filteredWorkItems.forEach((workItem) => {
      const ownerId = workItem.source_node_id ?? workItem.capability_id ?? workItem.product_area_id;
      const ownerType = workItem.source_node_type ?? (workItem.capability_id ? "capability" : workItem.product_area_id ? "product_area" : null);

      if (ownerId && ownerType === "product_area") {
        const productArea = productAreaById.get(ownerId);
        if (productArea) {
          map.set(workItem.id, {
            badge: getHierarchyNodeKindLabel(productArea.node_kind),
            path: [activeProduct.name, productArea.name].join(" / "),
            isRoot: false,
          });
          return;
        }
      }

      if (ownerId && ownerType === "capability") {
        const capability = capabilityById.get(ownerId);
        if (capability) {
          const ownerPath = buildCapabilityPath(capability, productAreaById, capabilityById);
          map.set(workItem.id, {
            badge: getHierarchyNodeKindLabel(capability.node_kind),
            path: [activeProduct.name, ...ownerPath].join(" / "),
            isRoot: false,
          });
          return;
        }
      }

      if (workItem.capability_id) {
        const capability = capabilityById.get(workItem.capability_id);
        if (capability) {
          const ownerPath = buildCapabilityPath(capability, productAreaById, capabilityById);
          map.set(workItem.id, {
            badge: getHierarchyNodeKindLabel(capability.node_kind),
            path: [activeProduct.name, ...ownerPath].join(" / "),
            isRoot: false,
          });
          return;
        }
      }

      if (workItem.product_area_id) {
        const productArea = productAreaById.get(workItem.product_area_id);
        if (productArea) {
          map.set(workItem.id, {
            badge: getHierarchyNodeKindLabel(productArea.node_kind),
            path: [activeProduct.name, productArea.name].join(" / "),
            isRoot: false,
          });
          return;
        }
      }

      if (workItem.product_area_id || workItem.capability_id || workItem.source_node_id) {
        map.set(workItem.id, {
          badge: "Unknown Owner",
          path: activeProduct.name,
          isRoot: false,
        });
        return;
      }

      map.set(workItem.id, {
        badge: "Product",
        path: activeProduct.name,
        isRoot: true,
      });
    });

    return map;
  }, [activeProduct, capabilityById, filteredWorkItems, productAreaById]);

  return {
    activeProduct,
    activeProductArea,
    activeCapability,
    scopeDescriptor,
    createWorkItemScopeLabel,
    workItemOwnerMap,
  };
}
