import { useMemo } from "react";
import { useQueries, useQuery } from "@tanstack/react-query";

import {
  getWorkItem,
  listCapabilities,
  listProductAreas,
  listProducts,
  listRepositories,
  listWorkItemsPage,
} from "../../../lib/tauri";
import type {
  Capability,
  HierarchyNodeType,
  ProductArea,
} from "../../../lib/types";
import {
  WORK_ITEM_PAGE_SIZE,
} from "../lib/workItemListPageHelpers";

type WorkItemScopeDataInput = {
  activeProductId: string | null;
  activeProductAreaId: string | null;
  activeNodeId: string | null;
  activeNodeType: HierarchyNodeType | null;
  activeWorkItemId: string | null;
  statusFilter: string;
  workItemPageIndex: number;
};

export function useWorkItemScopeData({
  activeProductId,
  activeProductAreaId,
  activeNodeId,
  activeNodeType,
  activeWorkItemId,
  statusFilter,
  workItemPageIndex,
}: WorkItemScopeDataInput) {
  const { data: products, isLoading: productsLoading } = useQuery({
    queryKey: ["products"],
    queryFn: () => listProducts(),
  });
  const selectedProductId = (products ?? []).some((product) => product.id === activeProductId)
    ? activeProductId
    : ((products ?? [])[0]?.id ?? null);

  const workItemsScopeQueryKey = ["workItems", selectedProductId, activeNodeId, activeNodeType, statusFilter] as const;
  const workItemsQueryKey = [...workItemsScopeQueryKey, workItemPageIndex] as const;
  const { data: workItemPage, isLoading } = useQuery({
    queryKey: workItemsQueryKey,
    queryFn: () =>
      listWorkItemsPage({
        productId: selectedProductId ?? undefined,
        sourceNodeId: activeNodeId ?? undefined,
        sourceNodeType: activeNodeType ?? undefined,
        status: statusFilter || undefined,
        limit: WORK_ITEM_PAGE_SIZE,
        offset: workItemPageIndex * WORK_ITEM_PAGE_SIZE,
      }),
    enabled: !!selectedProductId,
  });
  const workItems = workItemPage?.items ?? [];
  const { data: activeProductAreas = [] } = useQuery<ProductArea[]>({
    queryKey: ["workItemProductAreas", selectedProductId],
    queryFn: () => listProductAreas(selectedProductId!),
    enabled: !!selectedProductId,
  });
  const productAreaById = useMemo(
    () => new Map(activeProductAreas.map((productArea) => [productArea.id, productArea])),
    [activeProductAreas],
  );
  const productAreaIdsForCapabilityLookup = useMemo(() => {
    const ids = new Set<string>();
    if (activeProductAreaId && productAreaById.has(activeProductAreaId)) {
      ids.add(activeProductAreaId);
    }
    workItems.forEach((workItem) => {
      if (workItem.product_area_id && productAreaById.has(workItem.product_area_id)) {
        ids.add(workItem.product_area_id);
      }
    });
    return Array.from(ids).sort();
  }, [activeProductAreaId, productAreaById, workItems]);
  const productAreaCapabilityQueries = useQueries({
    queries: productAreaIdsForCapabilityLookup.map((productAreaId) => ({
      queryKey: ["workItemProductAreaCapabilities", productAreaId],
      queryFn: () => listCapabilities(productAreaId),
      enabled: !!selectedProductId,
    })),
  });
  const activeCapabilities = useMemo<Capability[]>(
    () => productAreaCapabilityQueries.flatMap((query) => query.data ?? []),
    [productAreaCapabilityQueries],
  );
  const capabilityById = useMemo(
    () => new Map(activeCapabilities.map((capability) => [capability.id, capability])),
    [activeCapabilities],
  );
  const { data: repositories = [] } = useQuery({ queryKey: ["repositories"], queryFn: listRepositories });
  const filteredWorkItems = useMemo(() => {
    if (!selectedProductId) {
      return [];
    }
    return workItems.filter((workItem) => workItem.product_id === selectedProductId);
  }, [selectedProductId, workItems]);
  const selectedWorkItemId = useMemo(() => {
    const activeIdInScope = activeWorkItemId && filteredWorkItems.some((workItem) => workItem.id === activeWorkItemId)
      ? activeWorkItemId
      : null;
    return activeIdInScope ?? filteredWorkItems[0]?.id ?? null;
  }, [activeWorkItemId, filteredWorkItems]);
  const { data: selectedWorkItem } = useQuery({
    queryKey: ["workItem", selectedWorkItemId],
    queryFn: () => getWorkItem(selectedWorkItemId!),
    enabled: !!selectedWorkItemId,
  });

  return {
    products,
    productsLoading,
    selectedProductId,
    workItemsScopeQueryKey,
    workItemsQueryKey,
    workItemPage,
    isLoading,
    productAreaById,
    capabilityById,
    repositories,
    filteredWorkItems,
    selectedWorkItemId,
    selectedWorkItem,
  };
}
