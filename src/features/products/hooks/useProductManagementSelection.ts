import { useMemo } from "react";
import { useQuery } from "@tanstack/react-query";

import { findHierarchyNode, flattenHierarchyNodes } from "../../../lib/hierarchyTree";
import { getAllowedChildNodeKinds, groupHierarchyNodeKinds, orderHierarchyNodeKinds } from "../../../lib/hierarchyLabels";
import { getSubWorkItems, listWorkItemsPage } from "../../../lib/tauri";
import type {
  CapabilityTree,
  HierarchyNodeKind,
  Product,
  ProductArea,
  ProductDependency,
  ProductTree,
  WorkItem,
} from "../../../lib/types";
import {
  PRODUCT_MANAGEMENT_STORY_PAGE_SIZE,
  SUB_WORK_ITEM_PAGE_SIZE,
} from "../lib/productListPageState";
import type { ProductManagementTab, ProductPageTab } from "../lib/productRefreshScopes";
import {
  buildCapabilityLabelById,
  buildCapabilityOptionsFromNodes,
  buildDependencyTargetCapabilityOptions,
} from "../lib/productDependencyOptions";
import {
  getCapabilityOrderKey,
  getOrderedCapabilityTrees,
  orderItemsByIds,
} from "../lib/productHierarchyHelpers";
import {
  buildAllManagementFeatures,
  buildFeatureStories,
  buildManagementFeatures,
  buildSelectedManagementTasks,
  selectManagementCapabilityTree,
  selectManagementFeature,
  selectManagementStory,
} from "../lib/productManagementSelection";

type ProductManagementSelectionInput = {
  tree: ProductTree | undefined;
  productTreeById: Map<string, ProductTree>;
  productDependencies: ProductDependency[];
  products: Product[] | undefined;
  selectedProductId: string | null;
  selectedProductArea: ProductArea | null;
  selectedCapability: CapabilityTree["capability"] | null;
  selectedCapabilityParentKind: HierarchyNodeKind | null;
  dependencyDependsOnProductId: string;
  productAreaOrderIds: string[];
  capabilityOrderMap: Record<string, string[]>;
  activeProductAreaId: string | null;
  activeCapabilityId: string | null;
  activeWorkItemId: string | null;
  selectedManagementStoryId: string | null;
  managementStoryPageIndex: number;
  productPageTab: ProductPageTab;
  productManagementTab: ProductManagementTab;
};

