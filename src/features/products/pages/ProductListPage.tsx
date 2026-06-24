import React, { useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useLocation, useNavigate } from "react-router-dom";
import { useWorkspaceStore } from "../../../state/workspaceStore";
import { useUIStore } from "../../../state/uiStore";
import { ProductManagementConsole } from "../components/ProductManagementConsole";
import { ProductManagementModalStack } from "../components/ProductManagementModalStack";
import { ProductPageTabs } from "../components/ProductPageTabs";
import { ProductWorkspacePanel } from "../components/ProductWorkspacePanel";
import { useProductCatalogControls } from "../hooks/useProductCatalogControls";
import { useProductHierarchySelectionState } from "../hooks/useProductHierarchySelectionState";
import { useProductHierarchyMutations } from "../hooks/useProductHierarchyMutations";
import { useProductManagementSelection } from "../hooks/useProductManagementSelection";
import { useProductManagementWorkItemMutations } from "../hooks/useProductManagementWorkItemMutations";
import { useProductPageActions } from "../hooks/useProductPageActions";
import { useProductPageSync } from "../hooks/useProductPageSync";
import { useProductPageViewModel } from "../hooks/useProductPageViewModel";
import { styles } from "../lib/productListPageStyles";
import {
  HIDE_EXAMPLE_PRODUCTS_KEY,
  emptyProductDependencyDraft,
  emptyProductForm,
  emptyWorkItemDraft,
  type CapabilityFormState,
  type ProductAreaFormState,
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
} from "../lib/productRefreshScopes";
import type { Product, WorkItem } from "../../../lib/types";





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
  const [productAreaForm, setProductAreaForm] = useState<ProductAreaFormState>({ name: "", description: "", purpose: "", nodeKind: "product_area" });
  const [productAreaDraft, setProductAreaDraft] = useState<ProductAreaFormState>({ name: "", description: "", purpose: "", nodeKind: "product_area" });
  const [capabilityForm, setCapabilityForm] = useState<CapabilityFormState>({ name: "", description: "", acceptanceCriteria: "", technicalNotes: "", nodeKind: "capability" });
  const [capabilityDraft, setCapabilityDraft] = useState<CapabilityFormState>({ name: "", description: "", acceptanceCriteria: "", technicalNotes: "", nodeKind: "capability" });
  const [productManagementTab, setProductManagementTab] = useState<ProductManagementTab>("areas");
  const [formError, setFormError] = useState<string | null>(null);
  const [productPageTab, setProductPageTab] = useState<ProductPageTab>(() => isProductDetailRoute ? "design" : "list");
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

  const {
    productSearch,
    setProductSearch,
    productStatusFilter,
    setProductStatusFilter,
    productSourceFilter,
    setProductSourceFilter,
    productTagFilter,
    setProductTagFilter,
    productSort,
    setProductSort,
    showDefaultProductsInTable,
    setShowDefaultProductsInTable,
    showCustomProductsInTable,
    setShowCustomProductsInTable,
    catalogFilterMsg,
    catalogFilterError,
    statusProductId,
    setStatusProductId,
    statusDepth,
    setStatusDepth,
    statusGroupBy,
    setStatusGroupBy,
    updateDefaultProductVisibility,
  } = useProductCatalogControls({
    queryClient,
  });

  const {
    products,
    isLoading,
    productDependencies,
    selectedProductId,
    selectedProduct,
    tree,
    productTreeById,
    productTreeSummaryById,
    scopeSummaryIndex,
    allProductTags,
    includeDefaultProductsInCatalog,
    productTableRows,
    statusSummary,
    statusRows,
  } = useProductPageViewModel({
    activeProductId,
    productPageTab,
    productSearch,
    productStatusFilter,
    productSourceFilter,
    productTagFilter,
    productSort,
    showDefaultProductsInTable,
    showCustomProductsInTable,
    statusProductId,
    statusDepth,
    statusGroupBy,
  });

  const {
    productAreaOrderIds,
    capabilityOrderMap,
    selectedProductArea,
    selectedCapability,
    selectedCapabilityParentKind,
  } = useProductHierarchySelectionState({
    tree,
    activeProductAreaId,
    activeCapabilityId,
  });

  const {
    createProductMutation,
    updateProductMutation,
    createProductDependencyMutation,
    createProductAreaMutation,
    updateProductAreaMutation,
    createCapabilityMutation,
    updateCapabilityMutation,
    archiveMutation,
    resetProductPlanMutation,
    deleteHierarchyMutation,
    reorderProductAreasMutation,
    reorderCapabilitiesMutation,
    invalidateTasks,
  } = useProductHierarchyMutations({
    queryClient,
    selectedProductId,
    activeProductAreaId,
    activeCapabilityId,
    selectedProductArea,
    selectedCapability,
    productForm,
    setProductForm,
    productDraft,
    productAreaForm,
    setProductAreaForm,
    productAreaDraft,
    capabilityForm,
    setCapabilityForm,
    capabilityDraft,
    dependencyDraft,
    setDependencyDraft,
    statusProductId,
    setStatusProductId,
    closeProductDialog,
    closeProductAreaDialog,
    closeCapabilityDialog,
    setProductWorkspaceTab,
    setActiveProduct,
    setActiveProductArea,
    setActiveCapability,
    setDeleteProductCandidate,
    setDeleteConfirmName,
    setDeleteConfirmArchive,
    setResetPlanCandidate,
    setResetPlanConfirmName,
    setResetPlanConfirmTree,
    setResetPlanDeleteDelivery,
    setDeleteHierarchyCandidate,
    setDeleteHierarchyConfirmName,
    setDeleteHierarchyConfirmChecked,
    setFormError,
  });

  const {
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
  } = useProductManagementSelection({
    tree,
    productTreeById,
    productDependencies,
    products,
    selectedProductId,
    selectedProductArea,
    selectedCapability,
    selectedCapabilityParentKind,
    dependencyDependsOnProductId: dependencyDraft.dependsOnProductId,
    productAreaOrderIds,
    capabilityOrderMap,
    activeProductAreaId,
    activeCapabilityId,
    activeWorkItemId,
    selectedManagementStoryId,
    managementStoryPageIndex,
    productPageTab,
    productManagementTab,
  });
  const {
    createManagementStoryMutation,
    updateManagementStoryMutation,
    createManagementTaskMutation,
    updateManagementTaskMutation,
    deleteManagementWorkItemMutation,
  } = useProductManagementWorkItemMutations({
    selectedProductId,
    selectedManagementFeatureNode,
    selectedManagementStory,
    setSelectedManagementStoryId,
    setActiveWorkItem,
    storyDraft,
    setStoryDraft,
    taskDraft,
    setTaskDraft,
    editingStory,
    setEditingStory,
    editingTask,
    setEditingTask,
    setStoryDialogMode,
    setTaskDialogMode,
    setDeleteWorkItemCandidate,
    setDeleteWorkItemConfirmName,
    setDeleteWorkItemConfirmChecked,
    setFormError,
    invalidateTasks,
  });
  useProductPageSync({
    isLoading,
    activeProductId,
    selectedProductId,
    products,
    setActiveProduct,
    statusProductId,
    setStatusProductId,
    setActiveWorkItem,
    activeProductAreaId,
    activeCapabilityId,
    selectedProduct,
    setProductForm,
    setProductDraft,
    productDialogMode,
    selectedProductArea,
    setProductAreaForm,
    setProductAreaDraft,
    productAreaDialogMode,
    selectedCapability,
    setCapabilityForm,
    setCapabilityDraft,
    capabilityDialogMode,
    selectedManagementFeatureNode,
    productManagementTab,
    setSelectedManagementStoryId,
    setManagementStoryPageIndex,
    setFormError,
  });
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

  const {
    selectProductArea,
    selectCapabilityForManagement,
    openCreateCapabilityForArea,
    openCreateFeatureForCapability,
    openEditProductArea,
    openEditCapabilityNode,
    requestDeleteHierarchyNode,
    openFeatureInBuilder,
    openStoryInBuilder,
    renderCopyableEntityId,
    editProductFromList,
    openProductDesign,
    openProductOverview,
    openProductStatus,
    openProductDependencies,
    requestArchiveProduct,
    requestResetProductPlan,
    openCreateStoryDialog,
    openEditStoryDialog,
    openCreateTaskDialog,
    openEditTaskDialog,
    requestDeleteWorkItem,
  } = useProductPageActions({
    navigate,
    copiedEntityId,
    setCopiedEntityId,
    setActiveProduct,
    setActiveHierarchyNode,
    setActiveWorkItem,
    setActiveView,
    setStatusProductId,
    setProductPageTab,
    setProductDraft,
    openProductDialog,
    openProductAreaDialog,
    openCapabilityDialog,
    setProductAreaDraft,
    setCapabilityForm,
    setCapabilityDraft,
    setDeleteProductCandidate,
    setDeleteConfirmName,
    setDeleteConfirmArchive,
    setResetPlanCandidate,
    setResetPlanConfirmName,
    setResetPlanConfirmTree,
    setResetPlanDeleteDelivery,
    setDeleteHierarchyCandidate,
    setDeleteHierarchyConfirmName,
    setDeleteHierarchyConfirmChecked,
    setSelectedManagementStoryId,
    setStoryDialogMode,
    setTaskDialogMode,
    setEditingStory,
    setEditingTask,
    setStoryDraft,
    setTaskDraft,
    setDeleteWorkItemCandidate,
    setDeleteWorkItemConfirmName,
    setDeleteWorkItemConfirmChecked,
    setFormError,
  });

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
