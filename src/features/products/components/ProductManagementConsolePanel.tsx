import type React from "react";

import type {
  CapabilityTree,
  HierarchyTreeNode,
  Product,
  ProductAreaTree,
  ProductTree,
  WorkItem,
} from "../../../lib/types";
import type { ProductManagementTab } from "../lib/productRefreshScopes";
import type { WorkItemScopeSummaryIndex } from "../lib/productStatusSummary";
import { ProductManagementConsole } from "./ProductManagementConsole";
import type { ProductManagementFeatureEntry } from "./ProductManagementWorkItemFeatureSelector";

type DeleteHierarchyCandidate = {
  kind: "product_area" | "capability" | "feature";
  id: string;
  name: string;
};

type ProductManagementConsolePanelProps = {
  selectedProduct: Product | null;
  productManagementTab: ProductManagementTab;
  onProductManagementTabChange: (tab: ProductManagementTab) => void;
  refreshLabel: string;
  onRefresh: () => Promise<void>;
  selectedProductId: string | null;
  renderCopyableEntityId: (label: string, id: string) => React.ReactNode;
  productAreaProductAreas: ProductAreaTree[];
  onResetProductPlan: (product: Product) => void;
  onCreateProductArea: () => void;
  onSelectProductArea: (productAreaTree: ProductAreaTree) => void;
  onEditProductArea: (productAreaTree: ProductAreaTree) => void;
  onDeleteHierarchyNode: (candidate: DeleteHierarchyCandidate) => void;
  selectedProductAreaTree: ProductAreaTree | null;
  managementCapabilities: CapabilityTree[];
  productTree: ProductTree | null | undefined;
  scopeSummaryIndex: WorkItemScopeSummaryIndex;
  onCreateCapability: (productAreaTree: ProductAreaTree) => void;
  onSelectCapabilityForManagement: (capabilityTree: CapabilityTree) => void;
  onEditCapabilityNode: (capabilityTree: CapabilityTree) => void;
  selectedManagementCapabilityTree: CapabilityTree | null;
  managementFeatures: CapabilityTree[];
  onCreateFeature: (capabilityTree: CapabilityTree) => void;
  allManagementFeatures: ProductManagementFeatureEntry[];
  selectedManagementFeature: ProductManagementFeatureEntry | null;
  setSelectedManagementStoryId: (storyId: string | null) => void;
  featureStories: WorkItem[];
  selectedManagementStory: WorkItem | null;
  selectedManagementFeatureNode: HierarchyTreeNode | null;
  managementStoryPageIndex: number;
  hasNextStoryPage: boolean;
  setManagementStoryPageIndex: React.Dispatch<React.SetStateAction<number>>;
  onCreateStory: () => void;
  onOpenFeatureInBuilder: (featureNode: HierarchyTreeNode | null) => void;
  setActiveWorkItem: (workItemId: string | null) => void;
  onOpenStoryInBuilder: (story: WorkItem) => void;
  onEditStory: (story: WorkItem) => void;
  onCreateTask: () => void;
  onEditTask: (task: WorkItem) => void;
  onRequestDeleteWorkItem: (workItem: WorkItem, kind: "story" | "task") => void;
  selectedManagementTasks: WorkItem[];
};

export function ProductManagementConsolePanel({
  selectedProduct,
  productManagementTab,
  onProductManagementTabChange,
  refreshLabel,
  onRefresh,
  selectedProductId,
  renderCopyableEntityId,
  productAreaProductAreas,
  onResetProductPlan,
  onCreateProductArea,
  onSelectProductArea,
  onEditProductArea,
  onDeleteHierarchyNode,
  selectedProductAreaTree,
  managementCapabilities,
  productTree,
  scopeSummaryIndex,
  onCreateCapability,
  onSelectCapabilityForManagement,
  onEditCapabilityNode,
  selectedManagementCapabilityTree,
  managementFeatures,
  onCreateFeature,
  allManagementFeatures,
  selectedManagementFeature,
  setSelectedManagementStoryId,
  featureStories,
  selectedManagementStory,
  selectedManagementFeatureNode,
  managementStoryPageIndex,
  hasNextStoryPage,
  setManagementStoryPageIndex,
  onCreateStory,
  onOpenFeatureInBuilder,
  setActiveWorkItem,
  onOpenStoryInBuilder,
  onEditStory,
  onCreateTask,
  onEditTask,
  onRequestDeleteWorkItem,
  selectedManagementTasks,
}: ProductManagementConsolePanelProps) {
  return (
    <ProductManagementConsole
      selectedProduct={selectedProduct}
      activeTab={productManagementTab}
      onTabChange={onProductManagementTabChange}
      refreshLabel={refreshLabel}
      onRefresh={onRefresh}
      refreshDisabled={!selectedProductId}
      renderCopyableEntityId={renderCopyableEntityId}
      productAreas={productAreaProductAreas}
      onResetProductPlan={onResetProductPlan}
      onCreateProductArea={onCreateProductArea}
      onSelectProductArea={onSelectProductArea}
      onEditProductArea={onEditProductArea}
      onDeleteHierarchyNode={onDeleteHierarchyNode}
      selectedProductAreaTree={selectedProductAreaTree}
      capabilities={managementCapabilities}
      productTree={productTree}
      selectedProductId={selectedProductId}
      scopeSummaryIndex={scopeSummaryIndex}
      onCreateCapability={onCreateCapability}
      onSelectCapability={onSelectCapabilityForManagement}
      onEditCapability={onEditCapabilityNode}
      selectedCapabilityTree={selectedManagementCapabilityTree}
      features={managementFeatures}
      onCreateFeature={onCreateFeature}
      allFeatures={allManagementFeatures}
      selectedFeature={selectedManagementFeature}
      onSelectFeature={(entry) => {
        onSelectCapabilityForManagement(entry.capabilityTree);
        setSelectedManagementStoryId(null);
      }}
      stories={featureStories}
      selectedStory={selectedManagementStory}
      canCreateStory={!!selectedManagementFeatureNode}
      storyPageIndex={managementStoryPageIndex}
      hasNextStoryPage={hasNextStoryPage}
      onPreviousStoryPage={() => setManagementStoryPageIndex((current) => Math.max(0, current - 1))}
      onNextStoryPage={() => setManagementStoryPageIndex((current) => current + 1)}
      onCreateStory={onCreateStory}
      onOpenBuilder={() => onOpenFeatureInBuilder(selectedManagementFeatureNode)}
      onSelectStory={(story) => {
        setSelectedManagementStoryId(story.id);
        setActiveWorkItem(story.id);
      }}
      onEditStory={onEditStory}
      onDeleteStory={(story) => onRequestDeleteWorkItem(story, "story")}
      tasks={selectedManagementTasks}
      onOpenStory={onOpenStoryInBuilder}
      onCreateTask={onCreateTask}
      onEditTask={onEditTask}
      onDeleteTask={(task) => onRequestDeleteWorkItem(task, "task")}
    />
  );
}
