import React, { useEffect, useMemo, useState } from "react";
import { useMutation, useQueries, useQuery, useQueryClient } from "@tanstack/react-query";
import { useLocation, useNavigate } from "react-router-dom";
import {
  archiveProduct,
  createCapability,
  createLocalWorkspace,
  createProductArea,
  createProduct,
  createProductDependency,
  createWorkItem,
  deleteCapability,
  deleteProductArea,
  deleteWorkItem,
  getProductTree,
  getSubWorkItems,
  getSetting,
  listProductDependencies,
  listProductReferences,
  listProducts,
  listWorkItemsPage,
  reorderCapabilities,
  reorderProductAreas,
  resetProductPlan,
  revealInFinder,
  resolveRepositoryForScope,
  setSetting,
  summarizeProductTree,
  summarizeWorkItemsByProduct,
  summarizeWorkItemsByScope,
  updateCapability,
  updateProductArea,
  updateProduct,
  updateWorkItem,
} from "../../../lib/tauri";
import {
  countDescendantNodes,
  countHierarchyNodes,
  findHierarchyNode,
  findHierarchyNodePath,
  flattenHierarchyNodes,
  getDirectChildNodes,
  getDirectWorkItemsForNode,
  getHierarchyNodeKey,
  getHierarchyNodeSectionId,
} from "../../../lib/hierarchyTree";
import {
  getAllowedChildNodeKinds,
  getDefaultChildNodeKind,
  groupHierarchyNodeKinds,
  getHierarchyChildLabel,
  getHierarchyNodeKindLabel,
  orderHierarchyNodeKinds,
  ROOT_NODE_KINDS,
  supportsHierarchyChildren,
} from "../../../lib/hierarchyLabels";
import { useWorkspaceStore } from "../../../state/workspaceStore";
import { useUIStore } from "../../../state/uiStore";
import { ScopeBreadcrumb } from "../../../app/layout/ScopeBreadcrumb";
import { ProductOverviewPage } from "./ProductOverviewPage";
import { ProductManagementAreasTab } from "../components/ProductManagementAreasTab";
import { ProductManagementCapabilitiesTab } from "../components/ProductManagementCapabilitiesTab";
import { ProductManagementFeaturesTab } from "../components/ProductManagementFeaturesTab";
import { ProductManagementHeader } from "../components/ProductManagementHeader";
import { ProductManagementStoryDetailPane } from "../components/ProductManagementStoryDetailPane";
import { ProductManagementStoriesPane } from "../components/ProductManagementStoriesPane";
import { ProductManagementWorkItemFeatureSelector } from "../components/ProductManagementWorkItemFeatureSelector";
import { ProductCatalogTab } from "../components/ProductCatalogTab";
import {
  CapabilityFormModal,
  ProductAreaFormModal,
} from "../components/ProductManagementHierarchyModals";
import { ProductDependenciesTab } from "../components/ProductDependenciesTab";
import { ProductStatusTab } from "../components/ProductStatusTab";
import { ScopedRefreshButton } from "../components/ScopedRefreshButton";
import {
  DeleteHierarchyNodeModal,
  DeleteManagementWorkItemModal,
  ManagementWorkItemFormModal,
} from "../components/ProductManagementDeliveryModals";
import {
  DeleteProductModal,
  ProductFormModal,
  ResetProductPlanModal,
} from "../components/ProductManagementProductModals";
import { styles } from "../lib/productListPageStyles";
import {
  HIDE_EXAMPLE_PRODUCTS_KEY,
  PRODUCT_MANAGEMENT_STORY_PAGE_SIZE,
  SUB_WORK_ITEM_PAGE_SIZE,
  emptyProductDependencyDraft,
  emptyProductForm,
  emptyWorkItemDraft,
  parseBooleanSetting,
  productToForm,
  workItemToDraft,
  type ProductDependencyDraft,
  type ProductFormState,
  type WorkItemDraftState,
} from "../lib/productListPageState";
import { refreshScopedProductQueries } from "../lib/productQueryRefresh";
import {
  getProductManagementRefreshLabel,
  getProductManagementRefreshQueryKeys,
  getProductPageRefreshLabel,
  getProductPageRefreshQueryKeys,
  isProductPageRefreshDisabled,
  type ProductManagementTab,
  type ProductPageTab,
  type ProductStatusGroupBy,
} from "../lib/productRefreshScopes";
import { getProductAreaReferenceScope } from "../lib/productReferences";
import {
  buildProductCatalogRows,
  getProductCatalogTags,
  type ProductCatalogSort,
  type ProductCatalogSourceFilter,
  type ProductCatalogStatusFilter,
} from "../lib/productCatalogRows";
import {
  buildProductStatusSummary,
  buildStatusRows,
  buildWorkItemScopeSummaryIndex,
} from "../lib/productStatusSummary";
import {
  countCapabilities,
  findCapabilityTree,
  flattenCapabilityTreeList,
  getCapabilityOrderKey,
  getOrderedCapabilityTrees,
  orderItemsByIds,
  seedCapabilityOrderMap,
} from "../lib/productHierarchyHelpers";
import type { CapabilityTree, HierarchyNodeKind, HierarchyTreeNode, ProductAreaTree, Product, ProductTree, ProductTreeSummary, ProductWorkItemSummary, Repository, WorkItem, WorkItemScopeSummary } from "../../../lib/types";





