import { useState } from "react";
import type { Product, WorkItem } from "../../../lib/types";
import {
  emptyProductDependencyDraft,
  emptyProductForm,
  emptyWorkItemDraft,
  type CapabilityFormState,
  type ProductAreaFormState,
  type ProductDependencyDraft,
  type ProductFormState,
  type WorkItemDraftState,
} from "../lib/productListPageState";
import type {
  ProductManagementTab,
  ProductPageTab,
} from "../lib/productRefreshScopes";

type ProductListPageStateOptions = {
  isProductDetailRoute: boolean;
};

export function useProductListPageState({
  isProductDetailRoute,
}: ProductListPageStateOptions) {
  const [productForm, setProductForm] =
    useState<ProductFormState>(emptyProductForm);
  const [productDraft, setProductDraft] =
    useState<ProductFormState>(emptyProductForm);
  const [productAreaForm, setProductAreaForm] =
    useState<ProductAreaFormState>({
      name: "",
      description: "",
      purpose: "",
      nodeKind: "product_area",
    });
  const [productAreaDraft, setProductAreaDraft] =
    useState<ProductAreaFormState>({
      name: "",
      description: "",
      purpose: "",
      nodeKind: "product_area",
    });
  const [capabilityForm, setCapabilityForm] = useState<CapabilityFormState>({
    name: "",
    description: "",
    acceptanceCriteria: "",
    technicalNotes: "",
    nodeKind: "capability",
  });
  const [capabilityDraft, setCapabilityDraft] = useState<CapabilityFormState>({
    name: "",
    description: "",
    acceptanceCriteria: "",
    technicalNotes: "",
    nodeKind: "capability",
  });
  const [productManagementTab, setProductManagementTab] =
    useState<ProductManagementTab>("areas");
  const [formError, setFormError] = useState<string | null>(null);
  const [productPageTab, setProductPageTab] = useState<ProductPageTab>(() =>
    isProductDetailRoute ? "design" : "list",
  );
  const [dependencyDraft, setDependencyDraft] =
    useState<ProductDependencyDraft>(emptyProductDependencyDraft);
  const [deleteProductCandidate, setDeleteProductCandidate] =
    useState<Product | null>(null);
  const [deleteConfirmName, setDeleteConfirmName] = useState("");
  const [deleteConfirmArchive, setDeleteConfirmArchive] = useState(false);
  const [resetPlanCandidate, setResetPlanCandidate] =
    useState<Product | null>(null);
  const [resetPlanConfirmName, setResetPlanConfirmName] = useState("");
  const [resetPlanConfirmTree, setResetPlanConfirmTree] = useState(false);
  const [resetPlanDeleteDelivery, setResetPlanDeleteDelivery] =
    useState(false);
  const [deleteHierarchyCandidate, setDeleteHierarchyCandidate] =
    useState<null | {
      kind: "product_area" | "capability" | "feature";
      id: string;
      name: string;
    }>(null);
  const [deleteHierarchyConfirmName, setDeleteHierarchyConfirmName] =
    useState("");
  const [deleteHierarchyConfirmChecked, setDeleteHierarchyConfirmChecked] =
    useState(false);
  const [selectedManagementStoryId, setSelectedManagementStoryId] = useState<
    string | null
  >(null);
  const [managementStoryPageIndex, setManagementStoryPageIndex] = useState(0);
  const [storyDialogMode, setStoryDialogMode] = useState<
    "closed" | "create" | "edit"
  >("closed");
  const [taskDialogMode, setTaskDialogMode] = useState<
    "closed" | "create" | "edit"
  >("closed");
  const [editingStory, setEditingStory] = useState<WorkItem | null>(null);
  const [editingTask, setEditingTask] = useState<WorkItem | null>(null);
  const [deleteWorkItemCandidate, setDeleteWorkItemCandidate] =
    useState<null | { workItem: WorkItem; kind: "story" | "task" }>(null);
  const [deleteWorkItemConfirmName, setDeleteWorkItemConfirmName] =
    useState("");
  const [deleteWorkItemConfirmChecked, setDeleteWorkItemConfirmChecked] =
    useState(false);
  const [storyDraft, setStoryDraft] =
    useState<WorkItemDraftState>(emptyWorkItemDraft);
  const [taskDraft, setTaskDraft] =
    useState<WorkItemDraftState>(emptyWorkItemDraft);
  const [copiedEntityId, setCopiedEntityId] = useState<string | null>(null);

  return {
    productForm,
    setProductForm,
    productDraft,
    setProductDraft,
    productAreaForm,
    setProductAreaForm,
    productAreaDraft,
    setProductAreaDraft,
    capabilityForm,
    setCapabilityForm,
    capabilityDraft,
    setCapabilityDraft,
    productManagementTab,
    setProductManagementTab,
    formError,
    setFormError,
    productPageTab,
    setProductPageTab,
    dependencyDraft,
    setDependencyDraft,
    deleteProductCandidate,
    setDeleteProductCandidate,
    deleteConfirmName,
    setDeleteConfirmName,
    deleteConfirmArchive,
    setDeleteConfirmArchive,
    resetPlanCandidate,
    setResetPlanCandidate,
    resetPlanConfirmName,
    setResetPlanConfirmName,
    resetPlanConfirmTree,
    setResetPlanConfirmTree,
    resetPlanDeleteDelivery,
    setResetPlanDeleteDelivery,
    deleteHierarchyCandidate,
    setDeleteHierarchyCandidate,
    deleteHierarchyConfirmName,
    setDeleteHierarchyConfirmName,
    deleteHierarchyConfirmChecked,
    setDeleteHierarchyConfirmChecked,
    selectedManagementStoryId,
    setSelectedManagementStoryId,
    managementStoryPageIndex,
    setManagementStoryPageIndex,
    storyDialogMode,
    setStoryDialogMode,
    taskDialogMode,
    setTaskDialogMode,
    editingStory,
    setEditingStory,
    editingTask,
    setEditingTask,
    deleteWorkItemCandidate,
    setDeleteWorkItemCandidate,
    deleteWorkItemConfirmName,
    setDeleteWorkItemConfirmName,
    deleteWorkItemConfirmChecked,
    setDeleteWorkItemConfirmChecked,
    storyDraft,
    setStoryDraft,
    taskDraft,
    setTaskDraft,
    copiedEntityId,
    setCopiedEntityId,
  };
}
