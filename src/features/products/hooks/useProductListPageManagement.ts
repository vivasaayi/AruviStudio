import { useProductHierarchyMutations } from "./useProductHierarchyMutations";
import { useProductListPageData } from "./useProductListPageData";
import { useProductListPageState } from "./useProductListPageState";
import { useProductManagementSelection } from "./useProductManagementSelection";
import { useProductManagementWorkItemMutations } from "./useProductManagementWorkItemMutations";
import { useProductPageRuntimeContext } from "./useProductPageRuntimeContext";

type ProductListPageManagementInput = {
  activeProductAreaId: string | null;
  activeCapabilityId: string | null;
  activeWorkItemId: string | null;
  data: ReturnType<typeof useProductListPageData>;
  state: ReturnType<typeof useProductListPageState>;
  setActiveWorkItem: ReturnType<typeof useProductPageRuntimeContext>["workspace"]["setActiveWorkItem"];
  invalidateTasks: ReturnType<typeof useProductHierarchyMutations>["invalidateTasks"];
};

export function useProductListPageManagement({
  activeProductAreaId,
  activeCapabilityId,
  activeWorkItemId,
  data,
  state,
  setActiveWorkItem,
  invalidateTasks,
}: ProductListPageManagementInput) {
  const {
    tree,
    productTreeById,
    productDependencies,
    products,
    selectedProductId,
    selectedProductArea,
    selectedCapability,
    selectedCapabilityParentKind,
    productAreaOrderIds,
    capabilityOrderMap,
    dependencyDraft,
    selectedManagementStoryId,
    managementStoryPageIndex,
    productPageTab,
    productManagementTab,
  } = data;
  const {
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
    setSelectedManagementStoryId,
    setFormError,
  } = state;

  const selection = useProductManagementSelection({
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

  const workItemMutations = useProductManagementWorkItemMutations({
    selectedProductId,
    selectedManagementFeatureNode: selection.selectedManagementFeatureNode,
    selectedManagementStory: selection.selectedManagementStory,
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

  return {
    ...selection,
    ...workItemMutations,
  };
}