export function useProductManagementSelection({
  tree,
  productTreeById,
  productDependencies,
  products,
  selectedProductId,
  selectedProductArea,
  selectedCapability,
  selectedCapabilityParentKind,
  dependencyDependsOnProductId,
  productAreaOrderIds,
  capabilityOrderMap,
  activeProductAreaId,
  activeCapabilityId,
  activeWorkItemId,
  selectedManagementStoryId,
  managementStoryPageIndex,
  productPageTab,
  productManagementTab,
}: ProductManagementSelectionInput) {
  const allTreeNodes = useMemo(() => (tree ? flattenHierarchyNodes(tree.roots) : []), [tree]);
  const selectedCapabilityOptions = useMemo(
    () => buildCapabilityOptionsFromNodes(allTreeNodes),
    [allTreeNodes],
  );
  const dependencyTargetCapabilityOptions = useMemo(
    () => buildDependencyTargetCapabilityOptions(productTreeById, dependencyDependsOnProductId),
    [dependencyDependsOnProductId, productTreeById],
  );
  const selectedProductDependencies = useMemo(
    () => productDependencies.filter((dependency) => dependency.product_id === selectedProductId),
    [productDependencies, selectedProductId],
  );
  const productNameById = useMemo(
    () => new Map((products ?? []).map((product) => [product.id, product.name])),
    [products],
  );
  const capabilityLabelById = useMemo(
    () => buildCapabilityLabelById(productTreeById, allTreeNodes),
    [allTreeNodes, productTreeById],
  );
  const editableCapabilityNodeKinds = useMemo(() => {
    if (!selectedCapability) {
      return [] as HierarchyNodeKind[];
    }
    const allowedKinds = getAllowedChildNodeKinds(selectedCapabilityParentKind);
    return orderHierarchyNodeKinds(allowedKinds.includes(selectedCapability.node_kind)
      ? allowedKinds
      : [selectedCapability.node_kind, ...allowedKinds]);
  }, [selectedCapability, selectedCapabilityParentKind]);
  const selectedCapabilityAllowedKindGroups = useMemo(
    () => groupHierarchyNodeKinds(getAllowedChildNodeKinds(selectedCapability?.node_kind ?? selectedProductArea?.node_kind)),
    [selectedCapability?.node_kind, selectedProductArea?.node_kind],
  );
  const editableCapabilityNodeKindGroups = useMemo(
    () => groupHierarchyNodeKinds(editableCapabilityNodeKinds),
    [editableCapabilityNodeKinds],
  );
  const orderedProductAreas = useMemo(() => {
    if (!tree) {
      return [];
    }
    return orderItemsByIds(tree.product_areas, productAreaOrderIds, (productAreaTree) => productAreaTree.product_area.id);
  }, [tree, productAreaOrderIds]);
  const productAreaProductAreas = useMemo(
    () => orderedProductAreas.filter((productAreaTree) => productAreaTree.product_area.node_kind === "product_area"),
    [orderedProductAreas],
  );
  const selectedProductAreaTree = useMemo(
    () => productAreaProductAreas.find((productAreaTree) => productAreaTree.product_area.id === activeProductAreaId) ?? productAreaProductAreas[0] ?? null,
    [activeProductAreaId, productAreaProductAreas],
  );
  const selectedProductAreaNode = useMemo(
    () => selectedProductAreaTree ? tree?.roots.find((node) => node.id === selectedProductAreaTree.product_area.id) ?? null : null,
    [selectedProductAreaTree, tree],
  );
  const managementCapabilities = useMemo(
    () => selectedProductAreaTree
      ? getOrderedCapabilityTrees(
          selectedProductAreaTree.features,
          capabilityOrderMap[getCapabilityOrderKey(selectedProductAreaTree.product_area.id, null)],
        ).filter((capabilityTree) => capabilityTree.capability.node_kind === "capability")
      : [],
    [capabilityOrderMap, selectedProductAreaTree],
  );
  const selectedManagementCapabilityTree = useMemo(
    () => selectManagementCapabilityTree(
      managementCapabilities,
      activeCapabilityId,
      selectedCapability?.parent_capability_id,
    ),
    [activeCapabilityId, managementCapabilities, selectedCapability?.parent_capability_id],
  );
  const managementFeatures = useMemo(
    () => buildManagementFeatures(selectedManagementCapabilityTree, capabilityOrderMap),
    [capabilityOrderMap, selectedManagementCapabilityTree],
  );
  const allManagementFeatures = useMemo(
    () => buildAllManagementFeatures(productAreaProductAreas),
    [productAreaProductAreas],
  );
  const selectedManagementFeature = useMemo(
    () => selectManagementFeature(allManagementFeatures, activeCapabilityId),
    [activeCapabilityId, allManagementFeatures],
  );
  const selectedManagementFeatureNode = useMemo(
    () => selectedManagementFeature ? findHierarchyNode(tree?.roots ?? [], selectedManagementFeature.capabilityTree.capability.id, "capability") : null,
    [selectedManagementFeature, tree],
  );
  const { data: managementFeatureWorkItemPage } = useQuery({
    queryKey: ["productTasks", selectedProductId, selectedManagementFeatureNode?.id, selectedManagementFeatureNode?.node_type, managementStoryPageIndex],
    queryFn: () =>
      listWorkItemsPage({
        productId: selectedProductId ?? undefined,
        sourceNodeId: selectedManagementFeatureNode?.id,
        sourceNodeType: selectedManagementFeatureNode?.node_type,
        topLevelOnly: true,
        limit: PRODUCT_MANAGEMENT_STORY_PAGE_SIZE,
        offset: managementStoryPageIndex * PRODUCT_MANAGEMENT_STORY_PAGE_SIZE,
      }),
    enabled: !!selectedProductId && !!selectedManagementFeatureNode && productPageTab === "design" && productManagementTab === "work_items",
  });
  const managementFeatureWorkItems = managementFeatureWorkItemPage?.items ?? [];
  const featureStories = useMemo(
    () => buildFeatureStories(selectedManagementFeatureNode, managementFeatureWorkItems),
    [managementFeatureWorkItems, selectedManagementFeatureNode],
  );
  const selectedManagementStory = useMemo(
    () => selectManagementStory(featureStories, selectedManagementStoryId, activeWorkItemId),
    [activeWorkItemId, featureStories, selectedManagementStoryId],
  );
  const selectedManagementStoryIdForTasks = selectedManagementStory?.id ?? null;
  const { data: selectedManagementStoryTasks = [] } = useQuery({
    queryKey: ["subWorkItems", selectedManagementStoryIdForTasks, SUB_WORK_ITEM_PAGE_SIZE],
    queryFn: () =>
      getSubWorkItems(selectedManagementStoryIdForTasks!, {
        limit: SUB_WORK_ITEM_PAGE_SIZE,
        offset: 0,
      }),
    enabled: !!selectedManagementStoryIdForTasks,
  });
  const selectedManagementTasks = useMemo(
    () => buildSelectedManagementTasks(
      selectedManagementStory,
      selectedManagementStoryTasks,
      managementFeatureWorkItems,
    ),
    [managementFeatureWorkItems, selectedManagementStory, selectedManagementStoryTasks],
  );

  return {
    selectedCapabilityOptions,
    dependencyTargetCapabilityOptions,
    selectedProductDependencies,
    productNameById,
    capabilityLabelById,
    selectedCapabilityAllowedKindGroups,
    editableCapabilityNodeKindGroups,
    productAreaProductAreas,
    selectedProductAreaTree,
    selectedProductAreaNode,
    managementCapabilities,
    selectedManagementCapabilityTree,
    managementFeatures,
    allManagementFeatures,
    selectedManagementFeature,
    selectedManagementFeatureNode,
    managementFeatureWorkItemPage,
    featureStories,
    selectedManagementStory,
    selectedManagementStoryIdForTasks,
    selectedManagementTasks,
  };
}
