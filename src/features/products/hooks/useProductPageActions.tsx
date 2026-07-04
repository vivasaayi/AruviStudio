import type { Dispatch, SetStateAction } from "react";
import type { NavigateFunction } from "react-router-dom";

import type {
  CapabilityTree,
  HierarchyTreeNode,
  Product,
  ProductAreaTree,
  WorkItem,
} from "../../../lib/types";
import { CopyableEntityId } from "../components/CopyableEntityId";
import {
  createEmptyCapabilityForm,
  emptyWorkItemDraft,
  productToForm,
  workItemToDraft,
  type CapabilityFormState,
  type ProductAreaFormState,
  type ProductFormState,
  type WorkItemDraftState,
} from "../lib/productListPageState";
import type { ProductPageTab } from "../lib/productRefreshScopes";

type DialogMode = "closed" | "create" | "edit";
type HierarchySelection = {
  nodeId: string;
  nodeType: "product_area" | "capability";
  productAreaId: string | null;
  capabilityId: string | null;
};
type DeleteHierarchyCandidate = {
  kind: "product_area" | "capability" | "feature";
  id: string;
  name: string;
};
type DeleteWorkItemCandidate = {
  workItem: WorkItem;
  kind: "story" | "task";
};

type ProductPageActionsInput = {
  navigate: NavigateFunction;
  copiedEntityId: string | null;
  setCopiedEntityId: Dispatch<SetStateAction<string | null>>;
  setActiveProduct: (productId: string | null) => void;
  setActiveHierarchyNode: (selection: HierarchySelection) => void;
  setActiveWorkItem: (workItemId: string | null) => void;
  setActiveView: (view: "work-items") => void;
  setStatusProductId: (productId: string) => void;
  setProductPageTab: Dispatch<SetStateAction<ProductPageTab>>;
  setProductDraft: Dispatch<SetStateAction<ProductFormState>>;
  openProductDialog: (mode: Exclude<DialogMode, "closed">) => void;
  openProductAreaDialog: (mode: Exclude<DialogMode, "closed">) => void;
  openCapabilityDialog: (mode: Exclude<DialogMode, "closed">) => void;
  setProductAreaDraft: Dispatch<SetStateAction<ProductAreaFormState>>;
  setCapabilityForm: Dispatch<SetStateAction<CapabilityFormState>>;
  setCapabilityDraft: Dispatch<SetStateAction<CapabilityFormState>>;
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
  setSelectedManagementStoryId: Dispatch<SetStateAction<string | null>>;
  setStoryDialogMode: Dispatch<SetStateAction<DialogMode>>;
  setTaskDialogMode: Dispatch<SetStateAction<DialogMode>>;
  setEditingStory: Dispatch<SetStateAction<WorkItem | null>>;
  setEditingTask: Dispatch<SetStateAction<WorkItem | null>>;
  setStoryDraft: Dispatch<SetStateAction<WorkItemDraftState>>;
  setTaskDraft: Dispatch<SetStateAction<WorkItemDraftState>>;
  setDeleteWorkItemCandidate: Dispatch<SetStateAction<DeleteWorkItemCandidate | null>>;
  setDeleteWorkItemConfirmName: Dispatch<SetStateAction<string>>;
  setDeleteWorkItemConfirmChecked: Dispatch<SetStateAction<boolean>>;
  setFormError: Dispatch<SetStateAction<string | null>>;
};