export function ProductListPage() {
  const queryClient = useQueryClient();
  const navigate = useNavigate();
  const location = useLocation();
  const isProductDetailRoute = location.pathname.startsWith("/products/");
  const {
    activeProductId,
    activeProductAreaId,
    activeCapabilityId,
    activeNodeId,
    activeNodeType,
    activeWorkItemId,
    activeWorkspacePath,
    setActiveProduct,
    setActiveProductArea,
    setActiveCapability,
    setActiveHierarchyNode,
    setActiveWorkItem,
  } = useWorkspaceStore();
  const {
    productDialogMode,
    productAreaDialogMode,
    capabilityDialogMode,
    productWorkspaceTab,
    expandedProductAreas,
    expandedCapabilities,
    closeProductDialog,
    openProductDialog,
    closeProductAreaDialog,
    openProductAreaDialog,
    closeCapabilityDialog,
    openCapabilityDialog,
    setProductWorkspaceTab,
    toggleProductAreaExpanded,
    toggleCapabilityExpanded,
    setProductAreaExpanded,
    setCapabilityExpanded,
    setActiveView,
  } = useUIStore();

  const [productForm, setProductForm] = useState<ProductFormState>(emptyProductForm);
  const [productDraft, setProductDraft] = useState<ProductFormState>(emptyProductForm);
  const [productAreaForm, setProductAreaForm] = useState<{ name: string; description: string; purpose: string; nodeKind: HierarchyNodeKind }>({ name: "", description: "", purpose: "", nodeKind: "product_area" });
  const [productAreaDraft, setProductAreaDraft] = useState<{ name: string; description: string; purpose: string; nodeKind: HierarchyNodeKind }>({ name: "", description: "", purpose: "", nodeKind: "product_area" });
  const [capabilityForm, setCapabilityForm] = useState<{ name: string; description: string; acceptanceCriteria: string; technicalNotes: string; nodeKind: HierarchyNodeKind }>({ name: "", description: "", acceptanceCriteria: "", technicalNotes: "", nodeKind: "capability" });
  const [capabilityDraft, setCapabilityDraft] = useState<{ name: string; description: string; acceptanceCriteria: string; technicalNotes: string; nodeKind: HierarchyNodeKind }>({ name: "", description: "", acceptanceCriteria: "", technicalNotes: "", nodeKind: "capability" });
  const [productManagementTab, setProductManagementTab] = useState<ProductManagementTab>("areas");
  const [formError, setFormError] = useState<string | null>(null);
  const [workspaceActionMsg, setWorkspaceActionMsg] = useState<string | null>(null);
  const [workspaceActionError, setWorkspaceActionError] = useState<string | null>(null);
  const [draggedProductAreaId, setDraggedProductAreaId] = useState<string | null>(null);
  const [draggedFeature, setDraggedFeature] = useState<null | { id: string; productAreaId: string; parentCapabilityId?: string | null; siblingIds: string[] }>(null);
  const [productAreaOrderIds, setProductAreaOrderIds] = useState<string[]>([]);
  const [capabilityOrderMap, setFeatureOrderMap] = useState<Record<string, string[]>>({});
  const [productPageTab, setProductPageTab] = useState<ProductPageTab>(() => isProductDetailRoute ? "design" : "list");
  const [productSearch, setProductSearch] = useState("");
  const [productStatusFilter, setProductStatusFilter] = useState<ProductCatalogStatusFilter>("all");
  const [productSourceFilter, setProductSourceFilter] = useState<ProductCatalogSourceFilter>("all");
  const [productTagFilter, setProductTagFilter] = useState("all");
  const [productSort, setProductSort] = useState<ProductCatalogSort>("name");
  const [showDefaultProductsInTable, setShowDefaultProductsInTable] = useState(true);
  const [showCustomProductsInTable, setShowCustomProductsInTable] = useState(true);
  const [catalogFilterMsg, setCatalogFilterMsg] = useState<string | null>(null);
  const [catalogFilterError, setCatalogFilterError] = useState<string | null>(null);
  const [statusProductId, setStatusProductId] = useState<string>("all");
  const [statusDepth, setStatusDepth] = useState(1);
  const [statusGroupBy, setStatusGroupBy] = useState<ProductStatusGroupBy>("work_status");
  const [dependencyDraft, setDependencyDraft] = useState<ProductDependencyDraft>(emptyProductDependencyDraft);
  const [deleteProductCandidate, setDeleteProductCandidate] = useState<Product | null>(null);
  const [deleteConfirmName, setDeleteConfirmName] = useState("");
  const [deleteConfirmArchive, setDeleteConfirmArchive] = useState(false);
  const [resetPlanCandidate, setResetPlanCandidate] = useState<Product | null>(null);
  const [resetPlanConfirmName, setResetPlanConfirmName] = useState("");
  const [resetPlanConfirmTree, setResetPlanConfirmTree] = useState(false);
  const [resetPlanDeleteDelivery, setResetPlanDeleteDelivery] = useState(false);
  const [deleteHierarchyCandidate, setDeleteHierarchyCandidate] = useState<null | {
    kind: "product_area" | "capability" | "feature";
    id: string;
    name: string;
  }>(null);
  const [deleteHierarchyConfirmName, setDeleteHierarchyConfirmName] = useState("");
  const [deleteHierarchyConfirmChecked, setDeleteHierarchyConfirmChecked] = useState(false);
  const [selectedManagementStoryId, setSelectedManagementStoryId] = useState<string | null>(null);
  const [managementStoryPageIndex, setManagementStoryPageIndex] = useState(0);
  const [storyDialogMode, setStoryDialogMode] = useState<"closed" | "create" | "edit">("closed");
  const [taskDialogMode, setTaskDialogMode] = useState<"closed" | "create" | "edit">("closed");
  const [editingStory, setEditingStory] = useState<WorkItem | null>(null);
  const [editingTask, setEditingTask] = useState<WorkItem | null>(null);
  const [deleteWorkItemCandidate, setDeleteWorkItemCandidate] = useState<null | { workItem: WorkItem; kind: "story" | "task" }>(null);
  const [deleteWorkItemConfirmName, setDeleteWorkItemConfirmName] = useState("");
  const [deleteWorkItemConfirmChecked, setDeleteWorkItemConfirmChecked] = useState(false);
  const [storyDraft, setStoryDraft] = useState<WorkItemDraftState>(emptyWorkItemDraft);
  const [taskDraft, setTaskDraft] = useState<WorkItemDraftState>(emptyWorkItemDraft);
  const [outlineSearchTerm, setOutlineSearchTerm] = useState("");
  const [outlineKindFilter, setOutlineKindFilter] = useState<HierarchyNodeKind | "">("");
  const [recentNodeKeys, setRecentNodeKeys] = useState<string[]>([]);
  const [copiedEntityId, setCopiedEntityId] = useState<string | null>(null);
  const outlineNodeRefs = React.useRef<Record<string, HTMLDivElement | null>>({});

  const { data: products, isLoading } = useQuery({ queryKey: ["products"], queryFn: listProducts });
  const { data: productDependencies = [] } = useQuery({
    queryKey: ["product-dependencies"],
    queryFn: listProductDependencies,
    enabled: productPageTab === "design" || productPageTab === "dependencies",
  });
  const { data: productReferences = [] } = useQuery({
    queryKey: ["product-references"],
    queryFn: () => listProductReferences(),
    enabled: productPageTab === "design",
  });
  const { data: hideExampleProductsSetting } = useQuery({
    queryKey: ["setting", HIDE_EXAMPLE_PRODUCTS_KEY],
    queryFn: () => getSetting(HIDE_EXAMPLE_PRODUCTS_KEY),
  });
  const { data: productWorkItemSummaries = [] } = useQuery<ProductWorkItemSummary[]>({
    queryKey: ["productWorkItemSummary"],
    queryFn: summarizeWorkItemsByProduct,
    enabled: productPageTab === "list" || productPageTab === "status",
  });
  const visibleActiveProductId = products?.some((product) => product.id === activeProductId)
    ? activeProductId
    : null;
  const selectedProductId = visibleActiveProductId ?? products?.[0]?.id ?? null;
  const selectedProduct = useMemo(
    () => products?.find((product) => product.id === selectedProductId) ?? null,
    [products, selectedProductId],
  );

  useEffect(() => {
    if (isLoading) {
      return;
    }
    if (activeProductId !== selectedProductId) {
      setActiveProduct(selectedProductId);
    }
  }, [activeProductId, isLoading, selectedProductId, setActiveProduct]);

  useEffect(() => {
    if (statusProductId === "all") {
      return;
    }
    if (!products?.some((product) => product.id === statusProductId)) {
      setStatusProductId("all");
    }
  }, [products, statusProductId]);

  const { data: tree } = useQuery({
    queryKey: ["productTree", selectedProductId],
    queryFn: () => getProductTree(selectedProductId!),
    enabled: !!selectedProduct && (productPageTab === "design" || productPageTab === "dependencies"),
  });

  const productTreeQueries = useQueries({
    queries: (products ?? []).map((product) => ({
      queryKey: ["productTree", product.id],
      queryFn: () => getProductTree(product.id),
      enabled: !!product.id && (productPageTab === "dependencies" || (productPageTab === "status" && statusGroupBy !== "work_status")),
    })),
  });

  const productTreeSummaryQueries = useQueries({
    queries: (products ?? []).map((product) => ({
      queryKey: ["productTreeSummary", product.id],
      queryFn: () => summarizeProductTree(product.id),
      enabled: !!product.id && (productPageTab === "list" || productPageTab === "status"),
    })),
  });

  const workItemScopeSummaryProductId = productPageTab === "status"
    ? statusProductId === "all" ? undefined : statusProductId
    : selectedProductId ?? undefined;
  const { data: workItemScopeSummaries = [] } = useQuery<WorkItemScopeSummary[]>({
    queryKey: ["workItemScopeSummary", workItemScopeSummaryProductId ?? "all"],
    queryFn: () => summarizeWorkItemsByScope({ productId: workItemScopeSummaryProductId }),
    enabled: productPageTab === "status" || (!!selectedProduct && productPageTab === "design"),
  });

  const { data: resolvedWorkspace } = useQuery<Repository | null>({
    queryKey: ["productScopeRepo", selectedProductId, activeProductAreaId],
    queryFn: () => resolveRepositoryForScope({ productId: selectedProductId, productAreaId: activeProductAreaId }),
    enabled: !!selectedProduct && productPageTab === "design",
  });
  const effectiveWorkspacePath = resolvedWorkspace?.local_path ?? activeWorkspacePath ?? null;

  const productTreeById = useMemo(() => {
    const map = new Map<string, ProductTree>();
    (products ?? []).forEach((product, index) => {
      const result = productTreeQueries[index]?.data;
      if (result) {
        map.set(product.id, result);
      }
    });
    return map;
  }, [productTreeQueries, products]);

  const productTreeSummaryById = useMemo(() => {
    const map = new Map<string, ProductTreeSummary>();
    (products ?? []).forEach((product, index) => {
      const result = productTreeSummaryQueries[index]?.data;
      if (result) {
        map.set(product.id, result);
      }
    });
    return map;
  }, [productTreeSummaryQueries, products]);

  const scopeSummaryIndex = useMemo(() => buildWorkItemScopeSummaryIndex(workItemScopeSummaries), [workItemScopeSummaries]);

  const productSummaryById = useMemo(() => {
    const map = new Map<string, ProductWorkItemSummary>();
    productWorkItemSummaries.forEach((summary) => map.set(summary.product_id, summary));
    return map;
  }, [productWorkItemSummaries]);

  const allProductTags = useMemo(() => getProductCatalogTags(products ?? []), [products]);

  const includeDefaultProductsInCatalog = !parseBooleanSetting(hideExampleProductsSetting, true);
  const productTableRows = useMemo(() => buildProductCatalogRows({
    products: products ?? [],
    productTreeSummaryById,
    productSummaryById,
    search: productSearch,
    statusFilter: productStatusFilter,
    sourceFilter: productSourceFilter,
    tagFilter: productTagFilter,
    sort: productSort,
    showDefaultProducts: showDefaultProductsInTable,
    showCustomProducts: showCustomProductsInTable,
  }), [
    productSearch,
    productSort,
    productSourceFilter,
    productStatusFilter,
    productTagFilter,
    productSummaryById,
    productTreeSummaryById,
    products,
    showCustomProductsInTable,
    showDefaultProductsInTable,
  ]);

  const selectedStatusProduct = statusProductId === "all"
    ? null
    : products?.find((product) => product.id === statusProductId) ?? null;
  const selectedStatusProducts = selectedStatusProduct ? [selectedStatusProduct] : (products ?? []);
  const statusSummary = useMemo(
    () => buildProductStatusSummary(selectedStatusProducts, productTreeSummaryById, productSummaryById),
    [productSummaryById, productTreeSummaryById, selectedStatusProducts],
  );
  const statusRows = useMemo(
    () => buildStatusRows(selectedStatusProducts, productTreeById, scopeSummaryIndex, statusDepth, statusGroupBy),
    [productTreeById, scopeSummaryIndex, selectedStatusProducts, statusDepth, statusGroupBy],
  );

  const updateDefaultProductVisibility = async (includeDefaultProducts: boolean) => {
    try {
      setCatalogFilterMsg(null);
      setCatalogFilterError(null);
      await setSetting(HIDE_EXAMPLE_PRODUCTS_KEY, includeDefaultProducts ? "false" : "true");
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["products"] }),
        queryClient.invalidateQueries({ queryKey: ["setting", HIDE_EXAMPLE_PRODUCTS_KEY] }),
      ]);
      setCatalogFilterMsg(includeDefaultProducts ? "Default products are included." : "Default products are hidden.");
    } catch (error) {
      setCatalogFilterError(String(error));
    }
  };

  const openWorkspaceInIde = () => {
    if (resolvedWorkspace) {
      useWorkspaceStore.getState().setActiveRepo(resolvedWorkspace.id);
      useWorkspaceStore.getState().setActiveWorkspace(resolvedWorkspace.local_path);
    } else if (effectiveWorkspacePath) {
      useWorkspaceStore.getState().setActiveWorkspace(effectiveWorkspacePath);
    }
    setWorkspaceActionError(null);
    setActiveView("ide");
    navigate("/ide");
  };

  useEffect(() => {
    if (!activeProductId && products?.[0]?.id) {
      setActiveProduct(products[0].id);
    }
  }, [activeProductId, products, setActiveProduct]);

  useEffect(() => {
    setActiveWorkItem(null);
    setFormError(null);
    setWorkspaceActionMsg(null);
    setWorkspaceActionError(null);
  }, [selectedProductId, activeProductAreaId, activeCapabilityId, setActiveWorkItem]);

  useEffect(() => {
    if (selectedProduct) {
      setProductDraft(productToForm(selectedProduct));
    }
  }, [selectedProduct]);

  useEffect(() => {
    setFormError(null);
    if (productDialogMode === "create") {
      setProductForm(emptyProductForm);
    }
  }, [productDialogMode]);

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
    setFeatureOrderMap(nextCapabilityMap);
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

  useEffect(() => {
    if (productAreaDialogMode === "create") {
      setProductAreaForm({ name: "", description: "", purpose: "", nodeKind: "product_area" });
      return;
    }
    if (productAreaDialogMode === "edit" && selectedProductArea) {
      setProductAreaDraft({
        name: selectedProductArea.name,
        description: selectedProductArea.description,
        purpose: selectedProductArea.purpose,
        nodeKind: selectedProductArea.node_kind,
      });
    }
  }, [productAreaDialogMode, selectedProductArea]);

  useEffect(() => {
    if (capabilityDialogMode === "create") {
      setCapabilityForm({
        name: "",
        description: "",
        acceptanceCriteria: "",
        technicalNotes: "",
        nodeKind: getDefaultChildNodeKind(selectedCapability?.node_kind ?? selectedProductArea?.node_kind),
      });
      setFormError(null);
      return;
    }
    if (capabilityDialogMode === "edit" && selectedCapability) {
      setCapabilityDraft({
        name: selectedCapability.name,
        description: selectedCapability.description,
        acceptanceCriteria: selectedCapability.acceptance_criteria,
        technicalNotes: selectedCapability.technical_notes,
        nodeKind: selectedCapability.node_kind,
      });
    }
  }, [capabilityDialogMode, selectedCapability, selectedProductArea]);

  const invalidateHierarchy = async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ["products"] }),
      queryClient.invalidateQueries({ queryKey: ["productTree", selectedProductId] }),
      queryClient.invalidateQueries({ queryKey: ["productOverviewProductAreas", selectedProductId] }),
      queryClient.invalidateQueries({ queryKey: ["productTreeSummary", selectedProductId] }),
      queryClient.invalidateQueries({ queryKey: ["sidebarProductTree", selectedProductId] }),
    ]);
  };

  const invalidateTasks = async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ["workItemScopeSummary", selectedProductId] }),
      queryClient.invalidateQueries({ queryKey: ["productTasks", selectedProductId] }),
      queryClient.invalidateQueries({ queryKey: ["productWorkItemSummary"] }),
      queryClient.invalidateQueries({ queryKey: ["subWorkItems"] }),
      queryClient.invalidateQueries({ queryKey: ["workItems"] }),
      queryClient.invalidateQueries({ queryKey: ["sidebarWorkItems", selectedProductId] }),
    ]);
  };

  const createProductMutation = useMutation({
    mutationFn: () => createProduct(productForm),
    onSuccess: async (createdProduct) => {
      await invalidateHierarchy();
      setProductForm(emptyProductForm);
      setActiveProduct(createdProduct.id);
      closeProductDialog();
    },
    onError: (error) => setFormError(String(error)),
  });

  const updateProductMutation = useMutation({
    mutationFn: () =>
      updateProduct({
        id: selectedProductId!,
        name: productDraft.name,
        description: productDraft.description,
        vision: productDraft.vision,
        goals: productDraft.goals,
        tags: productDraft.tags,
        lifecycle: productDraft.lifecycle,
        health: productDraft.health,
        ownerLabel: productDraft.ownerLabel,
        investmentStatus: productDraft.investmentStatus,
        roadmap: productDraft.roadmap,
        evidence: productDraft.evidence,
      }),
    onSuccess: async () => {
      await invalidateHierarchy();
      closeProductDialog();
    },
    onError: (error) => setFormError(String(error)),
  });

  const createProductDependencyMutation = useMutation({
    mutationFn: () => createProductDependency({
      productId: selectedProductId!,
      capabilityId: dependencyDraft.capabilityId || null,
      dependsOnProductId: dependencyDraft.dependsOnProductId,
      dependsOnCapabilityId: dependencyDraft.dependsOnCapabilityId || null,
      dependencyKind: dependencyDraft.dependencyKind,
      description: dependencyDraft.description.trim(),
      status: "active",
    }),
    onSuccess: async () => {
      setDependencyDraft(emptyProductDependencyDraft);
      setFormError(null);
      await queryClient.invalidateQueries({ queryKey: ["product-dependencies"] });
    },
    onError: (error) => setFormError(String(error)),
  });

  const createProductAreaMutation = useMutation({
    mutationFn: () => createProductArea({ productId: selectedProductId!, ...productAreaForm }),
    onSuccess: async (createdProductArea) => {
      await invalidateHierarchy();
      setProductAreaForm({ name: "", description: "", purpose: "", nodeKind: "product_area" });
      setProductWorkspaceTab("structure");
      setActiveProductArea(createdProductArea.id);
      closeProductAreaDialog();
    },
    onError: (error) => setFormError(String(error)),
  });

  const updateProductAreaMutation = useMutation({
    mutationFn: () =>
      updateProductArea({
        id: activeProductAreaId!,
        name: productAreaDraft.name,
        description: productAreaDraft.description,
        purpose: productAreaDraft.purpose,
        nodeKind: "product_area",
      }),
    onSuccess: async (updatedProductArea) => {
      await invalidateHierarchy();
      setActiveProductArea(updatedProductArea.id);
      closeProductAreaDialog();
    },
    onError: (error) => setFormError(String(error)),
  });

  const createCapabilityMutation = useMutation({
    mutationFn: () =>
      createCapability({
        productAreaId: activeProductAreaId ?? selectedCapability?.product_area_id ?? selectedProductArea?.id ?? "",
        parentCapabilityId: activeCapabilityId ?? undefined,
        name: capabilityForm.name,
        description: capabilityForm.description,
        acceptanceCriteria: capabilityForm.acceptanceCriteria,
        priority: "medium",
        risk: "low",
        technicalNotes: capabilityForm.technicalNotes,
        nodeKind: capabilityForm.nodeKind,
      }),
    onSuccess: async (createdCapability) => {
      await invalidateHierarchy();
      setCapabilityForm({
        name: "",
        description: "",
        acceptanceCriteria: "",
        technicalNotes: "",
        nodeKind: getDefaultChildNodeKind(selectedCapability?.node_kind ?? selectedProductArea?.node_kind),
      });
      setProductWorkspaceTab("structure");
      setActiveCapability(createdCapability.id);
      closeCapabilityDialog();
    },
    onError: (error) => setFormError(String(error)),
  });

  const updateCapabilityMutation = useMutation({
    mutationFn: () =>
      updateCapability({
        id: activeCapabilityId!,
        name: capabilityDraft.name,
        description: capabilityDraft.description,
        acceptanceCriteria: capabilityDraft.acceptanceCriteria,
        technicalNotes: capabilityDraft.technicalNotes,
        nodeKind: capabilityDraft.nodeKind,
      }),
    onSuccess: async (updatedCapability) => {
      await invalidateHierarchy();
      setActiveCapability(updatedCapability.id);
      closeCapabilityDialog();
    },
    onError: (error) => setFormError(String(error)),
  });

  const archiveMutation = useMutation({
    mutationFn: (id: string) => archiveProduct(id),
    onSuccess: async (_, archivedId) => {
      await invalidateHierarchy();
      if (selectedProductId === archivedId) {
        setActiveProduct(null);
      }
      if (statusProductId === archivedId) {
        setStatusProductId("all");
      }
      setDeleteProductCandidate(null);
      setDeleteConfirmName("");
      setDeleteConfirmArchive(false);
    },
    onError: (error) => setFormError(String(error)),
  });

  const resetProductPlanMutation = useMutation({
    mutationFn: async (data: { productId: string; deleteDelivery: boolean }) => {
      if (data.productId !== selectedProductId) {
        throw new Error("Select the product before resetting its plan.");
      }
      await resetProductPlan(data);
    },
    onSuccess: async () => {
      await invalidateHierarchy();
      await invalidateTasks();
      setActiveProductArea(null);
      setActiveCapability(null);
      setResetPlanCandidate(null);
      setResetPlanConfirmName("");
      setResetPlanConfirmTree(false);
      setResetPlanDeleteDelivery(false);
      setProductWorkspaceTab("structure");
      setFormError(null);
    },
    onError: (error) => setFormError(String(error)),
  });

  const deleteHierarchyMutation = useMutation({
    mutationFn: async (candidate: NonNullable<typeof deleteHierarchyCandidate>) => {
      if (candidate.kind === "product_area") {
        await deleteProductArea(candidate.id);
        return;
      }
      await deleteCapability(candidate.id);
    },
    onSuccess: async () => {
      await invalidateHierarchy();
      setActiveProductArea(null);
      setActiveCapability(null);
      setDeleteHierarchyCandidate(null);
      setDeleteHierarchyConfirmName("");
      setDeleteHierarchyConfirmChecked(false);
      setFormError(null);
    },
    onError: (error) => setFormError(String(error)),
  });

  const reorderProductAreasMutation = useMutation({
    mutationFn: (orderedIds: string[]) => reorderProductAreas(selectedProductId!, orderedIds),
    onSuccess: async () => invalidateHierarchy(),
  });

  const reorderCapabilitiesMutation = useMutation({
    mutationFn: (data: { productAreaId: string; parentCapabilityId?: string; orderedIds: string[] }) => reorderCapabilities(data),
    onSuccess: async () => invalidateHierarchy(),
  });

  const createManagementStoryMutation = useMutation({
    mutationFn: () => {
      if (!selectedProductId || !selectedManagementFeatureNode) {
        throw new Error("Select a feature before adding a story.");
      }
      return createWorkItem({
        productId: selectedProductId,
        productAreaId: selectedManagementFeatureNode.product_area_id ?? undefined,
        capabilityId: selectedManagementFeatureNode.capability_id ?? undefined,
        sourceNodeId: selectedManagementFeatureNode.id,
        sourceNodeType: selectedManagementFeatureNode.node_type,
        title: storyDraft.title.trim(),
        problemStatement: storyDraft.problemStatement.trim(),
        description: storyDraft.description.trim(),
        acceptanceCriteria: storyDraft.acceptanceCriteria.trim(),
        constraints: storyDraft.constraints.trim(),
        workItemType: "story",
        priority: storyDraft.priority,
        complexity: storyDraft.complexity,
      });
    },
    onSuccess: async (createdStory) => {
      await invalidateTasks();
      setSelectedManagementStoryId(createdStory.id);
      setActiveWorkItem(createdStory.id);
      setStoryDraft(emptyWorkItemDraft);
      setStoryDialogMode("closed");
      setFormError(null);
    },
    onError: (error) => setFormError(String(error)),
  });

  const updateManagementStoryMutation = useMutation({
    mutationFn: () => {
      if (!editingStory) {
        throw new Error("Select a story before editing.");
      }
      return updateWorkItem({
        id: editingStory.id,
        title: storyDraft.title.trim(),
        status: storyDraft.status,
        problemStatement: storyDraft.problemStatement.trim(),
        description: storyDraft.description.trim(),
        acceptanceCriteria: storyDraft.acceptanceCriteria.trim(),
        constraints: storyDraft.constraints.trim(),
      });
    },
    onSuccess: async (updatedStory) => {
      await invalidateTasks();
      setSelectedManagementStoryId(updatedStory.id);
      setActiveWorkItem(updatedStory.id);
      setEditingStory(null);
      setStoryDraft(emptyWorkItemDraft);
      setStoryDialogMode("closed");
      setFormError(null);
    },
    onError: (error) => setFormError(String(error)),
  });

  const createManagementTaskMutation = useMutation({
    mutationFn: () => {
      if (!selectedProductId || !selectedManagementFeatureNode || !selectedManagementStory) {
        throw new Error("Select a story before adding a task.");
      }
      return createWorkItem({
        productId: selectedProductId,
        productAreaId: selectedManagementFeatureNode.product_area_id ?? undefined,
        capabilityId: selectedManagementFeatureNode.capability_id ?? undefined,
        sourceNodeId: selectedManagementFeatureNode.id,
        sourceNodeType: selectedManagementFeatureNode.node_type,
        parentWorkItemId: selectedManagementStory.id,
        title: taskDraft.title.trim(),
        problemStatement: taskDraft.problemStatement.trim(),
        description: taskDraft.description.trim(),
        acceptanceCriteria: taskDraft.acceptanceCriteria.trim(),
        constraints: taskDraft.constraints.trim(),
        workItemType: "task",
        priority: taskDraft.priority,
        complexity: taskDraft.complexity,
      });
    },
    onSuccess: async () => {
      await invalidateTasks();
      setTaskDraft(emptyWorkItemDraft);
      setTaskDialogMode("closed");
      setFormError(null);
    },
    onError: (error) => setFormError(String(error)),
  });

  const updateManagementTaskMutation = useMutation({
    mutationFn: () => {
      if (!editingTask) {
        throw new Error("Select a task before editing.");
      }
      return updateWorkItem({
        id: editingTask.id,
        title: taskDraft.title.trim(),
        status: taskDraft.status,
        problemStatement: taskDraft.problemStatement.trim(),
        description: taskDraft.description.trim(),
        acceptanceCriteria: taskDraft.acceptanceCriteria.trim(),
        constraints: taskDraft.constraints.trim(),
      });
    },
    onSuccess: async () => {
      await invalidateTasks();
      setEditingTask(null);
      setTaskDraft(emptyWorkItemDraft);
      setTaskDialogMode("closed");
      setFormError(null);
    },
    onError: (error) => setFormError(String(error)),
  });

  const deleteManagementWorkItemMutation = useMutation({
    mutationFn: async (candidate: { workItem: WorkItem; kind: "story" | "task" }) => {
      if (candidate.kind === "story") {
        for (;;) {
          const childTasks = await getSubWorkItems(candidate.workItem.id, {
            limit: SUB_WORK_ITEM_PAGE_SIZE,
            offset: 0,
          });
          if (childTasks.length === 0) {
            break;
          }
          await Promise.all(childTasks.map((workItem) => deleteWorkItem(workItem.id)));
          if (childTasks.length < SUB_WORK_ITEM_PAGE_SIZE) {
            break;
          }
        }
      }
      await deleteWorkItem(candidate.workItem.id);
    },
    onSuccess: async () => {
      await invalidateTasks();
      setDeleteWorkItemCandidate(null);
      setDeleteWorkItemConfirmName("");
      setDeleteWorkItemConfirmChecked(false);
      setSelectedManagementStoryId(null);
      setFormError(null);
    },
    onError: (error) => setFormError(String(error)),
  });

  const createWorkspaceMutation = useMutation({
    mutationFn: () =>
      createLocalWorkspace({
        productId: selectedProductId,
        productAreaId: activeProductAreaId,
      }),
    onSuccess: async (provisioned) => {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["repositories"] }),
        queryClient.invalidateQueries({ queryKey: ["productScopeRepo", selectedProductId, activeProductAreaId] }),
        queryClient.invalidateQueries({ queryKey: ["ideScopeRepo"] }),
        queryClient.invalidateQueries({ queryKey: ["sidebarProductTree", selectedProductId] }),
      ]);
      setWorkspaceActionError(null);
      setWorkspaceActionMsg(`Workspace ready at ${provisioned.created_path}. Opening IDE.`);
      setActiveView("ide");
      navigate("/ide");
      useWorkspaceStore.getState().setActiveWorkspace(provisioned.created_path);
      useWorkspaceStore.getState().setActiveRepo(provisioned.repository.id);
    },
    onError: (error) => {
      setWorkspaceActionMsg(null);
      setWorkspaceActionError(String(error));
    },
  });

  const capabilityCount = tree ? countCapabilities(tree.product_areas) : 0;
  const totalNodeCount = tree ? countHierarchyNodes(tree.roots) : 0;
  const selectedHierarchyNode = useMemo(
    () => (tree ? findHierarchyNode(tree.roots, activeNodeId, activeNodeType) : null),
    [tree, activeNodeId, activeNodeType],
  );
  const selectedHierarchyPath = useMemo(
    () => (tree ? findHierarchyNodePath(tree.roots, activeNodeId, activeNodeType) : []),
    [tree, activeNodeId, activeNodeType],
  );
  const allTreeNodes = useMemo(() => (tree ? flattenHierarchyNodes(tree.roots) : []), [tree]);
  const canonicalManagementNodeCount = useMemo(
    () => allTreeNodes.filter((node) => node.node_kind === "product_area" || node.node_kind === "capability" || node.node_kind === "feature").length,
    [allTreeNodes],
  );
  const selectedCapabilityOptions = useMemo(
    () => allTreeNodes
      .filter((node) => node.node_type === "capability")
      .map((node) => ({ id: node.id, label: node.path.join(" / ") })),
    [allTreeNodes],
  );
  const dependencyTargetCapabilityOptions = useMemo(() => {
    const targetTree = productTreeById.get(dependencyDraft.dependsOnProductId);
    return targetTree ? flattenHierarchyNodes(targetTree.roots)
      .filter((node) => node.node_type === "capability")
      .map((node) => ({ id: node.id, label: node.path.join(" / ") })) : [];
  }, [dependencyDraft.dependsOnProductId, productTreeById]);
  const selectedProductDependencies = useMemo(
    () => productDependencies.filter((dependency) => dependency.product_id === selectedProductId),
    [productDependencies, selectedProductId],
  );
  const productNameById = useMemo(
    () => new Map((products ?? []).map((product) => [product.id, product.name])),
    [products],
  );
  const capabilityLabelById = useMemo(() => {
    const map = new Map<string, string>();
    productTreeById.forEach((productTree) => {
      flattenHierarchyNodes(productTree.roots)
        .filter((node) => node.node_type === "capability")
        .forEach((node) => map.set(node.id, node.path.join(" / ")));
    });
    allTreeNodes
      .filter((node) => node.node_type === "capability")
      .forEach((node) => map.set(node.id, node.path.join(" / ")));
    return map;
  }, [allTreeNodes, productTreeById]);
  const nodeLookup = useMemo(
    () => new Map(allTreeNodes.map((node) => [getHierarchyNodeKey(node), node])),
    [allTreeNodes],
  );
  const selectedNodeKey = activeNodeId && activeNodeType ? `${activeNodeType}:${activeNodeId}` : null;
  const outlineNodeKindOptions = useMemo(
    () => orderHierarchyNodeKinds(Array.from(new Set(allTreeNodes.map((node) => node.node_kind)))),
    [allTreeNodes],
  );
  const outlineNodeKindGroups = useMemo(() => groupHierarchyNodeKinds(outlineNodeKindOptions), [outlineNodeKindOptions]);
  const hasOutlineFilter = outlineSearchTerm.trim().length > 0 || outlineKindFilter.length > 0;
  const filteredOutlineRoots = useMemo(() => {
    if (!tree) {
      return [];
    }
    if (!hasOutlineFilter) {
      return tree.roots;
    }

    const normalizedSearch = outlineSearchTerm.trim().toLowerCase();
    const filterNode = (node: HierarchyTreeNode): HierarchyTreeNode | null => {
      const childMatches = node.children
        .map(filterNode)
        .filter(Boolean) as HierarchyTreeNode[];
      const matchesSearch = normalizedSearch.length === 0
        || [node.name, ...node.path, node.description, node.summary].join(" ").toLowerCase().includes(normalizedSearch);
      const matchesKind = !outlineKindFilter || node.node_kind === outlineKindFilter;
      if ((matchesSearch && matchesKind) || childMatches.length > 0) {
        return {
          ...node,
          children: childMatches,
        };
      }
      return null;
    };

    return tree.roots
      .map(filterNode)
      .filter(Boolean) as HierarchyTreeNode[];
  }, [hasOutlineFilter, outlineKindFilter, outlineSearchTerm, tree]);
  const selectedNodeKind = selectedHierarchyNode?.node_kind ?? selectedCapability?.node_kind ?? selectedProductArea?.node_kind ?? null;
  const selectedNodeTitle = selectedHierarchyNode?.name ?? selectedProduct?.name ?? "Product";
  const selectedNodeSummary = selectedHierarchyNode?.summary
    || selectedHierarchyNode?.description
    || selectedProduct?.description
    || "Add a durable description so the selected node reads like a documented section instead of a blank planning shell.";
  const selectedScopePath = selectedProduct
    ? [selectedProduct.name, ...selectedHierarchyPath.map((node) => node.name)]
    : [];
  const selectedNodeEntityLabel = selectedCapability
    ? getHierarchyNodeKindLabel(selectedCapability.node_kind)
    : selectedProductArea
      ? getHierarchyNodeKindLabel(selectedProductArea.node_kind)
      : "Product";
  const selectedAllowedChildKinds = selectedHierarchyNode ? selectedHierarchyNode.allowed_child_kinds : ROOT_NODE_KINDS;
  const selectedChildNodeKinds = selectedHierarchyNode ? selectedAllowedChildKinds : [];
  const canCreateChildCapability = selectedChildNodeKinds.length > 0;
  const nextCapabilityEntityLabel = selectedChildNodeKinds.length > 0
    ? getHierarchyNodeKindLabel(getDefaultChildNodeKind(selectedNodeKind))
    : "Child Node";
  const selectedDirectChildren = useMemo(
    () => getDirectChildNodes(tree, selectedHierarchyNode),
    [tree, selectedHierarchyNode],
  );
  const selectedReferenceScope = useMemo(() => {
    if (!selectedProductId) {
      return null;
    }
    if (selectedHierarchyNode?.node_type === "product_area") {
      return getProductAreaReferenceScope(selectedHierarchyNode.id);
    }
    if (selectedHierarchyNode?.node_type === "capability" && selectedHierarchyNode.capability_id) {
      return {
        scopeType: selectedHierarchyNode.node_kind === "feature" ? "feature" as const : "capability" as const,
        scopeId: selectedHierarchyNode.capability_id,
      };
    }
    return { scopeType: "product" as const, scopeId: selectedProductId };
  }, [selectedHierarchyNode, selectedProductId]);
  const selectedReferences = useMemo(
    () => selectedReferenceScope
      ? productReferences.filter((reference) => reference.scope_type === selectedReferenceScope.scopeType && reference.scope_id === selectedReferenceScope.scopeId)
      : [],
    [productReferences, selectedReferenceScope],
  );
  const selectedMetricCards = selectedHierarchyNode
    ? [
        { label: "Direct Children", value: selectedDirectChildren.length, help: `${selectedDirectChildren.length} immediate child ${selectedDirectChildren.length === 1 ? "node" : "nodes"}` },
        { label: "Subtree Nodes", value: countDescendantNodes(selectedHierarchyNode) + 1, help: "Selected node plus all nested descendants" },
        { label: "References", value: selectedReferences.length, help: "Attached context for this management scope" },
        { label: "Dependencies", value: selectedProductDependencies.length, help: "Cross-product dependencies for this product" },
      ]
    : [
        { label: "Product Areas", value: tree?.roots.filter((node) => node.node_kind === "product_area").length ?? 0, help: "Top-level product management areas" },
        { label: "Management Nodes", value: canonicalManagementNodeCount, help: "Product areas, capabilities, and features" },
        { label: "References", value: selectedReferences.length, help: "Attached product context" },
        { label: "Dependencies", value: selectedProductDependencies.length, help: "Cross-product dependencies" },
      ];
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
  const selectedManagementCapabilityTree = useMemo(() => {
    const selectedTopLevelCapability = managementCapabilities.find((capabilityTree) => capabilityTree.capability.id === activeCapabilityId);
    if (selectedTopLevelCapability) {
      return selectedTopLevelCapability;
    }
    if (selectedCapability?.parent_capability_id) {
      return managementCapabilities.find((capabilityTree) => capabilityTree.capability.id === selectedCapability.parent_capability_id) ?? managementCapabilities[0] ?? null;
    }
    return managementCapabilities[0] ?? null;
  }, [activeCapabilityId, managementCapabilities, selectedCapability?.parent_capability_id]);
  const managementFeatures = useMemo(
    () => selectedManagementCapabilityTree
      ? getOrderedCapabilityTrees(
          selectedManagementCapabilityTree.children,
          capabilityOrderMap[getCapabilityOrderKey(selectedManagementCapabilityTree.capability.product_area_id, selectedManagementCapabilityTree.capability.id)],
        ).filter((capabilityTree) => capabilityTree.capability.node_kind === "feature")
      : [],
    [capabilityOrderMap, selectedManagementCapabilityTree],
  );
  const allManagementFeatures = useMemo(
    () => productAreaProductAreas.flatMap((productAreaTree) =>
      flattenCapabilityTreeList(productAreaTree.features)
        .filter((capabilityTree) => capabilityTree.capability.node_kind === "feature")
        .map((capabilityTree) => ({
          capabilityTree,
          productArea: productAreaTree.product_area,
          parentCapability: capabilityTree.capability.parent_capability_id
            ? findCapabilityTree(productAreaProductAreas, capabilityTree.capability.parent_capability_id)?.capability ?? null
            : null,
        })),
    ),
    [productAreaProductAreas],
  );
  const selectedManagementFeature = useMemo(() => {
    const activeFeature = allManagementFeatures.find((entry) => entry.capabilityTree.capability.id === activeCapabilityId);
    if (activeFeature) {
      return activeFeature;
    }
    return allManagementFeatures[0] ?? null;
  }, [activeCapabilityId, allManagementFeatures]);
  const selectedManagementFeatureNode = useMemo(
    () => selectedManagementFeature ? findHierarchyNode(tree?.roots ?? [], selectedManagementFeature.capabilityTree.capability.id, "capability") : null,
    [selectedManagementFeature, tree],
  );
  useEffect(() => {
    setManagementStoryPageIndex(0);
    setSelectedManagementStoryId(null);
  }, [selectedManagementFeatureNode?.id, selectedProductId, productManagementTab]);

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
  const featureStories = useMemo(() => {
    if (!selectedManagementFeatureNode) {
      return [];
    }
    return getDirectWorkItemsForNode(selectedManagementFeatureNode, managementFeatureWorkItems)
      .filter((workItem) => !workItem.parent_work_item_id);
  }, [managementFeatureWorkItems, selectedManagementFeatureNode]);
  const selectedManagementStory = useMemo(
    () => featureStories.find((workItem) => workItem.id === selectedManagementStoryId)
      ?? featureStories.find((workItem) => workItem.id === activeWorkItemId)
      ?? featureStories[0]
      ?? null,
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
    () => {
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
    },
    [managementFeatureWorkItems, selectedManagementStory, selectedManagementStoryTasks],
  );
  const refreshProductManagementTabQueries = async () => {
    await refreshScopedProductQueries(queryClient, getProductManagementRefreshQueryKeys({
      selectedProductId,
      productManagementTab,
      selectedManagementStoryIdForTasks,
    }));
  };

  const refreshActiveProductPageTab = async () => {
    await refreshScopedProductQueries(queryClient, getProductPageRefreshQueryKeys({
      productPageTab,
      selectedProductId,
      statusGroupBy,
      statusProductId,
      hideExampleProductsKey: HIDE_EXAMPLE_PRODUCTS_KEY,
      productManagementTab,
      selectedManagementStoryIdForTasks,
    }));
  };

  const activeProductPageRefreshLabel = getProductPageRefreshLabel(productPageTab);
  const activeProductPageRefreshDisabled = isProductPageRefreshDisabled(productPageTab, selectedProductId);
  const productManagementRefreshLabel = getProductManagementRefreshLabel(productManagementTab);

  const structureRows = useMemo(() => {
    if (!tree) {
      return [];
    }
    return selectedDirectChildren.map((node) => ({
      id: node.id,
      name: node.name,
      subtitle: node.summary || node.description || getHierarchyNodeKindLabel(node.node_kind),
      type: getHierarchyNodeKindLabel(node.node_kind, { lowercase: true }),
      directChildren: node.children.length,
      references: productReferences.filter((reference) => {
        const scopeType = node.node_type === "product_area" ? "product_area" : node.node_kind === "feature" ? "feature" : "capability";
        const scopeId = node.node_type === "capability" && node.capability_id ? node.capability_id : node.id;
        return reference.scope_type === scopeType && reference.scope_id === scopeId;
      }).length,
      onSelect: () => {
        setActiveHierarchyNode({
          nodeId: node.id,
          nodeType: node.node_type,
          productAreaId: node.product_area_id,
          capabilityId: node.capability_id,
        });
      },
      onEdit: () => {
        setActiveHierarchyNode({
          nodeId: node.id,
          nodeType: node.node_type,
          productAreaId: node.product_area_id,
          capabilityId: node.capability_id,
        });
        if (node.node_type === "product_area") {
          const productAreaMatch = tree.product_areas.find((productAreaTree) => productAreaTree.product_area.id === node.id)?.product_area;
          if (!productAreaMatch) {
            return;
          }
          setProductAreaDraft({
            name: productAreaMatch.name,
            description: productAreaMatch.description,
            purpose: productAreaMatch.purpose,
            nodeKind: productAreaMatch.node_kind,
          });
          useUIStore.getState().openProductAreaDialog("edit");
          return;
        }
        const capabilityMatch = findCapabilityTree(tree.product_areas, node.id)?.capability;
        if (!capabilityMatch) {
          return;
        }
        setCapabilityDraft({
          name: capabilityMatch.name,
          description: capabilityMatch.description,
          acceptanceCriteria: capabilityMatch.acceptance_criteria,
          technicalNotes: capabilityMatch.technical_notes,
          nodeKind: capabilityMatch.node_kind,
        });
        useUIStore.getState().openCapabilityDialog("edit");
      },
    }));
  }, [productReferences, selectedDirectChildren, selectedProductId, setActiveHierarchyNode, tree]);

  const openSelectedSectionInBook = () => {
    if (!selectedProductId) {
      return;
    }
    setActiveProduct(selectedProductId);
    setActiveView("product-overview");
    navigate(`/product-overview#${getHierarchyNodeSectionId(selectedHierarchyNode)}`);
  };

  const editSelectedScope = () => {
    if (!selectedProduct) {
      return;
    }
    if (!selectedHierarchyNode) {
      openProductDialog("edit");
      return;
    }
    if (selectedHierarchyNode.node_type === "product_area") {
      openProductAreaDialog("edit");
      return;
    }
    openCapabilityDialog("edit");
  };

  const openCreateInSelectedScope = () => {
    if (!selectedHierarchyNode) {
      useUIStore.getState().openProductAreaDialog("create");
      return;
    }
    if (!canCreateChildCapability) {
      return;
    }
    useUIStore.getState().openCapabilityDialog("create");
  };

  const selectProductArea = (productAreaTree: ProductAreaTree) => {
    setActiveHierarchyNode({
      nodeId: productAreaTree.product_area.id,
      nodeType: "product_area",
      productAreaId: productAreaTree.product_area.id,
      capabilityId: null,
    });
  };

  const selectCapabilityForManagement = (capabilityTree: CapabilityTree) => {
    setActiveHierarchyNode({
      nodeId: capabilityTree.capability.id,
      nodeType: "capability",
      productAreaId: capabilityTree.capability.product_area_id,
      capabilityId: capabilityTree.capability.id,
    });
  };

  const openCreateCapabilityForArea = (productAreaTree: ProductAreaTree) => {
    selectProductArea(productAreaTree);
    setCapabilityForm({ name: "", description: "", acceptanceCriteria: "", technicalNotes: "", nodeKind: "capability" });
    openCapabilityDialog("create");
  };

  const openCreateFeatureForCapability = (capabilityTree: CapabilityTree) => {
    selectCapabilityForManagement(capabilityTree);
    setCapabilityForm({ name: "", description: "", acceptanceCriteria: "", technicalNotes: "", nodeKind: "feature" });
    openCapabilityDialog("create");
  };

  const openEditProductArea = (productAreaTree: ProductAreaTree) => {
    selectProductArea(productAreaTree);
    setProductAreaDraft({
      name: productAreaTree.product_area.name,
      description: productAreaTree.product_area.description,
      purpose: productAreaTree.product_area.purpose,
      nodeKind: productAreaTree.product_area.node_kind,
    });
    openProductAreaDialog("edit");
  };

  const openEditCapabilityNode = (capabilityTree: CapabilityTree) => {
    selectCapabilityForManagement(capabilityTree);
    setCapabilityDraft({
      name: capabilityTree.capability.name,
      description: capabilityTree.capability.description,
      acceptanceCriteria: capabilityTree.capability.acceptance_criteria,
      technicalNotes: capabilityTree.capability.technical_notes,
      nodeKind: capabilityTree.capability.node_kind,
    });
    openCapabilityDialog("edit");
  };

  const requestDeleteHierarchyNode = (candidate: NonNullable<typeof deleteHierarchyCandidate>) => {
    setDeleteHierarchyCandidate(candidate);
    setDeleteHierarchyConfirmName("");
    setDeleteHierarchyConfirmChecked(false);
    setFormError(null);
  };

  const openFeatureInBuilder = (featureNode: HierarchyTreeNode | null) => {
    if (featureNode) {
      setActiveHierarchyNode({
        nodeId: featureNode.id,
        nodeType: featureNode.node_type,
        productAreaId: featureNode.product_area_id,
        capabilityId: featureNode.capability_id,
      });
    }
    setActiveView("work-items");
    navigate("/work-items");
  };

  const openStoryInBuilder = (story: WorkItem) => {
    setSelectedManagementStoryId(story.id);
    setActiveWorkItem(story.id);
    setActiveView("work-items");
    navigate("/work-items");
  };

  useEffect(() => {
    if (!selectedNodeKey) {
      return;
    }
    setRecentNodeKeys((current) => [selectedNodeKey, ...current.filter((key) => key !== selectedNodeKey)].slice(0, 6));
  }, [selectedNodeKey]);

  const setOutlineNodeExpandedState = (node: HierarchyTreeNode, expanded: boolean) => {
    if (node.node_type === "product_area") {
      setProductAreaExpanded(node.id, expanded);
      return;
    }
    setCapabilityExpanded(node.id, expanded);
  };

  const collapseOutlineNodes = () => {
    allTreeNodes.forEach((node) => setOutlineNodeExpandedState(node, false));
  };

  const expandSelectedOutlinePath = () => {
    selectedHierarchyPath.forEach((node) => setOutlineNodeExpandedState(node, true));
  };

  const jumpToSelectedOutlineNode = () => {
    if (!selectedNodeKey) {
      return;
    }
    expandSelectedOutlinePath();
    requestAnimationFrame(() => {
      outlineNodeRefs.current[selectedNodeKey]?.scrollIntoView({ block: "center", behavior: "smooth" });
    });
  };

  const openOutlineNode = (node: HierarchyTreeNode) => {
    setActiveHierarchyNode({
      nodeId: node.id,
      nodeType: node.node_type,
      productAreaId: node.product_area_id,
      capabilityId: node.capability_id,
    });
    setProductWorkspaceTab("structure");
  };

  const createChildForOutlineNode = (node: HierarchyTreeNode) => {
    openOutlineNode(node);
    openCapabilityDialog("create");
  };

  const editOutlineNode = (node: HierarchyTreeNode) => {
    openOutlineNode(node);
    if (node.node_type === "product_area") {
      openProductAreaDialog("edit");
      return;
    }
    openCapabilityDialog("edit");
  };

  const copyEntityId = async (id: string) => {
    try {
      await navigator.clipboard.writeText(id);
      setCopiedEntityId(id);
      setTimeout(() => setCopiedEntityId((current) => current === id ? null : current), 1800);
    } catch {
      setCopiedEntityId(null);
    }
  };

  const renderCopyableEntityId = (label: string, id: string) => {
    const displayId = id.length > 18 ? `${id.slice(0, 8)}...${id.slice(-6)}` : id;
    const isCopied = copiedEntityId === id;

    return (
      <div style={styles.copyIdRow} title={id}>
        <span style={styles.copyIdLabel}>{label}</span>
        <span style={styles.copyIdValue}>{displayId}</span>
        <button
          type="button"
          style={styles.copyIdButton}
          onClick={(event) => {
            event.preventDefault();
            event.stopPropagation();
            void copyEntityId(id);
          }}
        >
          {isCopied ? "Copied" : "Copy"}
        </button>
      </div>
    );
  };

  const renderOutlineNode = (node: HierarchyTreeNode, depth = 0): React.ReactNode => {
    const nodeKey = getHierarchyNodeKey(node);
    const isActive = selectedNodeKey === nodeKey;
    const isExpanded = hasOutlineFilter
      ? true
      : node.node_type === "product_area"
        ? expandedProductAreas[node.id] ?? true
        : expandedCapabilities[node.id] ?? true;
    return (
      <div key={nodeKey}>
        <div
          ref={(element) => {
            outlineNodeRefs.current[nodeKey] = element;
          }}
          style={{
            ...(isActive ? styles.outlineNodeActive : styles.outlineNode),
            marginLeft: depth * 10,
          }}
        >
          <div style={styles.outlineNodeHeader}>
            {node.children.length > 0 ? (
              <button
                style={styles.outlineToggle}
                onClick={(event) => {
                  event.stopPropagation();
                  if (node.node_type === "product_area") {
                    toggleProductAreaExpanded(node.id);
                  } else {
                    toggleCapabilityExpanded(node.id);
                  }
                }}
              >
                {isExpanded ? "-" : "+"}
              </button>
            ) : (
              <div style={styles.outlineToggle}>.</div>
            )}
            <div style={styles.outlineNodeBody} onClick={() => openOutlineNode(node)}>
              <div style={styles.outlineNodeTitle}>{node.name}</div>
              <div style={styles.outlineNodeMeta}>
                {getHierarchyNodeKindLabel(node.node_kind)} · {node.children.length} {node.children.length === 1 ? "child" : "children"}
              </div>
              {node.summary || node.description ? <div style={styles.outlineNodeMeta}>{node.summary || node.description}</div> : null}
            </div>
          </div>
          <div style={styles.outlineActionRow}>
            {supportsHierarchyChildren(node.node_kind) ? (
              <button
                style={styles.outlineActionBtn}
                onClick={(event) => {
                  event.stopPropagation();
                  createChildForOutlineNode(node);
                }}
              >
                + Child Node
              </button>
            ) : null}
            <button
              style={styles.outlineActionBtn}
              onClick={(event) => {
                event.stopPropagation();
                editOutlineNode(node);
              }}
            >
              Edit
            </button>
          </div>
        </div>
        {isExpanded && node.children.length > 0 ? (
          <div style={styles.outlineChildWrap}>
            {node.children.map((child) => renderOutlineNode(child, depth + 1))}
          </div>
        ) : null}
      </div>
    );
  };

  const editProductFromList = (product: Product) => {
    setActiveProduct(product.id);
    setProductDraft(productToForm(product));
    setFormError(null);
    openProductDialog("edit");
  };

  const openProductDesign = (product: Product) => {
    setActiveProduct(product.id);
    setStatusProductId(product.id);
    setProductPageTab("design");
    navigate(`/products/${product.id}`);
  };

  const openProductOverview = (product: Product) => {
    setActiveProduct(product.id);
    setStatusProductId(product.id);
    setProductPageTab("overview");
  };

  const openProductStatus = (product: Product) => {
    setActiveProduct(product.id);
    setStatusProductId(product.id);
    setProductPageTab("status");
  };

  const openProductDependencies = (product: Product) => {
    setActiveProduct(product.id);
    setStatusProductId(product.id);
    setProductPageTab("dependencies");
  };

  const requestArchiveProduct = (product: Product) => {
    setDeleteProductCandidate(product);
    setDeleteConfirmName("");
    setDeleteConfirmArchive(false);
    setFormError(null);
  };

  const requestResetProductPlan = (product: Product) => {
    setResetPlanCandidate(product);
    setResetPlanConfirmName("");
    setResetPlanConfirmTree(false);
    setResetPlanDeleteDelivery(false);
    setFormError(null);
  };

  const openCreateStoryDialog = () => {
    setEditingStory(null);
    setStoryDraft(emptyWorkItemDraft);
    setStoryDialogMode("create");
    setFormError(null);
  };

  const openEditStoryDialog = (story: WorkItem) => {
    setEditingStory(story);
    setStoryDraft(workItemToDraft(story));
    setStoryDialogMode("edit");
    setFormError(null);
  };

  const openCreateTaskDialog = () => {
    setEditingTask(null);
    setTaskDraft(emptyWorkItemDraft);
    setTaskDialogMode("create");
    setFormError(null);
  };

  const openEditTaskDialog = (task: WorkItem) => {
    setEditingTask(task);
    setTaskDraft(workItemToDraft(task));
    setTaskDialogMode("edit");
    setFormError(null);
  };

  const requestDeleteWorkItem = (workItem: WorkItem, kind: "story" | "task") => {
    setDeleteWorkItemCandidate({ workItem, kind });
    setDeleteWorkItemConfirmName("");
    setDeleteWorkItemConfirmChecked(false);
    setFormError(null);
  };

  const deleteConfirmationReady = !!deleteProductCandidate
    && deleteConfirmName.trim() === deleteProductCandidate.name
    && deleteConfirmArchive;
  const resetPlanReady = !!resetPlanCandidate
    && resetPlanConfirmName.trim() === resetPlanCandidate.name
    && resetPlanConfirmTree;
  const deleteHierarchyReady = !!deleteHierarchyCandidate
    && deleteHierarchyConfirmName.trim() === deleteHierarchyCandidate.name
    && deleteHierarchyConfirmChecked;
  const deleteManagementWorkItemReady = !!deleteWorkItemCandidate
    && deleteWorkItemConfirmName.trim() === deleteWorkItemCandidate.workItem.title
    && deleteWorkItemConfirmChecked;

  const renderProductManagementConsole = () => (
    <div>
      <ProductManagementHeader
        selectedProduct={selectedProduct}
        activeTab={productManagementTab}
        onTabChange={setProductManagementTab}
        refreshLabel={productManagementRefreshLabel}
        onRefresh={refreshProductManagementTabQueries}
        refreshDisabled={!selectedProductId}
        renderCopyableEntityId={renderCopyableEntityId}
        styles={styles}
      />

      {productManagementTab === "areas" && (
        <ProductManagementAreasTab
          selectedProduct={selectedProduct}
          productAreas={productAreaProductAreas}
          onResetProductPlan={requestResetProductPlan}
          onCreateProductArea={() => openProductAreaDialog("create")}
          onOpenProductArea={(productAreaTree) => {
            selectProductArea(productAreaTree);
            setProductManagementTab("capabilities");
          }}
          onEditProductArea={openEditProductArea}
          onDeleteProductArea={(productAreaTree) => requestDeleteHierarchyNode({
            kind: "product_area",
            id: productAreaTree.product_area.id,
            name: productAreaTree.product_area.name,
          })}
          renderCopyableEntityId={renderCopyableEntityId}
          styles={styles}
        />
      )}

      {productManagementTab === "capabilities" && (
        <ProductManagementCapabilitiesTab
          productAreas={productAreaProductAreas}
          selectedProductAreaTree={selectedProductAreaTree}
          capabilities={managementCapabilities}
          productTree={tree}
          selectedProductId={selectedProductId}
          scopeSummaryIndex={scopeSummaryIndex}
          onSelectProductArea={selectProductArea}
          onCreateCapability={openCreateCapabilityForArea}
          onOpenCapability={(capabilityTree) => {
            selectCapabilityForManagement(capabilityTree);
            setProductManagementTab("features");
          }}
          onEditCapability={openEditCapabilityNode}
          onDeleteCapability={(capabilityTree) => requestDeleteHierarchyNode({
            kind: "capability",
            id: capabilityTree.capability.id,
            name: capabilityTree.capability.name,
          })}
          renderCopyableEntityId={renderCopyableEntityId}
          styles={styles}
        />
      )}

      {productManagementTab === "features" && (
        <ProductManagementFeaturesTab
          capabilities={managementCapabilities}
          selectedCapabilityTree={selectedManagementCapabilityTree}
          features={managementFeatures}
          productTree={tree}
          selectedProductId={selectedProductId}
          scopeSummaryIndex={scopeSummaryIndex}
          onSelectCapability={selectCapabilityForManagement}
          onCreateFeature={openCreateFeatureForCapability}
          onOpenFeatureStories={(featureTree) => {
            selectCapabilityForManagement(featureTree);
            setProductManagementTab("work_items");
          }}
          onEditFeature={openEditCapabilityNode}
          onDeleteFeature={(featureTree) => requestDeleteHierarchyNode({
            kind: "feature",
            id: featureTree.capability.id,
            name: featureTree.capability.name,
          })}
          renderCopyableEntityId={renderCopyableEntityId}
          styles={styles}
        />
      )}

      {productManagementTab === "work_items" && (
        <div style={styles.managementThreePane}>
          <ProductManagementWorkItemFeatureSelector
            features={allManagementFeatures}
            selectedFeature={selectedManagementFeature}
            onSelectFeature={(entry) => {
              selectCapabilityForManagement(entry.capabilityTree);
              setSelectedManagementStoryId(null);
            }}
            renderCopyableEntityId={renderCopyableEntityId}
            styles={styles}
          />
          <ProductManagementStoriesPane
            stories={featureStories}
            selectedStory={selectedManagementStory}
            canCreateStory={!!selectedManagementFeatureNode}
            storyPageIndex={managementStoryPageIndex}
            hasNextStoryPage={managementFeatureWorkItemPage?.has_more ?? false}
            onPreviousStoryPage={() => setManagementStoryPageIndex((current) => Math.max(0, current - 1))}
            onNextStoryPage={() => setManagementStoryPageIndex((current) => current + 1)}
            onCreateStory={openCreateStoryDialog}
            onOpenBuilder={() => openFeatureInBuilder(selectedManagementFeatureNode)}
            onSelectStory={(story) => {
              setSelectedManagementStoryId(story.id);
              setActiveWorkItem(story.id);
            }}
            onEditStory={openEditStoryDialog}
            onDeleteStory={(story) => requestDeleteWorkItem(story, "story")}
            styles={styles}
          />
          <ProductManagementStoryDetailPane
            selectedStory={selectedManagementStory}
            tasks={selectedManagementTasks}
            onEditStory={openEditStoryDialog}
            onOpenStory={openStoryInBuilder}
            onCreateTask={openCreateTaskDialog}
            onEditTask={openEditTaskDialog}
            onDeleteTask={(task) => requestDeleteWorkItem(task, "task")}
            styles={styles}
          />
        </div>
      )}
    </div>
  );

  return (
    <div style={styles.page}>
      <div style={styles.header}>
        <div style={styles.titleBlock}>
          <h1 style={styles.title}>Products</h1>
          <div style={styles.subtitle}>Manage products, shape product areas, capabilities, and features, then track delivery stories and tasks separately.</div>
        </div>
      </div>

      <div style={styles.pageTabs}>
        <div style={styles.pageTabGroup}>
          <span style={styles.pageTabGroupLabel}>Catalog</span>
          <button style={productPageTab === "list" ? styles.pageTabActive : styles.pageTab} onClick={() => setProductPageTab("list")}>Product List</button>
          <button style={productPageTab === "status" ? styles.pageTabActive : styles.pageTab} onClick={() => setProductPageTab("status")}>Product Status</button>
        </div>
        <div style={styles.pageTabGroup}>
          <span style={styles.pageTabGroupLabel}>Selected Product</span>
          <select
            aria-label="Selected product"
            style={styles.pageTabProductSelect}
            value={selectedProductId ?? ""}
            onChange={(event) => {
              const nextProductId = event.target.value || null;
              setActiveProduct(nextProductId);
              if (nextProductId && (productPageTab === "list" || productPageTab === "status")) {
                setProductPageTab("overview");
              }
            }}
          >
            <option value="">Select product</option>
            {(products ?? []).map((product) => <option key={product.id} value={product.id}>{product.name}</option>)}
          </select>
          <button style={productPageTab === "overview" ? styles.pageTabActive : styles.pageTab} onClick={() => setProductPageTab("overview")} disabled={!selectedProduct}>Product Overview</button>
          <button style={productPageTab === "design" ? styles.pageTabActive : styles.pageTab} onClick={() => setProductPageTab("design")} disabled={!selectedProduct}>Product Management</button>
          <button style={productPageTab === "dependencies" ? styles.pageTabActive : styles.pageTab} onClick={() => setProductPageTab("dependencies")} disabled={!selectedProduct}>Dependencies</button>
        </div>
        <div style={styles.tabRefreshSlot}>
          <ScopedRefreshButton
            label={activeProductPageRefreshLabel}
            onRefresh={refreshActiveProductPageTab}
            disabled={activeProductPageRefreshDisabled}
          />
        </div>
      </div>

      <div style={styles.workspace}>
        <div style={styles.panel}>
          <div style={styles.panelInner}>
            {productPageTab === "list" ? (
              <ProductCatalogTab
                productSearch={productSearch}
                productStatusFilter={productStatusFilter}
                productSourceFilter={productSourceFilter}
                productTagFilter={productTagFilter}
                productSort={productSort}
                allProductTags={allProductTags}
                showCustomProductsInTable={showCustomProductsInTable}
                showDefaultProductsInTable={showDefaultProductsInTable}
                includeDefaultProductsInCatalog={includeDefaultProductsInCatalog}
                catalogFilterMsg={catalogFilterMsg}
                catalogFilterError={catalogFilterError}
                productTableRows={productTableRows}
                isLoading={isLoading}
                onProductSearchChange={setProductSearch}
                onProductStatusFilterChange={setProductStatusFilter}
                onProductSourceFilterChange={setProductSourceFilter}
                onProductTagFilterChange={setProductTagFilter}
                onProductSortChange={setProductSort}
                onShowCustomProductsInTableChange={setShowCustomProductsInTable}
                onShowDefaultProductsInTableChange={setShowDefaultProductsInTable}
                onIncludeDefaultProductsInCatalogChange={updateDefaultProductVisibility}
                onAddProduct={() => openProductDialog("create")}
                onEditProduct={editProductFromList}
                onOpenProductStatus={openProductStatus}
                onOpenProductOverview={openProductOverview}
                onOpenProductDesign={openProductDesign}
                onOpenProductDependencies={openProductDependencies}
                onDeleteProduct={requestArchiveProduct}
                styles={styles}
              />
            ) : productPageTab === "status" ? (
              <ProductStatusTab
                products={products ?? []}
                statusProductId={statusProductId}
                statusDepth={statusDepth}
                statusGroupBy={statusGroupBy}
                statusSummary={statusSummary}
                statusRows={statusRows}
                isLoading={isLoading}
                onStatusProductChange={(nextProductId) => {
                  setStatusProductId(nextProductId);
                  if (nextProductId !== "all") {
                    setActiveProduct(nextProductId);
                  }
                }}
                onStatusDepthChange={setStatusDepth}
                onStatusGroupByChange={setStatusGroupBy}
                onOpenStatusRow={(row) => {
                  if (row.productId) {
                    setActiveProduct(row.productId);
                  }
                  if (row.nodeId && row.nodeType) {
                    setActiveHierarchyNode({
                      nodeId: row.nodeId,
                      nodeType: row.nodeType,
                      productAreaId: row.productAreaId ?? null,
                      capabilityId: row.capabilityId ?? null,
                    });
                    setProductPageTab("design");
                  }
                }}
                styles={styles}
              />
            ) : productPageTab === "overview" ? (
              selectedProduct ? (
                <ProductOverviewPage />
              ) : (
                <div style={styles.empty}>Select a product to view the product overview.</div>
              )
            ) : productPageTab === "dependencies" ? (
              selectedProduct ? (
                <ProductDependenciesTab
                  selectedProduct={selectedProduct}
                  products={products ?? []}
                  selectedProductId={selectedProductId}
                  dependencyDraft={dependencyDraft}
                  setDependencyDraft={setDependencyDraft}
                  selectedCapabilityOptions={selectedCapabilityOptions}
                  dependencyTargetCapabilityOptions={dependencyTargetCapabilityOptions}
                  selectedProductDependencies={selectedProductDependencies}
                  productNameById={productNameById}
                  capabilityLabelById={capabilityLabelById}
                  isCreatingDependency={createProductDependencyMutation.isPending}
                  onCreateDependency={() => createProductDependencyMutation.mutate()}
                  styles={styles}
                />
              ) : (
                <div style={styles.empty}>Select a product before editing dependencies.</div>
              )
            ) : selectedProduct ? (
              renderProductManagementConsole()
            ) : (
              <div style={styles.empty}>
                {isLoading
                  ? "Loading products..."
                  : products && products.length > 0
                    ? "Select a product from Product List to start refining the management tree."
                    : "No visible products yet. Use Add Product or disable Hide Example Products in Settings."}
              </div>
            )}
          </div>
        </div>
      </div>

      {productDialogMode !== "closed" && (
        <ProductFormModal
          mode={productDialogMode}
          productForm={productForm}
          productDraft={productDraft}
          setProductForm={setProductForm}
          setProductDraft={setProductDraft}
          formError={formError}
          isCreatePending={createProductMutation.isPending}
          isUpdatePending={updateProductMutation.isPending}
          onClose={closeProductDialog}
          onSubmit={() => (productDialogMode === "create" ? createProductMutation.mutate() : updateProductMutation.mutate())}
        />
      )}

      {deleteProductCandidate && (
        <DeleteProductModal
          product={deleteProductCandidate}
          confirmName={deleteConfirmName}
          confirmArchive={deleteConfirmArchive}
          isReady={deleteConfirmationReady}
          isPending={archiveMutation.isPending}
          formError={formError}
          onClose={() => setDeleteProductCandidate(null)}
          onConfirmNameChange={setDeleteConfirmName}
          onConfirmArchiveChange={setDeleteConfirmArchive}
          onArchive={(productId) => archiveMutation.mutate(productId)}
        />
      )}

      {resetPlanCandidate && (
        <ResetProductPlanModal
          product={resetPlanCandidate}
          confirmName={resetPlanConfirmName}
          confirmTree={resetPlanConfirmTree}
          deleteDelivery={resetPlanDeleteDelivery}
          isReady={resetPlanReady}
          isPending={resetProductPlanMutation.isPending}
          formError={formError}
          onClose={() => setResetPlanCandidate(null)}
          onConfirmNameChange={setResetPlanConfirmName}
          onConfirmTreeChange={setResetPlanConfirmTree}
          onDeleteDeliveryChange={setResetPlanDeleteDelivery}
          onReset={(data) => resetProductPlanMutation.mutate(data)}
        />
      )}

      {deleteHierarchyCandidate && (
        <DeleteHierarchyNodeModal
          candidate={deleteHierarchyCandidate}
          confirmName={deleteHierarchyConfirmName}
          confirmChecked={deleteHierarchyConfirmChecked}
          isReady={deleteHierarchyReady}
          isPending={deleteHierarchyMutation.isPending}
          formError={formError}
          onClose={() => setDeleteHierarchyCandidate(null)}
          onConfirmNameChange={setDeleteHierarchyConfirmName}
          onConfirmCheckedChange={setDeleteHierarchyConfirmChecked}
          onDelete={(candidate) => deleteHierarchyMutation.mutate(candidate)}
        />
      )}

      {deleteWorkItemCandidate && (
        <DeleteManagementWorkItemModal
          candidate={deleteWorkItemCandidate}
          confirmName={deleteWorkItemConfirmName}
          confirmChecked={deleteWorkItemConfirmChecked}
          isReady={deleteManagementWorkItemReady}
          isPending={deleteManagementWorkItemMutation.isPending}
          formError={formError}
          onClose={() => setDeleteWorkItemCandidate(null)}
          onConfirmNameChange={setDeleteWorkItemConfirmName}
          onConfirmCheckedChange={setDeleteWorkItemConfirmChecked}
          onDelete={(candidate) => deleteManagementWorkItemMutation.mutate(candidate)}
        />
      )}

      {storyDialogMode !== "closed" && (
        <ManagementWorkItemFormModal
          kind="story"
          mode={storyDialogMode}
          contextLabel="Feature"
          contextTitle={selectedManagementFeature?.capabilityTree.capability.name ?? "No feature selected"}
          draft={storyDraft}
          setDraft={setStoryDraft}
          canSubmit={!!selectedManagementFeatureNode}
          isCreatePending={createManagementStoryMutation.isPending}
          isUpdatePending={updateManagementStoryMutation.isPending}
          formError={formError}
          onClose={() => setStoryDialogMode("closed")}
          onSubmit={() => storyDialogMode === "edit" ? updateManagementStoryMutation.mutate() : createManagementStoryMutation.mutate()}
        />
      )}

      {taskDialogMode !== "closed" && (
        <ManagementWorkItemFormModal
          kind="task"
          mode={taskDialogMode}
          contextLabel="Story"
          contextTitle={selectedManagementStory?.title ?? "No story selected"}
          draft={taskDraft}
          setDraft={setTaskDraft}
          canSubmit={!!selectedManagementStory}
          isCreatePending={createManagementTaskMutation.isPending}
          isUpdatePending={updateManagementTaskMutation.isPending}
          formError={formError}
          onClose={() => setTaskDialogMode("closed")}
          onSubmit={() => taskDialogMode === "edit" ? updateManagementTaskMutation.mutate() : createManagementTaskMutation.mutate()}
        />
      )}

      {productAreaDialogMode !== "closed" && (
        <ProductAreaFormModal
          mode={productAreaDialogMode}
          selectedProductArea={selectedProductArea}
          form={productAreaForm}
          draft={productAreaDraft}
          setForm={setProductAreaForm}
          setDraft={setProductAreaDraft}
          formError={formError}
          selectedProductId={selectedProductId}
          isCreatePending={createProductAreaMutation.isPending}
          isUpdatePending={updateProductAreaMutation.isPending}
          onClose={closeProductAreaDialog}
          onSubmit={() => productAreaDialogMode === "create" ? createProductAreaMutation.mutate() : updateProductAreaMutation.mutate()}
        />
      )}

      {capabilityDialogMode !== "closed" && (
        <CapabilityFormModal
          mode={capabilityDialogMode}
          selectedProductArea={selectedProductArea}
          selectedCapability={selectedCapability}
          form={capabilityForm}
          draft={capabilityDraft}
          setForm={setCapabilityForm}
          setDraft={setCapabilityDraft}
          createKindGroups={selectedCapabilityAllowedKindGroups}
          editKindGroups={editableCapabilityNodeKindGroups}
          formError={formError}
          activeProductAreaId={activeProductAreaId}
          isCreatePending={createCapabilityMutation.isPending}
          isUpdatePending={updateCapabilityMutation.isPending}
          onClose={closeCapabilityDialog}
          onSubmit={() => capabilityDialogMode === "create" ? createCapabilityMutation.mutate() : updateCapabilityMutation.mutate()}
        />
      )}

    </div>
  );
}
