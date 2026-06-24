import type { ProductListPageController } from "../hooks/useProductListPageController";
import { ProductManagementConsolePanel } from "./ProductManagementConsolePanel";
import { ProductManagementModalStack } from "./ProductManagementModalStack";
import { ProductPageTabs } from "./ProductPageTabs";
import { ProductWorkspacePanel } from "./ProductWorkspacePanel";
import { styles } from "../lib/productListPageStyles";

type ProductListPageBodyProps = {
  controller: ProductListPageController;
};

export function ProductListPageBody({ controller }: ProductListPageBodyProps) {
  const {
    activeProductAreaId,
    allManagementFeatures,
    allProductTags,
    archiveMutation,
    capabilityDialogMode,
    capabilityDraft,
    capabilityForm,
    capabilityLabelById,
    catalogFilterError,
    catalogFilterMsg,
    closeCapabilityDialog,
    closeProductAreaDialog,
    closeProductDialog,
    createCapabilityMutation,
    createManagementStoryMutation,
    createManagementTaskMutation,
    createProductAreaMutation,
    createProductDependencyMutation,
    createProductMutation,
    deleteConfirmArchive,
    deleteConfirmName,
    deleteConfirmationReady,
    deleteHierarchyCandidate,
    deleteHierarchyConfirmChecked,
    deleteHierarchyConfirmName,
    deleteHierarchyMutation,
    deleteHierarchyReady,
    deleteManagementWorkItemReady,
    deleteManagementWorkItemMutation,
    deleteProductCandidate,
    deleteWorkItemCandidate,
    deleteWorkItemConfirmChecked,
    deleteWorkItemConfirmName,
    dependencyDraft,
    dependencyTargetCapabilityOptions,
    editProductFromList,
    editableCapabilityNodeKindGroups,
    featureStories,
    formError,
    includeDefaultProductsInCatalog,
    isLoading,
    managementCapabilities,
    managementFeatureWorkItemPage,
    managementFeatures,
    managementStoryPageIndex,
    openCapabilityDialog,
    openCreateCapabilityForArea,
    openCreateFeatureForCapability,
    openCreateStoryDialog,
    openCreateTaskDialog,
    openEditCapabilityNode,
    openEditProductArea,
    openEditStoryDialog,
    openEditTaskDialog,
    openFeatureInBuilder,
    openProductDependencies,
    openProductDesign,
    openProductDialog,
    openProductAreaDialog,
    openProductOverview,
    openProductStatus,
    openStoryInBuilder,
    productAreaDialogMode,
    productAreaDraft,
    productAreaForm,
    productAreaProductAreas,
    productDialogMode,
    productDraft,
    productForm,
    productManagementRefreshLabel,
    productManagementTab,
    productNameById,
    productPageTab,
    productSearch,
    productSort,
    productSourceFilter,
    productStatusFilter,
    productTagFilter,
    productTableRows,
    products,
    renderCopyableEntityId,
    requestArchiveProduct,
    requestDeleteHierarchyNode,
    requestDeleteWorkItem,
    requestResetProductPlan,
    resetPlanCandidate,
    resetPlanConfirmName,
    resetPlanConfirmTree,
    resetPlanDeleteDelivery,
    resetPlanReady,
    resetProductPlanMutation,
    scopeSummaryIndex,
    selectCapabilityForManagement,
    selectProductArea,
    selectedCapability,
    selectedCapabilityAllowedKindGroups,
    selectedCapabilityOptions,
    selectedManagementCapabilityTree,
    selectedManagementFeature,
    selectedManagementFeatureNode,
    selectedManagementStory,
    selectedManagementTasks,
    selectedProduct,
    selectedProductArea,
    selectedProductAreaTree,
    selectedProductDependencies,
    selectedProductId,
    setActiveHierarchyNode,
    setActiveProduct,
    setActiveWorkItem,
    setCapabilityDraft,
    setCapabilityForm,
    setDeleteConfirmArchive,
    setDeleteConfirmName,
    setDeleteHierarchyCandidate,
    setDeleteHierarchyConfirmChecked,
    setDeleteHierarchyConfirmName,
    setDeleteProductCandidate,
    setDeleteWorkItemCandidate,
    setDeleteWorkItemConfirmChecked,
    setDeleteWorkItemConfirmName,
    setDependencyDraft,
    setManagementStoryPageIndex,
    setProductAreaDraft,
    setProductAreaForm,
    setProductDraft,
    setProductForm,
    setProductManagementTab,
    setProductPageTab,
    setProductSearch,
    setProductSort,
    setProductSourceFilter,
    setProductTagFilter,
    setProductStatusFilter,
    setResetPlanCandidate,
    setResetPlanConfirmName,
    setResetPlanConfirmTree,
    setResetPlanDeleteDelivery,
    setSelectedManagementStoryId,
    setShowCustomProductsInTable,
    setShowDefaultProductsInTable,
    setStatusDepth,
    setStatusGroupBy,
    setStatusProductId,
    setStoryDialogMode,
    setStoryDraft,
    setTaskDialogMode,
    setTaskDraft,
    showCustomProductsInTable,
    showDefaultProductsInTable,
    statusDepth,
    statusGroupBy,
    statusProductId,
    statusRows,
    statusSummary,
    storyDialogMode,
    storyDraft,
    taskDialogMode,
    taskDraft,
    tree,
    updateCapabilityMutation,
    updateDefaultProductVisibility,
    updateManagementStoryMutation,
    updateManagementTaskMutation,
    updateProductAreaMutation,
    updateProductMutation,
    activeProductPageRefreshLabel,
    activeProductPageRefreshDisabled,
    refreshActiveProductPageTab,
    refreshProductManagementTabQueries,
  } = controller;

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
        productManagementConsole={(
          <ProductManagementConsolePanel
            selectedProduct={selectedProduct}
            productManagementTab={productManagementTab}
            onProductManagementTabChange={setProductManagementTab}
            refreshLabel={productManagementRefreshLabel}
            onRefresh={refreshProductManagementTabQueries}
            selectedProductId={selectedProductId}
            renderCopyableEntityId={renderCopyableEntityId}
            productAreaProductAreas={productAreaProductAreas}
            onResetProductPlan={requestResetProductPlan}
            onCreateProductArea={() => openProductAreaDialog("create")}
            onSelectProductArea={selectProductArea}
            onEditProductArea={openEditProductArea}
            onDeleteHierarchyNode={requestDeleteHierarchyNode}
            selectedProductAreaTree={selectedProductAreaTree}
            managementCapabilities={managementCapabilities}
            productTree={tree}
            scopeSummaryIndex={scopeSummaryIndex}
            onCreateCapability={openCreateCapabilityForArea}
            onSelectCapabilityForManagement={selectCapabilityForManagement}
            onEditCapabilityNode={openEditCapabilityNode}
            selectedManagementCapabilityTree={selectedManagementCapabilityTree}
            managementFeatures={managementFeatures}
            onCreateFeature={openCreateFeatureForCapability}
            allManagementFeatures={allManagementFeatures}
            selectedManagementFeature={selectedManagementFeature}
            setSelectedManagementStoryId={setSelectedManagementStoryId}
            featureStories={featureStories}
            selectedManagementStory={selectedManagementStory}
            selectedManagementFeatureNode={selectedManagementFeatureNode}
            managementStoryPageIndex={managementStoryPageIndex}
            hasNextStoryPage={managementFeatureWorkItemPage?.has_more ?? false}
            setManagementStoryPageIndex={setManagementStoryPageIndex}
            onCreateStory={openCreateStoryDialog}
            onOpenFeatureInBuilder={openFeatureInBuilder}
            setActiveWorkItem={setActiveWorkItem}
            onOpenStoryInBuilder={openStoryInBuilder}
            onEditStory={openEditStoryDialog}
            onCreateTask={openCreateTaskDialog}
            onEditTask={openEditTaskDialog}
            onRequestDeleteWorkItem={requestDeleteWorkItem}
            selectedManagementTasks={selectedManagementTasks}
          />
        )}
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
