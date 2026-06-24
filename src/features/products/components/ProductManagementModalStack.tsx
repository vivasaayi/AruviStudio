import type { Dispatch, SetStateAction } from "react";

import type { Capability, HierarchyNodeKind, Product, ProductArea, WorkItem } from "../../../lib/types";
import type {
  ProductFormState,
  WorkItemDraftState,
} from "../lib/productListPageState";
import {
  DeleteHierarchyNodeModal,
  DeleteManagementWorkItemModal,
  ManagementWorkItemFormModal,
} from "./ProductManagementDeliveryModals";
import {
  CapabilityFormModal,
  ProductAreaFormModal,
} from "./ProductManagementHierarchyModals";
import {
  DeleteProductModal,
  ProductFormModal,
  ResetProductPlanModal,
} from "./ProductManagementProductModals";

type ProductAreaFormState = {
  name: string;
  description: string;
  purpose: string;
  nodeKind: HierarchyNodeKind;
};

type CapabilityFormState = {
  name: string;
  description: string;
  acceptanceCriteria: string;
  technicalNotes: string;
  nodeKind: HierarchyNodeKind;
};

type NodeKindGroup = {
  label: string;
  kinds: HierarchyNodeKind[];
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

type ProductManagementModalStackProps = {
  productDialogMode: "closed" | "create" | "edit";
  productForm: ProductFormState;
  productDraft: ProductFormState;
  setProductForm: Dispatch<SetStateAction<ProductFormState>>;
  setProductDraft: Dispatch<SetStateAction<ProductFormState>>;
  isCreateProductPending: boolean;
  isUpdateProductPending: boolean;
  onCloseProductDialog: () => void;
  onSubmitProduct: () => void;
  deleteProductCandidate: Product | null;
  deleteConfirmName: string;
  deleteConfirmArchive: boolean;
  deleteProductReady: boolean;
  isArchiveProductPending: boolean;
  onCloseDeleteProduct: () => void;
  onDeleteConfirmNameChange: (value: string) => void;
  onDeleteConfirmArchiveChange: (value: boolean) => void;
  onArchiveProduct: (productId: string) => void;
  resetPlanCandidate: Product | null;
  resetPlanConfirmName: string;
  resetPlanConfirmTree: boolean;
  resetPlanDeleteDelivery: boolean;
  resetPlanReady: boolean;
  isResetPlanPending: boolean;
  onCloseResetPlan: () => void;
  onResetPlanConfirmNameChange: (value: string) => void;
  onResetPlanConfirmTreeChange: (value: boolean) => void;
  onResetPlanDeleteDeliveryChange: (value: boolean) => void;
  onResetPlan: (data: { productId: string; deleteDelivery: boolean }) => void;
  deleteHierarchyCandidate: DeleteHierarchyCandidate | null;
  deleteHierarchyConfirmName: string;
  deleteHierarchyConfirmChecked: boolean;
  deleteHierarchyReady: boolean;
  isDeleteHierarchyPending: boolean;
  onCloseDeleteHierarchy: () => void;
  onDeleteHierarchyConfirmNameChange: (value: string) => void;
  onDeleteHierarchyConfirmCheckedChange: (value: boolean) => void;
  onDeleteHierarchy: (candidate: DeleteHierarchyCandidate) => void;
  deleteWorkItemCandidate: DeleteWorkItemCandidate | null;
  deleteWorkItemConfirmName: string;
  deleteWorkItemConfirmChecked: boolean;
  deleteWorkItemReady: boolean;
  isDeleteWorkItemPending: boolean;
  onCloseDeleteWorkItem: () => void;
  onDeleteWorkItemConfirmNameChange: (value: string) => void;
  onDeleteWorkItemConfirmCheckedChange: (value: boolean) => void;
  onDeleteWorkItem: (candidate: DeleteWorkItemCandidate) => void;
  storyDialogMode: "closed" | "create" | "edit";
  selectedFeatureTitle: string;
  storyDraft: WorkItemDraftState;
  setStoryDraft: Dispatch<SetStateAction<WorkItemDraftState>>;
  canSubmitStory: boolean;
  isCreateStoryPending: boolean;
  isUpdateStoryPending: boolean;
  onCloseStoryDialog: () => void;
  onSubmitStory: () => void;
  taskDialogMode: "closed" | "create" | "edit";
  selectedStoryTitle: string;
  taskDraft: WorkItemDraftState;
  setTaskDraft: Dispatch<SetStateAction<WorkItemDraftState>>;
  canSubmitTask: boolean;
  isCreateTaskPending: boolean;
  isUpdateTaskPending: boolean;
  onCloseTaskDialog: () => void;
  onSubmitTask: () => void;
  productAreaDialogMode: "closed" | "create" | "edit";
  selectedProductArea: ProductArea | null;
  productAreaForm: ProductAreaFormState;
  productAreaDraft: ProductAreaFormState;
  setProductAreaForm: Dispatch<SetStateAction<ProductAreaFormState>>;
  setProductAreaDraft: Dispatch<SetStateAction<ProductAreaFormState>>;
  selectedProductId: string | null;
  isCreateProductAreaPending: boolean;
  isUpdateProductAreaPending: boolean;
  onCloseProductAreaDialog: () => void;
  onSubmitProductArea: () => void;
  capabilityDialogMode: "closed" | "create" | "edit";
  selectedCapability: Capability | null;
  capabilityForm: CapabilityFormState;
  capabilityDraft: CapabilityFormState;
  setCapabilityForm: Dispatch<SetStateAction<CapabilityFormState>>;
  setCapabilityDraft: Dispatch<SetStateAction<CapabilityFormState>>;
  createKindGroups: NodeKindGroup[];
  editKindGroups: NodeKindGroup[];
  activeProductAreaId: string | null;
  isCreateCapabilityPending: boolean;
  isUpdateCapabilityPending: boolean;
  onCloseCapabilityDialog: () => void;
  onSubmitCapability: () => void;
  formError: string | null;
};

export function ProductManagementModalStack({
  productDialogMode,
  productForm,
  productDraft,
  setProductForm,
  setProductDraft,
  isCreateProductPending,
  isUpdateProductPending,
  onCloseProductDialog,
  onSubmitProduct,
  deleteProductCandidate,
  deleteConfirmName,
  deleteConfirmArchive,
  deleteProductReady,
  isArchiveProductPending,
  onCloseDeleteProduct,
  onDeleteConfirmNameChange,
  onDeleteConfirmArchiveChange,
  onArchiveProduct,
  resetPlanCandidate,
  resetPlanConfirmName,
  resetPlanConfirmTree,
  resetPlanDeleteDelivery,
  resetPlanReady,
  isResetPlanPending,
  onCloseResetPlan,
  onResetPlanConfirmNameChange,
  onResetPlanConfirmTreeChange,
  onResetPlanDeleteDeliveryChange,
  onResetPlan,
  deleteHierarchyCandidate,
  deleteHierarchyConfirmName,
  deleteHierarchyConfirmChecked,
  deleteHierarchyReady,
  isDeleteHierarchyPending,
  onCloseDeleteHierarchy,
  onDeleteHierarchyConfirmNameChange,
  onDeleteHierarchyConfirmCheckedChange,
  onDeleteHierarchy,
  deleteWorkItemCandidate,
  deleteWorkItemConfirmName,
  deleteWorkItemConfirmChecked,
  deleteWorkItemReady,
  isDeleteWorkItemPending,
  onCloseDeleteWorkItem,
  onDeleteWorkItemConfirmNameChange,
  onDeleteWorkItemConfirmCheckedChange,
  onDeleteWorkItem,
  storyDialogMode,
  selectedFeatureTitle,
  storyDraft,
  setStoryDraft,
  canSubmitStory,
  isCreateStoryPending,
  isUpdateStoryPending,
  onCloseStoryDialog,
  onSubmitStory,
  taskDialogMode,
  selectedStoryTitle,
  taskDraft,
  setTaskDraft,
  canSubmitTask,
  isCreateTaskPending,
  isUpdateTaskPending,
  onCloseTaskDialog,
  onSubmitTask,
  productAreaDialogMode,
  selectedProductArea,
  productAreaForm,
  productAreaDraft,
  setProductAreaForm,
  setProductAreaDraft,
  selectedProductId,
  isCreateProductAreaPending,
  isUpdateProductAreaPending,
  onCloseProductAreaDialog,
  onSubmitProductArea,
  capabilityDialogMode,
  selectedCapability,
  capabilityForm,
  capabilityDraft,
  setCapabilityForm,
  setCapabilityDraft,
  createKindGroups,
  editKindGroups,
  activeProductAreaId,
  isCreateCapabilityPending,
  isUpdateCapabilityPending,
  onCloseCapabilityDialog,
  onSubmitCapability,
  formError,
}: ProductManagementModalStackProps) {
  return (
    <>
      {productDialogMode !== "closed" && (
        <ProductFormModal
          mode={productDialogMode}
          productForm={productForm}
          productDraft={productDraft}
          setProductForm={setProductForm}
          setProductDraft={setProductDraft}
          formError={formError}
          isCreatePending={isCreateProductPending}
          isUpdatePending={isUpdateProductPending}
          onClose={onCloseProductDialog}
          onSubmit={onSubmitProduct}
        />
      )}

      {deleteProductCandidate && (
        <DeleteProductModal
          product={deleteProductCandidate}
          confirmName={deleteConfirmName}
          confirmArchive={deleteConfirmArchive}
          isReady={deleteProductReady}
          isPending={isArchiveProductPending}
          formError={formError}
          onClose={onCloseDeleteProduct}
          onConfirmNameChange={onDeleteConfirmNameChange}
          onConfirmArchiveChange={onDeleteConfirmArchiveChange}
          onArchive={onArchiveProduct}
        />
      )}

      {resetPlanCandidate && (
        <ResetProductPlanModal
          product={resetPlanCandidate}
          confirmName={resetPlanConfirmName}
          confirmTree={resetPlanConfirmTree}
          deleteDelivery={resetPlanDeleteDelivery}
          isReady={resetPlanReady}
          isPending={isResetPlanPending}
          formError={formError}
          onClose={onCloseResetPlan}
          onConfirmNameChange={onResetPlanConfirmNameChange}
          onConfirmTreeChange={onResetPlanConfirmTreeChange}
          onDeleteDeliveryChange={onResetPlanDeleteDeliveryChange}
          onReset={onResetPlan}
        />
      )}

      {deleteHierarchyCandidate && (
        <DeleteHierarchyNodeModal
          candidate={deleteHierarchyCandidate}
          confirmName={deleteHierarchyConfirmName}
          confirmChecked={deleteHierarchyConfirmChecked}
          isReady={deleteHierarchyReady}
          isPending={isDeleteHierarchyPending}
          formError={formError}
          onClose={onCloseDeleteHierarchy}
          onConfirmNameChange={onDeleteHierarchyConfirmNameChange}
          onConfirmCheckedChange={onDeleteHierarchyConfirmCheckedChange}
          onDelete={onDeleteHierarchy}
        />
      )}

      {deleteWorkItemCandidate && (
        <DeleteManagementWorkItemModal
          candidate={deleteWorkItemCandidate}
          confirmName={deleteWorkItemConfirmName}
          confirmChecked={deleteWorkItemConfirmChecked}
          isReady={deleteWorkItemReady}
          isPending={isDeleteWorkItemPending}
          formError={formError}
          onClose={onCloseDeleteWorkItem}
          onConfirmNameChange={onDeleteWorkItemConfirmNameChange}
          onConfirmCheckedChange={onDeleteWorkItemConfirmCheckedChange}
          onDelete={onDeleteWorkItem}
        />
      )}

      {storyDialogMode !== "closed" && (
        <ManagementWorkItemFormModal
          kind="story"
          mode={storyDialogMode}
          contextLabel="Feature"
          contextTitle={selectedFeatureTitle}
          draft={storyDraft}
          setDraft={setStoryDraft}
          canSubmit={canSubmitStory}
          isCreatePending={isCreateStoryPending}
          isUpdatePending={isUpdateStoryPending}
          formError={formError}
          onClose={onCloseStoryDialog}
          onSubmit={onSubmitStory}
        />
      )}

      {taskDialogMode !== "closed" && (
        <ManagementWorkItemFormModal
          kind="task"
          mode={taskDialogMode}
          contextLabel="Story"
          contextTitle={selectedStoryTitle}
          draft={taskDraft}
          setDraft={setTaskDraft}
          canSubmit={canSubmitTask}
          isCreatePending={isCreateTaskPending}
          isUpdatePending={isUpdateTaskPending}
          formError={formError}
          onClose={onCloseTaskDialog}
          onSubmit={onSubmitTask}
        />
      )}

      {productAreaDialogMode !== "closed" && (
        <ProductAreaFormModal
          mode={productAreaDialogMode}
          selectedProductArea={selectedProductArea}
          form={productAreaForm}
          draft={productAreaDraft}
          setForm={setProductAreaForm}
          setDraft={setProductAreaDraft}
          formError={formError}
          selectedProductId={selectedProductId}
          isCreatePending={isCreateProductAreaPending}
          isUpdatePending={isUpdateProductAreaPending}
          onClose={onCloseProductAreaDialog}
          onSubmit={onSubmitProductArea}
        />
      )}

      {capabilityDialogMode !== "closed" && (
        <CapabilityFormModal
          mode={capabilityDialogMode}
          selectedProductArea={selectedProductArea}
          selectedCapability={selectedCapability}
          form={capabilityForm}
          draft={capabilityDraft}
          setForm={setCapabilityForm}
          setDraft={setCapabilityDraft}
          createKindGroups={createKindGroups}
          editKindGroups={editKindGroups}
          formError={formError}
          activeProductAreaId={activeProductAreaId}
          isCreatePending={isCreateCapabilityPending}
          isUpdatePending={isUpdateCapabilityPending}
          onClose={onCloseCapabilityDialog}
          onSubmit={onSubmitCapability}
        />
      )}
    </>
  );
}
