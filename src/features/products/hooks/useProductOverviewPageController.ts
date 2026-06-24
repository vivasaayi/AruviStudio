import React, { useEffect } from "react";
import { useQuery } from "@tanstack/react-query";
import type { NavigateFunction } from "react-router-dom";
import {
  listCapabilities,
  listProductAreas,
  listProductReferences,
  listProducts,
  summarizeProductTree,
  summarizeWorkItemsByProduct,
} from "../../../lib/tauri";
import type {
  Capability,
  Product,
  ProductArea,
  ProductAreaTree,
  ProductReference,
  ProductTreeSummary,
  ProductWorkItemSummary,
  WorkItem,
} from "../../../lib/types";
import type { ProductOverviewPlannerAction } from "../components/ProductOverviewDocument";
import { buildCapabilityTrees, buildProductAreaOnlyTree } from "../lib/productOverviewTree";
import { type WorkItemMetrics } from "../lib/productOverview";
import { useProductOverviewExports } from "./useProductOverviewExports";

type ProductOverviewControllerArgs = {
  activeProductId: string | null;
  setActiveProduct: (productId: string | null) => void;
  setActiveProductArea: (productAreaId: string | null) => void;
  setActiveCapability: (capabilityId: string | null) => void;
  setActiveWorkItem: (workItemId: string | null) => void;
  setActiveView: (view: "products" | "work-items" | "planner") => void;
  setProductWorkspaceTab: (tab: "structure") => void;
  setWorkItemWorkspaceTab: (tab: "detail") => void;
  openProductDialog: (mode: "create" | "edit") => void;
  openProductAreaDialog: (mode: "create" | "edit") => void;
  openCapabilityDialog: (mode: "create" | "edit") => void;
  navigate: NavigateFunction;
};

