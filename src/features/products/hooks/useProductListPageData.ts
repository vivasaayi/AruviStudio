import { useProductCatalogControls } from "./useProductCatalogControls";
import { useProductHierarchySelectionState } from "./useProductHierarchySelectionState";
import { useProductListPageState } from "./useProductListPageState";
import { useProductPageRuntimeContext } from "./useProductPageRuntimeContext";
import { useProductPageViewModel } from "./useProductPageViewModel";

type ProductListPageDataInput = {
  activeProductId: string | null;
  activeProductAreaId: string | null;
  activeCapabilityId: string | null;
  queryClient: ReturnType<typeof useProductPageRuntimeContext>["queryClient"];
  state: ReturnType<typeof useProductListPageState>;
};

export function useProductListPageData({
  activeProductId,
  activeProductAreaId,
  activeCapabilityId,
  queryClient,
  state,
}: ProductListPageDataInput) {
  const {
    productPageTab,
    productManagementTab,
    dependencyDraft,
    selectedManagementStoryId,
    managementStoryPageIndex,
  } = state;

  const catalogControls = useProductCatalogControls({
    queryClient,
  });

  const viewModel = useProductPageViewModel({
    activeProductId,
    productPageTab,
    productSearch: catalogControls.productSearch,
    productStatusFilter: catalogControls.productStatusFilter,
    productSourceFilter: catalogControls.productSourceFilter,
    productTagFilter: catalogControls.productTagFilter,
    productSort: catalogControls.productSort,
    showDefaultProductsInTable: catalogControls.showDefaultProductsInTable,
    showCustomProductsInTable: catalogControls.showCustomProductsInTable,
    statusProductId: catalogControls.statusProductId,
    statusDepth: catalogControls.statusDepth,
    statusGroupBy: catalogControls.statusGroupBy,
  });

  const hierarchySelection = useProductHierarchySelectionState({
    tree: viewModel.tree,
    activeProductAreaId,
    activeCapabilityId,
  });

  return {
    ...catalogControls,
    ...viewModel,
    ...hierarchySelection,
    productPageTab,
    productManagementTab,
    dependencyDraft,
    selectedManagementStoryId,
    managementStoryPageIndex,
  };
}
