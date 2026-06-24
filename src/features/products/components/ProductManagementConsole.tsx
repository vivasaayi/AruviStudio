import type React from "react";

import type { CapabilityTree, Product, ProductAreaTree, ProductTree, WorkItem } from "../../../lib/types";
import type { ProductManagementTab } from "../lib/productRefreshScopes";
import type { WorkItemScopeSummaryIndex } from "../lib/productStatusSummary";
import { styles } from "../lib/productListPageStyles";
import { ProductManagementAreasTab } from "./ProductManagementAreasTab";
import { ProductManagementCapabilitiesTab } from "./ProductManagementCapabilitiesTab";
import { ProductManagementFeaturesTab } from "./ProductManagementFeaturesTab";
import { ProductManagementHeader } from "./ProductManagementHeader";
import { ProductManagementStoriesPane } from "./ProductManagementStoriesPane";
import { ProductManagementStoryDetailPane } from "./ProductManagementStoryDetailPane";
import {
  ProductManagementWorkItemFeatureSelector,
  type ProductManagementFeatureEntry,
} from "./ProductManagementWorkItemFeatureSelector";

type DeleteHierarchyCandidate = {
  kind: "product_area" | "capability" | "feature";
  id: string;
  name: string;
};

type ProductManagementConsoleProps = {
  selectedProduct: Product | null;
  activeTab: ProductManagementTab;
  onTabChange: (tab: ProductManagementTab) => void;
  refreshLabel: string;
  onRefresh: () => Promise<void>;
  refreshDisabled: boolean;
  renderCopyableEntityId: (label: string, id: string) => React.ReactNode;
  productAreas: ProductAreaTree[];
  onResetProductPlan: (product: Product) => void;
  onCreateProductArea: () => void;
  onSelectProductArea: (productAreaTree: ProductAreaTree) => void;
  onEditProductArea: (productAreaTree: ProductAreaTree) => void;
  onDeleteHierarchyNode: (candidate: DeleteHierarchyCandidate) => void;
  selectedProductAreaTree: ProductAreaTree | null;
  capabilities: CapabilityTree[];
  productTree: ProductTree | null | undefined;
  selectedProductId: string | null;
  scopeSummaryIndex: WorkItemScopeSummaryIndex;
  onCreateCapability: (productAreaTree: ProductAreaTree) => void;
  onSelectCapability: (capabilityTree: CapabilityTree) => void;
  onEditCapability: (capabilityTree: CapabilityTree) => void;
  selectedCapabilityTree: CapabilityTree | null;
  features: CapabilityTree[];
  onCreateFeature: (capabilityTree: CapabilityTree) => void;
  allFeatures: ProductManagementFeatureEntry[];
  selectedFeature: ProductManagementFeatureEntry | null;
  onSelectFeature: (entry: ProductManagementFeatureEntry) => void;
  stories: WorkItem[];
  selectedStory: WorkItem | null;
  canCreateStory: boolean;
  storyPageIndex: number;
  hasNextStoryPage: boolean;
  onPreviousStoryPage: () => void;
  onNextStoryPage: () => void;
  onCreateStory: () => void;
  onOpenBuilder: () => void;
  onSelectStory: (story: WorkItem) => void;
  onEditStory: (story: WorkItem) => void;
  onDeleteStory: (story: WorkItem) => void;
  tasks: WorkItem[];
  onOpenStory: (story: WorkItem) => void;
  onCreateTask: () => void;
  onEditTask: (task: WorkItem) => void;
  onDeleteTask: (task: WorkItem) => void;
};

