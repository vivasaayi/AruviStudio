import React, { useEffect, useMemo, useState } from "react";
import { useMutation, useQueries, useQuery, useQueryClient } from "@tanstack/react-query";
import { useLocation, useNavigate } from "react-router-dom";
import {
  archiveProduct,
  createCapability,
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
  listProducts,
  listWorkItemsPage,
  reorderCapabilities,
  reorderProductAreas,
  resetProductPlan,
  setSetting,
  summarizeProductTree,
  summarizeWorkItemsByProduct,
  summarizeWorkItemsByScope,
  updateCapability,
  updateProductArea,
  updateProduct,
  updateWorkItem,
} from "../../../lib/tauri";
import { findHierarchyNode, flattenHierarchyNodes } from "../../../lib/hierarchyTree";
import {
  getAllowedChildNodeKinds,
  getDefaultChildNodeKind,
  groupHierarchyNodeKinds,
  getHierarchyChildLabel,
  orderHierarchyNodeKinds,
} from "../../../lib/hierarchyLabels";
import { useWorkspaceStore } from "../../../state/workspaceStore";
import { useUIStore } from "../../../state/uiStore";
import { CopyableEntityId } from "../components/CopyableEntityId";
import { ProductManagementConsole } from "../components/ProductManagementConsole";
import { ProductManagementModalStack } from "../components/ProductManagementModalStack";
import { ProductPageTabs } from "../components/ProductPageTabs";
import { ProductWorkspacePanel } from "../components/ProductWorkspacePanel";
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
import {
  buildProductCatalogRows,
  getProductCatalogTags,
  type ProductCatalogSort,
  type ProductCatalogSourceFilter,
  type ProductCatalogStatusFilter,
} from "../lib/productCatalogRows";
import {
  buildCapabilityLabelById,
  buildCapabilityOptionsFromNodes,
  buildDependencyTargetCapabilityOptions,
} from "../lib/productDependencyOptions";
import {
  buildProductStatusSummary,
  buildStatusRows,
  buildWorkItemScopeSummaryIndex,
} from "../lib/productStatusSummary";
import {
  findCapabilityTree,
  getCapabilityOrderKey,
  getOrderedCapabilityTrees,
  orderItemsByIds,
  seedCapabilityOrderMap,
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
import type { CapabilityTree, HierarchyNodeKind, HierarchyTreeNode, ProductAreaTree, Product, ProductTree, ProductTreeSummary, ProductWorkItemSummary, WorkItem, WorkItemScopeSummary } from "../../../lib/types";





export function ProductListPage() {
  const queryClient = useQueryClient();
  const navigate = useNavigate();
  const location = useLocation();
  const isProductDetailRoute = location.pathname.startsWith("/products/");
  const {
    activeProductId,
    activeProductAreaId,
    activeCapabilityId,
    activeWorkItemId,
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
    closeProductDialog,
    openProductDialog,
    closeProductAreaDialog,
    openProductAreaDialog,
    closeCapabilityDialog,
    openCapabilityDialog,
    setProductWorkspaceTab,
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
  const [copiedEntityId, setCopiedEntityId] = useState<string | null>(null);

  const { data: products, isLoading } = useQuery({ queryKey: ["products"], queryFn: listProducts });
  const { data: productDependencies = [] } = useQuery({
    queryKey: ["product-dependencies"],
    queryFn: listProductDependencies,
    enabled: productPageTab === "design" || productPageTab === "dependencies",
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

  useEffect(() => {
    if (!activeProductId && products?.[0]?.id) {
      setActiveProduct(products[0].id);
    }
  }, [activeProductId, products, setActiveProduct]);

  useEffect(() => {
    setActiveWorkItem(null);
    setFormError(null);
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

  const allTreeNodes = useMemo(() => (tree ? flattenHierarchyNodes(tree.roots) : []), [tree]);
  const selectedCapabilityOptions = useMemo(
    () => buildCapabilityOptionsFromNodes(allTreeNodes),
    [allTreeNodes],
  );
  const dependencyTargetCapabilityOptions = useMemo(
    () => buildDependencyTargetCapabilityOptions(productTreeById, dependencyDraft.dependsOnProductId),
    [dependencyDraft.dependsOnProductId, productTreeById],
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
    return (
      <CopyableEntityId
        label={label}
        id={id}
        isCopied={copiedEntityId === id}
        onCopy={(entityId) => void copyEntityId(entityId)}
      />
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

  const productManagementConsole = (
    <ProductManagementConsole
      selectedProduct={selectedProduct}
      activeTab={productManagementTab}
      onTabChange={setProductManagementTab}
      refreshLabel={productManagementRefreshLabel}
      onRefresh={refreshProductManagementTabQueries}
      refreshDisabled={!selectedProductId}
      renderCopyableEntityId={renderCopyableEntityId}
      productAreas={productAreaProductAreas}
      onResetProductPlan={requestResetProductPlan}
      onCreateProductArea={() => openProductAreaDialog("create")}
      onSelectProductArea={selectProductArea}
      onEditProductArea={openEditProductArea}
      onDeleteHierarchyNode={requestDeleteHierarchyNode}
      selectedProductAreaTree={selectedProductAreaTree}
      capabilities={managementCapabilities}
      productTree={tree}
      selectedProductId={selectedProductId}
      scopeSummaryIndex={scopeSummaryIndex}
      onCreateCapability={openCreateCapabilityForArea}
      onSelectCapability={selectCapabilityForManagement}
      onEditCapability={openEditCapabilityNode}
      selectedCapabilityTree={selectedManagementCapabilityTree}
      features={managementFeatures}
      onCreateFeature={openCreateFeatureForCapability}
      allFeatures={allManagementFeatures}
      selectedFeature={selectedManagementFeature}
      onSelectFeature={(entry) => {
        selectCapabilityForManagement(entry.capabilityTree);
        setSelectedManagementStoryId(null);
      }}
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
      tasks={selectedManagementTasks}
      onOpenStory={openStoryInBuilder}
      onCreateTask={openCreateTaskDialog}
      onEditTask={openEditTaskDialog}
      onDeleteTask={(task) => requestDeleteWorkItem(task, "task")}
    />
  );

  return (
    <div style={styles.page}>
      <div style={styles.header}>
        <div style={styles.titleBlock}>
          <h1 style={styles.title}>Products</h1>
          <div style={styles.subtitle}>Manage products, shape product areas, capabilities, and features, then track delivery stories and tasks separately.</div>
        </div>
      </div>

      <ProductPageTabs
        productPageTab={productPageTab}
        selectedProductId={selectedProductId}
        selectedProduct={selectedProduct}
        products={products ?? []}
        refreshLabel={activeProductPageRefreshLabel}
        isRefreshDisabled={activeProductPageRefreshDisabled}
        onProductPageTabChange={setProductPageTab}
        onSelectedProductChange={(nextProductId) => {
          setActiveProduct(nextProductId);
          if (nextProductId && (productPageTab === "list" || productPageTab === "status")) {
            setProductPageTab("overview");
          }
        }}
        onRefresh={refreshActiveProductPageTab}
      />

      <ProductWorkspacePanel
        productPageTab={productPageTab}
        selectedProduct={selectedProduct}
        products={products ?? []}
        isLoading={isLoading}
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
        statusProductId={statusProductId}
        statusDepth={statusDepth}
        statusGroupBy={statusGroupBy}
        statusSummary={statusSummary}
        statusRows={statusRows}
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
        productManagementConsole={productManagementConsole}
      />

      <ProductManagementModalStack
        productDialogMode={productDialogMode}
        productForm={productForm}
        productDraft={productDraft}
        setProductForm={setProductForm}
        setProductDraft={setProductDraft}
        isCreateProductPending={createProductMutation.isPending}
        isUpdateProductPending={updateProductMutation.isPending}
        onCloseProductDialog={closeProductDialog}
        onSubmitProduct={() => (productDialogMode === "create" ? createProductMutation.mutate() : updateProductMutation.mutate())}
        deleteProductCandidate={deleteProductCandidate}
        deleteConfirmName={deleteConfirmName}
        deleteConfirmArchive={deleteConfirmArchive}
        deleteProductReady={deleteConfirmationReady}
        isArchiveProductPending={archiveMutation.isPending}
        onCloseDeleteProduct={() => setDeleteProductCandidate(null)}
        onDeleteConfirmNameChange={setDeleteConfirmName}
        onDeleteConfirmArchiveChange={setDeleteConfirmArchive}
        onArchiveProduct={(productId) => archiveMutation.mutate(productId)}
        resetPlanCandidate={resetPlanCandidate}
        resetPlanConfirmName={resetPlanConfirmName}
        resetPlanConfirmTree={resetPlanConfirmTree}
        resetPlanDeleteDelivery={resetPlanDeleteDelivery}
        resetPlanReady={resetPlanReady}
        isResetPlanPending={resetProductPlanMutation.isPending}
        onCloseResetPlan={() => setResetPlanCandidate(null)}
        onResetPlanConfirmNameChange={setResetPlanConfirmName}
        onResetPlanConfirmTreeChange={setResetPlanConfirmTree}
        onResetPlanDeleteDeliveryChange={setResetPlanDeleteDelivery}
        onResetPlan={(data) => resetProductPlanMutation.mutate(data)}
        deleteHierarchyCandidate={deleteHierarchyCandidate}
        deleteHierarchyConfirmName={deleteHierarchyConfirmName}
        deleteHierarchyConfirmChecked={deleteHierarchyConfirmChecked}
        deleteHierarchyReady={deleteHierarchyReady}
        isDeleteHierarchyPending={deleteHierarchyMutation.isPending}
        onCloseDeleteHierarchy={() => setDeleteHierarchyCandidate(null)}
        onDeleteHierarchyConfirmNameChange={setDeleteHierarchyConfirmName}
        onDeleteHierarchyConfirmCheckedChange={setDeleteHierarchyConfirmChecked}
        onDeleteHierarchy={(candidate) => deleteHierarchyMutation.mutate(candidate)}
        deleteWorkItemCandidate={deleteWorkItemCandidate}
        deleteWorkItemConfirmName={deleteWorkItemConfirmName}
        deleteWorkItemConfirmChecked={deleteWorkItemConfirmChecked}
        deleteWorkItemReady={deleteManagementWorkItemReady}
        isDeleteWorkItemPending={deleteManagementWorkItemMutation.isPending}
        onCloseDeleteWorkItem={() => setDeleteWorkItemCandidate(null)}
        onDeleteWorkItemConfirmNameChange={setDeleteWorkItemConfirmName}
        onDeleteWorkItemConfirmCheckedChange={setDeleteWorkItemConfirmChecked}
        onDeleteWorkItem={(candidate) => deleteManagementWorkItemMutation.mutate(candidate)}
        storyDialogMode={storyDialogMode}
        selectedFeatureTitle={selectedManagementFeature?.capabilityTree.capability.name ?? "No feature selected"}
        storyDraft={storyDraft}
        setStoryDraft={setStoryDraft}
        canSubmitStory={!!selectedManagementFeatureNode}
        isCreateStoryPending={createManagementStoryMutation.isPending}
        isUpdateStoryPending={updateManagementStoryMutation.isPending}
        onCloseStoryDialog={() => setStoryDialogMode("closed")}
        onSubmitStory={() => storyDialogMode === "edit" ? updateManagementStoryMutation.mutate() : createManagementStoryMutation.mutate()}
        taskDialogMode={taskDialogMode}
        selectedStoryTitle={selectedManagementStory?.title ?? "No story selected"}
        taskDraft={taskDraft}
        setTaskDraft={setTaskDraft}
        canSubmitTask={!!selectedManagementStory}
        isCreateTaskPending={createManagementTaskMutation.isPending}
        isUpdateTaskPending={updateManagementTaskMutation.isPending}
        onCloseTaskDialog={() => setTaskDialogMode("closed")}
        onSubmitTask={() => taskDialogMode === "edit" ? updateManagementTaskMutation.mutate() : createManagementTaskMutation.mutate()}
        productAreaDialogMode={productAreaDialogMode}
        selectedProductArea={selectedProductArea}
        productAreaForm={productAreaForm}
        productAreaDraft={productAreaDraft}
        setProductAreaForm={setProductAreaForm}
        setProductAreaDraft={setProductAreaDraft}
        selectedProductId={selectedProductId}
        isCreateProductAreaPending={createProductAreaMutation.isPending}
        isUpdateProductAreaPending={updateProductAreaMutation.isPending}
        onCloseProductAreaDialog={closeProductAreaDialog}
        onSubmitProductArea={() => productAreaDialogMode === "create" ? createProductAreaMutation.mutate() : updateProductAreaMutation.mutate()}
        capabilityDialogMode={capabilityDialogMode}
        selectedCapability={selectedCapability}
        capabilityForm={capabilityForm}
        capabilityDraft={capabilityDraft}
        setCapabilityForm={setCapabilityForm}
        setCapabilityDraft={setCapabilityDraft}
        createKindGroups={selectedCapabilityAllowedKindGroups}
        editKindGroups={editableCapabilityNodeKindGroups}
        activeProductAreaId={activeProductAreaId}
        isCreateCapabilityPending={createCapabilityMutation.isPending}
        isUpdateCapabilityPending={updateCapabilityMutation.isPending}
        onCloseCapabilityDialog={closeCapabilityDialog}
        onSubmitCapability={() => capabilityDialogMode === "create" ? createCapabilityMutation.mutate() : updateCapabilityMutation.mutate()}
        formError={formError}
      />

    </div>
  );
}