export function useProductOverviewPageController({
  activeProductId,
  setActiveProduct,
  setActiveProductArea,
  setActiveCapability,
  setActiveWorkItem,
  setActiveView,
  setProductWorkspaceTab,
  setWorkItemWorkspaceTab,
  openProductDialog,
  openProductAreaDialog,
  openCapabilityDialog,
  navigate,
}: ProductOverviewControllerArgs) {
  const { data: products = [], isLoading: productsLoading } = useQuery<Product[]>({
    queryKey: ["products"],
    queryFn: listProducts,
  });

  const visibleActiveProductId = products.some((product) => product.id === activeProductId)
    ? activeProductId
    : null;
  const selectedProductId = visibleActiveProductId ?? products[0]?.id ?? null;
  const selectedProduct = products.find((product) => product.id === selectedProductId) ?? null;

  useEffect(() => {
    if (productsLoading) {
      return;
    }
    if (activeProductId !== selectedProductId) {
      setActiveProduct(selectedProductId);
    }
  }, [activeProductId, productsLoading, selectedProductId, setActiveProduct]);

  const { data: productAreas = [], isLoading: productAreasLoading } = useQuery<ProductArea[]>({
    queryKey: ["productOverviewProductAreas", selectedProductId],
    queryFn: () => listProductAreas(selectedProductId!),
    enabled: !!selectedProduct,
  });
  const { data: treeSummary, isLoading: treeSummaryLoading } = useQuery<ProductTreeSummary>({
    queryKey: ["productTreeSummary", selectedProductId],
    queryFn: () => summarizeProductTree(selectedProductId!),
    enabled: !!selectedProduct,
  });
  const tree = React.useMemo(
    () => selectedProduct ? buildProductAreaOnlyTree(selectedProduct, productAreas) : undefined,
    [productAreas, selectedProduct],
  );
  const treeLoading = productAreasLoading || treeSummaryLoading;
  const loadProductAreaTree = React.useCallback(async (productArea: ProductArea): Promise<ProductAreaTree> => {
    const capabilities = await listCapabilities(productArea.id);
    return {
      product_area: productArea,
      features: buildCapabilityTrees(capabilities),
    };
  }, []);

  const { data: productWorkItemSummaries = [], isLoading: summariesLoading } = useQuery<ProductWorkItemSummary[]>({
    queryKey: ["productWorkItemSummary"],
    queryFn: summarizeWorkItemsByProduct,
    enabled: !!selectedProduct,
  });
  const selectedProductWorkItemSummary = React.useMemo(
    () => productWorkItemSummaries.find((summary) => summary.product_id === selectedProductId) ?? null,
    [productWorkItemSummaries, selectedProductId],
  );
  const overviewMetrics = React.useMemo<WorkItemMetrics>(() => {
    const total = selectedProductWorkItemSummary?.total_count ?? 0;
    const done = selectedProductWorkItemSummary?.done_count ?? 0;
    const blocked = selectedProductWorkItemSummary?.blocked_count ?? 0;
    const active = selectedProductWorkItemSummary?.active_count ?? 0;
    const wip = Math.max(0, active - blocked);
    const tbd = Math.max(0, total - done - active);
    return {
      total,
      done,
      wip,
      tbd,
      blocked,
      completion: total > 0 ? Math.round((done / total) * 100) : 0,
    };
  }, [selectedProductWorkItemSummary]);
  const workItems = React.useMemo<WorkItem[]>(() => [], []);

  const { data: productReferences = [], isLoading: referencesLoading } = useQuery<ProductReference[]>({
    queryKey: ["productOverviewPageReferences"],
    queryFn: () => listProductReferences(),
    enabled: !!selectedProduct,
  });
  const {
    bookTrimPresetId,
    exportError,
    exportHtml,
    exportPath,
    isExporting,
    runBookArtifactExport,
    setBookTrimPresetId,
  } = useProductOverviewExports({
    selectedProduct,
    workItems,
    productReferences,
  });

  useEffect(() => {
    if (treeLoading || summariesLoading) {
      return;
    }
    if (typeof window === "undefined") {
      return;
    }
    const hash = window.location.hash.replace(/^#/, "");
    if (!hash) {
      return;
    }
    requestAnimationFrame(() => {
      document.getElementById(hash)?.scrollIntoView({ block: "start" });
    });
  }, [selectedProductId, treeLoading, summariesLoading, tree, overviewMetrics.total]);

  const goToProductWorkspace = () => {
    setActiveView("products");
    navigate("/products");
  };

  const editProduct = () => {
    if (!selectedProductId) {
      return;
    }
    setActiveProduct(selectedProductId);
    setActiveView("products");
    navigate(`/products/${selectedProductId}`);
    openProductDialog("edit");
  };

  const editProductArea = (product_area: ProductArea) => {
    if (!selectedProductId) {
      return;
    }
    setActiveProduct(selectedProductId);
    setActiveProductArea(product_area.id);
    setActiveCapability(null);
    setProductWorkspaceTab("structure");
    setActiveView("products");
    navigate(`/products/${selectedProductId}`);
    openProductAreaDialog("edit");
  };

  const editCapability = (capability: Capability) => {
    if (!selectedProductId) {
      return;
    }
    setActiveProduct(selectedProductId);
    setActiveProductArea(capability.product_area_id);
    setActiveCapability(capability.id);
    setProductWorkspaceTab("structure");
    setActiveView("products");
    navigate(`/products/${selectedProductId}`);
    openCapabilityDialog("edit");
  };

  const openWorkItem = (workItem: WorkItem) => {
    setActiveProduct(workItem.product_id);
    setActiveProductArea(workItem.product_area_id ?? null);
    setActiveCapability(workItem.capability_id ?? null);
    setActiveWorkItem(workItem.id);
    setWorkItemWorkspaceTab("detail");
    setActiveView("work-items");
    navigate(`/work-items/${workItem.id}`);
  };

  const planFromItem = (action: ProductOverviewPlannerAction) => {
    const product = action.product;
    let prompt: string;

    setActiveProduct(product.id);
    setActiveWorkItem(null);

    switch (action.kind) {
      case "enhance_product":
        prompt = `Enhance product "${product.name}". Review the current product management tree, identify missing product areas, capabilities, features, stories, and tasks, then stage a concrete improvement plan.`;
        break;
      case "add_product_child":
        prompt = `Add a useful product area under product "${product.name}". Stage the new product area with its initial capabilities and starter stories.`;
        break;
      case "enhance_product_area":
        setActiveProductArea(action.product_area.id);
        prompt = `Enhance product area "${action.product_area.name}" in product "${product.name}". Add or revise child capabilities, features, stories, and tasks so this branch is execution-ready.`;
        break;
      case "add_product_area_child":
        setActiveProductArea(action.product_area.id);
        prompt = `Add child capabilities under product area "${action.product_area.name}" in product "${product.name}". Include concise descriptions, acceptance criteria, and starter stories where helpful.`;
        break;
      case "enhance_capability":
        setActiveProductArea(action.capability.product_area_id);
        setActiveCapability(action.capability.id);
        prompt = `Enhance ${action.capability.node_kind.replace(/_/g, " ")} "${action.capability.name}" under "${action.productAreaName}" in product "${product.name}". Improve its description, acceptance criteria, technical notes, and missing child structure.`;
        break;
      case "add_capability_child":
        setActiveProductArea(action.capability.product_area_id);
        setActiveCapability(action.capability.id);
        prompt = `Add child nodes under "${action.capability.name}" in product "${product.name}". Stage concrete features with clear descriptions and acceptance criteria.`;
        break;
      case "add_capability_work_item":
        setActiveProductArea(action.capability.product_area_id);
        setActiveCapability(action.capability.id);
        prompt = `Add delivery stories and tasks under "${action.capability.name}" in product "${product.name}". Make each story specific, testable, and scoped to this branch.`;
        break;
      case "enhance_work_item":
        setActiveProductArea(action.workItem.product_area_id ?? null);
        setActiveCapability(action.workItem.capability_id ?? null);
        setActiveWorkItem(action.workItem.id);
        prompt = `Enhance story "${action.workItem.title}" in product "${product.name}". Improve the problem statement, acceptance criteria, constraints, and split it into tasks if needed.`;
        break;
    }

    setActiveView("planner");
    navigate("/planner", { state: { plannerPrompt: prompt, plannerView: "conversation" } });
  };

  return {
    bookTrimPresetId,
    editCapability,
    editProduct,
    editProductArea,
    exportError,
    exportHtml,
    exportPath,
    goToProductWorkspace,
    isExporting,
    loadProductAreaTree,
    openWorkItem,
    overviewMetrics,
    planFromItem,
    productReferences,
    products,
    productsLoading,
    referencesLoading,
    runBookArtifactExport,
    selectedProduct,
    selectedProductId,
    selectedProductWorkItemSummary,
    selectProduct: setActiveProduct,
    setBookTrimPresetId,
    summariesLoading,
    tree,
    treeLoading,
    treeSummary,
    workItems,
  };
}
