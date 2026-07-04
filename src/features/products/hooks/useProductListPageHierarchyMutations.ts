import { useProductHierarchyMutations } from "./useProductHierarchyMutations";
import { useProductListPageData } from "./useProductListPageData";
import { useProductListPageState } from "./useProductListPageState";
import { useProductPageRuntimeContext } from "./useProductPageRuntimeContext";

type ProductListPageHierarchyMutationsInput = {
  activeProductAreaId: string | null;
  activeCapabilityId: string | null;
  closeCapabilityDialog: ReturnType<typeof useProductPageRuntimeContext>["ui"]["closeCapabilityDialog"];
  closeProductAreaDialog: ReturnType<typeof useProductPageRuntimeContext>["ui"]["closeProductAreaDialog"];
  closeProductDialog: ReturnType<typeof useProductPageRuntimeContext>["ui"]["closeProductDialog"];
  data: ReturnType<typeof useProductListPageData>;
  queryClient: ReturnType<typeof useProductPageRuntimeContext>["queryClient"];
  setActiveCapability: ReturnType<typeof useProductPageRuntimeContext>["workspace"]["setActiveCapability"];
  setActiveProduct: ReturnType<typeof useProductPageRuntimeContext>["workspace"]["setActiveProduct"];
  setActiveProductArea: ReturnType<typeof useProductPageRuntimeContext>["workspace"]["setActiveProductArea"];
  setProductWorkspaceTab: ReturnType<typeof useProductPageRuntimeContext>["ui"]["setProductWorkspaceTab"];
  state: ReturnType<typeof useProductListPageState>;
};

export function useProductListPageHierarchyMutations({
  activeProductAreaId,
  activeCapabilityId,
  closeCapabilityDialog,
  closeProductAreaDialog,
  closeProductDialog,
  data,
  queryClient,
  setActiveCapability,
  setActiveProduct,
  setActiveProductArea,
  setProductWorkspaceTab,
  state,
}: ProductListPageHierarchyMutationsInput) {
  const {
    capabilityDraft,
    capabilityForm,
    dependencyDraft,
    productAreaDraft,
    productAreaForm,
    productDraft,
    productForm,
    setCapabilityForm,
    setDeleteConfirmArchive,
    setDeleteConfirmName,
    setDeleteHierarchyCandidate,
    setDeleteHierarchyConfirmChecked,
    setDeleteHierarchyConfirmName,
    setDeleteProductCandidate,
    setDependencyDraft,
    setFormError,
    setProductAreaForm,
    setProductForm,
    setResetPlanCandidate,
    setResetPlanConfirmName,
    setResetPlanConfirmTree,
    setResetPlanDeleteDelivery,
  } = state;

  return useProductHierarchyMutations({
    queryClient,
    selectedProductId: data.selectedProductId,
    activeProductAreaId,
    activeCapabilityId,
    selectedProductArea: data.selectedProductArea,
    selectedCapability: data.selectedCapability,
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
    statusProductId: data.statusProductId,
    setStatusProductId: data.setStatusProductId,
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
}
