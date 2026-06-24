import { useEffect, type Dispatch, type SetStateAction } from "react";

import { getDefaultChildNodeKind } from "../../../lib/hierarchyLabels";
import type {
  Capability,
  HierarchyTreeNode,
  Product,
  ProductArea,
} from "../../../lib/types";
import {
  createEmptyCapabilityForm,
  emptyProductAreaForm,
  emptyProductForm,
  productToForm,
  type CapabilityFormState,
  type ProductAreaFormState,
  type ProductFormState,
} from "../lib/productListPageState";
import type { ProductManagementTab } from "../lib/productRefreshScopes";

type DialogMode = "closed" | "create" | "edit";
type ProductPageSyncInput = {
  isLoading: boolean;
  activeProductId: string | null;
  selectedProductId: string | null;
  products: Product[] | undefined;
  setActiveProduct: (productId: string | null) => void;
  statusProductId: string;
  setStatusProductId: (productId: string) => void;
  setActiveWorkItem: (workItemId: string | null) => void;
  activeProductAreaId: string | null;
  activeCapabilityId: string | null;
  selectedProduct: Product | null | undefined;
  setProductForm: Dispatch<SetStateAction<ProductFormState>>;
  setProductDraft: Dispatch<SetStateAction<ProductFormState>>;
  productDialogMode: DialogMode;
  selectedProductArea: ProductArea | null;
  setProductAreaForm: Dispatch<SetStateAction<ProductAreaFormState>>;
  setProductAreaDraft: Dispatch<SetStateAction<ProductAreaFormState>>;
  productAreaDialogMode: DialogMode;
  selectedCapability: Capability | null;
  setCapabilityForm: Dispatch<SetStateAction<CapabilityFormState>>;
  setCapabilityDraft: Dispatch<SetStateAction<CapabilityFormState>>;
  capabilityDialogMode: DialogMode;
  selectedManagementFeatureNode: HierarchyTreeNode | null;
  productManagementTab: ProductManagementTab;
  setSelectedManagementStoryId: Dispatch<SetStateAction<string | null>>;
  setManagementStoryPageIndex: Dispatch<SetStateAction<number>>;
  setFormError: Dispatch<SetStateAction<string | null>>;
};

export function useProductPageSync({
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
}: ProductPageSyncInput) {
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
  }, [products, setStatusProductId, statusProductId]);

  useEffect(() => {
    if (!activeProductId && products?.[0]?.id) {
      setActiveProduct(products[0].id);
    }
  }, [activeProductId, products, setActiveProduct]);

  useEffect(() => {
    setActiveWorkItem(null);
    setFormError(null);
  }, [selectedProductId, activeProductAreaId, activeCapabilityId, setActiveWorkItem, setFormError]);

  useEffect(() => {
    if (selectedProduct) {
      setProductDraft(productToForm(selectedProduct));
    }
  }, [selectedProduct, setProductDraft]);

  useEffect(() => {
    setFormError(null);
    if (productDialogMode === "create") {
      setProductForm(emptyProductForm);
    }
  }, [productDialogMode, setFormError, setProductForm]);

  useEffect(() => {
    if (productAreaDialogMode === "create") {
      setProductAreaForm(emptyProductAreaForm);
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
  }, [productAreaDialogMode, selectedProductArea, setProductAreaDraft, setProductAreaForm]);

  useEffect(() => {
    if (capabilityDialogMode === "create") {
      setCapabilityForm(createEmptyCapabilityForm(
        getDefaultChildNodeKind(selectedCapability?.node_kind ?? selectedProductArea?.node_kind),
      ));
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
  }, [
    capabilityDialogMode,
    selectedCapability,
    selectedProductArea,
    setCapabilityDraft,
    setCapabilityForm,
    setFormError,
  ]);

  useEffect(() => {
    setManagementStoryPageIndex(0);
    setSelectedManagementStoryId(null);
  }, [
    selectedManagementFeatureNode?.id,
    selectedProductId,
    productManagementTab,
    setManagementStoryPageIndex,
    setSelectedManagementStoryId,
  ]);
}