export function ProductManagementConsole({
  selectedProduct,
  activeTab,
  onTabChange,
  refreshLabel,
  onRefresh,
  refreshDisabled,
  renderCopyableEntityId,
  productAreas,
  onResetProductPlan,
  onCreateProductArea,
  onSelectProductArea,
  onEditProductArea,
  onDeleteHierarchyNode,
  selectedProductAreaTree,
  capabilities,
  productTree,
  selectedProductId,
  scopeSummaryIndex,
  onCreateCapability,
  onSelectCapability,
  onEditCapability,
  selectedCapabilityTree,
  features,
  onCreateFeature,
  allFeatures,
  selectedFeature,
  onSelectFeature,
  stories,
  selectedStory,
  canCreateStory,
  storyPageIndex,
  hasNextStoryPage,
  onPreviousStoryPage,
  onNextStoryPage,
  onCreateStory,
  onOpenBuilder,
  onSelectStory,
  onEditStory,
  onDeleteStory,
  tasks,
  onOpenStory,
  onCreateTask,
  onEditTask,
  onDeleteTask,
}: ProductManagementConsoleProps) {
  return (
    <div>
      <ProductManagementHeader
        selectedProduct={selectedProduct}
        activeTab={activeTab}
        onTabChange={onTabChange}
        refreshLabel={refreshLabel}
        onRefresh={onRefresh}
        refreshDisabled={refreshDisabled}
        renderCopyableEntityId={renderCopyableEntityId}
        styles={styles}
      />

      {activeTab === "areas" && (
        <ProductManagementAreasTab
          selectedProduct={selectedProduct}
          productAreas={productAreas}
          onResetProductPlan={onResetProductPlan}
          onCreateProductArea={onCreateProductArea}
          onOpenProductArea={(productAreaTree) => {
            onSelectProductArea(productAreaTree);
            onTabChange("capabilities");
          }}
          onEditProductArea={onEditProductArea}
          onDeleteProductArea={(productAreaTree) => onDeleteHierarchyNode({
            kind: "product_area",
            id: productAreaTree.product_area.id,
            name: productAreaTree.product_area.name,
          })}
          renderCopyableEntityId={renderCopyableEntityId}
          styles={styles}
        />
      )}

      {activeTab === "capabilities" && (
        <ProductManagementCapabilitiesTab
          productAreas={productAreas}
          selectedProductAreaTree={selectedProductAreaTree}
          capabilities={capabilities}
          productTree={productTree}
          selectedProductId={selectedProductId}
          scopeSummaryIndex={scopeSummaryIndex}
          onSelectProductArea={onSelectProductArea}
          onCreateCapability={onCreateCapability}
          onOpenCapability={(capabilityTree) => {
            onSelectCapability(capabilityTree);
            onTabChange("features");
          }}
          onEditCapability={onEditCapability}
          onDeleteCapability={(capabilityTree) => onDeleteHierarchyNode({
            kind: "capability",
            id: capabilityTree.capability.id,
            name: capabilityTree.capability.name,
          })}
          renderCopyableEntityId={renderCopyableEntityId}
          styles={styles}
        />
      )}

      {activeTab === "features" && (
        <ProductManagementFeaturesTab
          capabilities={capabilities}
          selectedCapabilityTree={selectedCapabilityTree}
          features={features}
          productTree={productTree}
          selectedProductId={selectedProductId}
          scopeSummaryIndex={scopeSummaryIndex}
          onSelectCapability={onSelectCapability}
          onCreateFeature={onCreateFeature}
          onOpenFeatureStories={(featureTree) => {
            onSelectCapability(featureTree);
            onTabChange("work_items");
          }}
          onEditFeature={onEditCapability}
          onDeleteFeature={(featureTree) => onDeleteHierarchyNode({
            kind: "feature",
            id: featureTree.capability.id,
            name: featureTree.capability.name,
          })}
          renderCopyableEntityId={renderCopyableEntityId}
          styles={styles}
        />
      )}

      {activeTab === "work_items" && (
        <div style={styles.managementThreePane}>
          <ProductManagementWorkItemFeatureSelector
            features={allFeatures}
            selectedFeature={selectedFeature}
            onSelectFeature={onSelectFeature}
            renderCopyableEntityId={renderCopyableEntityId}
            styles={styles}
          />
          <ProductManagementStoriesPane
            stories={stories}
            selectedStory={selectedStory}
            canCreateStory={canCreateStory}
            storyPageIndex={storyPageIndex}
            hasNextStoryPage={hasNextStoryPage}
            onPreviousStoryPage={onPreviousStoryPage}
            onNextStoryPage={onNextStoryPage}
            onCreateStory={onCreateStory}
            onOpenBuilder={onOpenBuilder}
            onSelectStory={onSelectStory}
            onEditStory={onEditStory}
            onDeleteStory={onDeleteStory}
            styles={styles}
          />
          <ProductManagementStoryDetailPane
            selectedStory={selectedStory}
            tasks={tasks}
            onEditStory={onEditStory}
            onOpenStory={onOpenStory}
            onCreateTask={onCreateTask}
            onEditTask={onEditTask}
            onDeleteTask={onDeleteTask}
            styles={styles}
          />
        </div>
      )}
    </div>
  );
}
