import type { Dispatch, SetStateAction } from "react";
import { useMutation, type QueryClient } from "@tanstack/react-query";

import {
  archiveProduct,
  createCapability,
  createProduct,
  createProductArea,
  createProductDependency,
  deleteCapability,
  deleteProductArea,
  reorderCapabilities,
  reorderProductAreas,
  resetProductPlan,
  updateCapability,
  updateProduct,
  updateProductArea,
} from "../../../lib/tauri";
import { getDefaultChildNodeKind } from "../../../lib/hierarchyLabels";
import type { Capability, Product, ProductArea } from "../../../lib/types";
import {
  emptyProductDependencyDraft,
  emptyProductForm,
  type CapabilityFormState,
  type ProductAreaFormState,
  type ProductDependencyDraft,
  type ProductFormState,
} from "../lib/productListPageState";

type DeleteHierarchyCandidate = {
  kind: "product_area" | "capability" | "feature";
  id: string;
  name: string;
};

type ProductHierarchyMutationsInput = {
  queryClient: QueryClient;
  selectedProductId: string | null;
  activeProductAreaId: string | null;
  activeCapabilityId: string | null;
  selectedProductArea: ProductArea | null;
  selectedCapability: Capability | null;
  productForm: ProductFormState;
  setProductForm: Dispatch<SetStateAction<ProductFormState>>;
  productDraft: ProductFormState;
  productAreaForm: ProductAreaFormState;
  setProductAreaForm: Dispatch<SetStateAction<ProductAreaFormState>>;
  productAreaDraft: ProductAreaFormState;
  capabilityForm: CapabilityFormState;
  setCapabilityForm: Dispatch<SetStateAction<CapabilityFormState>>;
  capabilityDraft: CapabilityFormState;
  dependencyDraft: ProductDependencyDraft;
  setDependencyDraft: Dispatch<SetStateAction<ProductDependencyDraft>>;
  statusProductId: string;
  setStatusProductId: Dispatch<SetStateAction<string>>;
  closeProductDialog: () => void;
  closeProductAreaDialog: () => void;
  closeCapabilityDialog: () => void;
  setProductWorkspaceTab: (tab: "book" | "structure" | "delivery") => void;
  setActiveProduct: (productId: string | null) => void;
  setActiveProductArea: (productAreaId: string | null) => void;
  setActiveCapability: (capabilityId: string | null) => void;
  setDeleteProductCandidate: Dispatch<SetStateAction<Product | null>>;
  setDeleteConfirmName: Dispatch<SetStateAction<string>>;
  setDeleteConfirmArchive: Dispatch<SetStateAction<boolean>>;
  setResetPlanCandidate: Dispatch<SetStateAction<Product | null>>;
  setResetPlanConfirmName: Dispatch<SetStateAction<string>>;
  setResetPlanConfirmTree: Dispatch<SetStateAction<boolean>>;
  setResetPlanDeleteDelivery: Dispatch<SetStateAction<boolean>>;
  setDeleteHierarchyCandidate: Dispatch<SetStateAction<DeleteHierarchyCandidate | null>>;
  setDeleteHierarchyConfirmName: Dispatch<SetStateAction<string>>;
  setDeleteHierarchyConfirmChecked: Dispatch<SetStateAction<boolean>>;
  setFormError: Dispatch<SetStateAction<string | null>>;
};

export function useProductHierarchyMutations({
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
}: ProductHierarchyMutationsInput) {
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
    mutationFn: async (candidate: DeleteHierarchyCandidate) => {
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

  return {
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
  };
}
