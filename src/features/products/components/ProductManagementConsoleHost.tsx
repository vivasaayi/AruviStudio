import type { ProductListPageController } from "../hooks/useProductListPageController";
import { ProductManagementConsolePanel } from "./ProductManagementConsolePanel";

type ProductManagementConsoleHostProps = {
  controller: ProductListPageController;
};

export function ProductManagementConsoleHost({ controller }: ProductManagementConsoleHostProps) {
  const {
    allManagementFeatures,
    featureStories,
    managementCapabilities,
    managementFeatureWorkItemPage,
    managementFeatures,
    managementStoryPageIndex,
    openCreateCapabilityForArea,
    openCreateFeatureForCapability,
    openCreateStoryDialog,
    openCreateTaskDialog,
    openEditCapabilityNode,
    openEditProductArea,
    openEditStoryDialog,
    openEditTaskDialog,
    openFeatureInBuilder,
    openProductAreaDialog,
    openStoryInBuilder,
    productAreaProductAreas,
    productManagementRefreshLabel,
    productManagementTab,
    renderCopyableEntityId,
    requestDeleteHierarchyNode,
    requestDeleteWorkItem,
    requestResetProductPlan,
    scopeSummaryIndex,
    selectCapabilityForManagement,
    selectProductArea,
    selectedManagementCapabilityTree,
    selectedManagementFeature,
    selectedManagementFeatureNode,
    selectedManagementStory,
    selectedManagementTasks,
    selectedProduct,
    selectedProductAreaTree,
    selectedProductId,
    setActiveWorkItem,
    setManagementStoryPageIndex,
    setProductManagementTab,
    setSelectedManagementStoryId,
    tree,
    refreshProductManagementTabQueries,
  } = controller;

  return (
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
  );
}