export function useProductPageActions({
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
}: ProductPageActionsInput) {
  const selectProductArea = (productAreaTree: ProductAreaTree) => {
    setActiveHierarchyNode({
      nodeId: productAreaTree.product_area.id,
      nodeType: "product_area",
      productAreaId: productAreaTree.product_area.id,
      capabilityId: null,
    });
  };

  const selectCapabilityForManagement = (capabilityTree: CapabilityTree) => {
    setActiveHierarchyNode({
      nodeId: capabilityTree.capability.id,
      nodeType: "capability",
      productAreaId: capabilityTree.capability.product_area_id,
      capabilityId: capabilityTree.capability.id,
    });
  };

  const openCreateCapabilityForArea = (productAreaTree: ProductAreaTree) => {
    selectProductArea(productAreaTree);
    setCapabilityForm(createEmptyCapabilityForm("capability"));
    openCapabilityDialog("create");
  };

  const openCreateFeatureForCapability = (capabilityTree: CapabilityTree) => {
    selectCapabilityForManagement(capabilityTree);
    setCapabilityForm(createEmptyCapabilityForm("feature"));
    openCapabilityDialog("create");
  };

  const openEditProductArea = (productAreaTree: ProductAreaTree) => {
    selectProductArea(productAreaTree);
    setProductAreaDraft({
      name: productAreaTree.product_area.name,
      description: productAreaTree.product_area.description,
      purpose: productAreaTree.product_area.purpose,
      nodeKind: productAreaTree.product_area.node_kind,
    });
    openProductAreaDialog("edit");
  };

  const openEditCapabilityNode = (capabilityTree: CapabilityTree) => {
    selectCapabilityForManagement(capabilityTree);
    setCapabilityDraft({
      name: capabilityTree.capability.name,
      description: capabilityTree.capability.description,
      acceptanceCriteria: capabilityTree.capability.acceptance_criteria,
      technicalNotes: capabilityTree.capability.technical_notes,
      nodeKind: capabilityTree.capability.node_kind,
    });
    openCapabilityDialog("edit");
  };

  const requestDeleteHierarchyNode = (candidate: DeleteHierarchyCandidate) => {
    setDeleteHierarchyCandidate(candidate);
    setDeleteHierarchyConfirmName("");
    setDeleteHierarchyConfirmChecked(false);
    setFormError(null);
  };

  const openFeatureInBuilder = (featureNode: HierarchyTreeNode | null) => {
    if (featureNode) {
      setActiveHierarchyNode({
        nodeId: featureNode.id,
        nodeType: featureNode.node_type,
        productAreaId: featureNode.product_area_id,
        capabilityId: featureNode.capability_id,
      });
    }
    setActiveView("work-items");
    navigate("/work-items");
  };

  const openStoryInBuilder = (story: WorkItem) => {
    setSelectedManagementStoryId(story.id);
    setActiveWorkItem(story.id);
    setActiveView("work-items");
    navigate("/work-items");
  };

  const copyEntityId = async (id: string) => {
    try {
      await navigator.clipboard.writeText(id);
      setCopiedEntityId(id);
      setTimeout(() => setCopiedEntityId((current) => current === id ? null : current), 1800);
    } catch {
      setCopiedEntityId(null);
    }
  };

  const renderCopyableEntityId = (label: string, id: string) => {
    return (
      <CopyableEntityId
        label={label}
        id={id}
        isCopied={copiedEntityId === id}
        onCopy={(entityId) => void copyEntityId(entityId)}
      />
    );
  };

  const editProductFromList = (product: Product) => {
    setActiveProduct(product.id);
    setProductDraft(productToForm(product));
    setFormError(null);
    openProductDialog("edit");
  };

  const openProductDesign = (product: Product) => {
    setActiveProduct(product.id);
    setStatusProductId(product.id);
    setProductPageTab("design");
    navigate(`/products/${product.id}`);
  };

  const openProductOverview = (product: Product) => {
    setActiveProduct(product.id);
    setStatusProductId(product.id);
    setProductPageTab("overview");
  };

  const openProductStatus = (product: Product) => {
    setActiveProduct(product.id);
    setStatusProductId(product.id);
    setProductPageTab("status");
  };

  const openProductDependencies = (product: Product) => {
    setActiveProduct(product.id);
    setStatusProductId(product.id);
    setProductPageTab("dependencies");
  };

  const requestArchiveProduct = (product: Product) => {
    setDeleteProductCandidate(product);
    setDeleteConfirmName("");
    setDeleteConfirmArchive(false);
    setFormError(null);
  };

  const requestResetProductPlan = (product: Product) => {
    setResetPlanCandidate(product);
    setResetPlanConfirmName("");
    setResetPlanConfirmTree(false);
    setResetPlanDeleteDelivery(false);
    setFormError(null);
  };

  const openCreateStoryDialog = () => {
    setEditingStory(null);
    setStoryDraft(emptyWorkItemDraft);
    setStoryDialogMode("create");
    setFormError(null);
  };

  const openEditStoryDialog = (story: WorkItem) => {
    setEditingStory(story);
    setStoryDraft(workItemToDraft(story));
    setStoryDialogMode("edit");
    setFormError(null);
  };

  const openCreateTaskDialog = () => {
    setEditingTask(null);
    setTaskDraft(emptyWorkItemDraft);
    setTaskDialogMode("create");
    setFormError(null);
  };

  const openEditTaskDialog = (task: WorkItem) => {
    setEditingTask(task);
    setTaskDraft(workItemToDraft(task));
    setTaskDialogMode("edit");
    setFormError(null);
  };

  const requestDeleteWorkItem = (workItem: WorkItem, kind: DeleteWorkItemCandidate["kind"]) => {
    setDeleteWorkItemCandidate({ workItem, kind });
    setDeleteWorkItemConfirmName("");
    setDeleteWorkItemConfirmChecked(false);
    setFormError(null);
  };

  return {
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
  };
}
