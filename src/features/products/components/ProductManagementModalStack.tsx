import type { Dispatch, SetStateAction } from "react";

import type { Capability, HierarchyNodeKind, Product, ProductArea, WorkItem } from "../../../lib/types";
import type {
  ProductFormState,
  WorkItemDraftState,
} from "../lib/productListPageState";
import {
  CapabilityFormModal,
  ProductAreaFormModal,
} from "./ProductManagementHierarchyModals";
import { ProductManagementDeliveryModalStack } from "./ProductManagementDeliveryModalStack";
import { ProductManagementProductModalStack } from "./ProductManagementProductModalStack";

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

export type ProductManagementModalStackProps = {
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
      <ProductManagementProductModalStack
        productDialogMode={productDialogMode}
        productForm={productForm}
        productDraft={productDraft}
        setProductForm={setProductForm}
        setProductDraft={setProductDraft}
        isCreateProductPending={isCreateProductPending}
        isUpdateProductPending={isUpdateProductPending}
        onCloseProductDialog={onCloseProductDialog}
        onSubmitProduct={onSubmitProduct}
        deleteProductCandidate={deleteProductCandidate}
        deleteConfirmName={deleteConfirmName}
        deleteConfirmArchive={deleteConfirmArchive}
        deleteProductReady={deleteProductReady}
        isArchiveProductPending={isArchiveProductPending}
        onCloseDeleteProduct={onCloseDeleteProduct}
        onDeleteConfirmNameChange={onDeleteConfirmNameChange}
        onDeleteConfirmArchiveChange={onDeleteConfirmArchiveChange}
        onArchiveProduct={onArchiveProduct}
        resetPlanCandidate={resetPlanCandidate}
        resetPlanConfirmName={resetPlanConfirmName}
        resetPlanConfirmTree={resetPlanConfirmTree}
        resetPlanDeleteDelivery={resetPlanDeleteDelivery}
        resetPlanReady={resetPlanReady}
        isResetPlanPending={isResetPlanPending}
        onCloseResetPlan={onCloseResetPlan}
        onResetPlanConfirmNameChange={onResetPlanConfirmNameChange}
        onResetPlanConfirmTreeChange={onResetPlanConfirmTreeChange}
        onResetPlanDeleteDeliveryChange={onResetPlanDeleteDeliveryChange}
        onResetPlan={onResetPlan}
        formError={formError}
      />

      <ProductManagementDeliveryModalStack
        deleteHierarchyCandidate={deleteHierarchyCandidate}
        deleteHierarchyConfirmName={deleteHierarchyConfirmName}
        deleteHierarchyConfirmChecked={deleteHierarchyConfirmChecked}
        deleteHierarchyReady={deleteHierarchyReady}
        isDeleteHierarchyPending={isDeleteHierarchyPending}
        onCloseDeleteHierarchy={onCloseDeleteHierarchy}
        onDeleteHierarchyConfirmNameChange={onDeleteHierarchyConfirmNameChange}
        onDeleteHierarchyConfirmCheckedChange={onDeleteHierarchyConfirmCheckedChange}
        onDeleteHierarchy={onDeleteHierarchy}
        deleteWorkItemCandidate={deleteWorkItemCandidate}
        deleteWorkItemConfirmName={deleteWorkItemConfirmName}
        deleteWorkItemConfirmChecked={deleteWorkItemConfirmChecked}
        deleteWorkItemReady={deleteWorkItemReady}
        isDeleteWorkItemPending={isDeleteWorkItemPending}
        onCloseDeleteWorkItem={onCloseDeleteWorkItem}
        onDeleteWorkItemConfirmNameChange={onDeleteWorkItemConfirmNameChange}
        onDeleteWorkItemConfirmCheckedChange={onDeleteWorkItemConfirmCheckedChange}
        onDeleteWorkItem={onDeleteWorkItem}
        storyDialogMode={storyDialogMode}
        selectedFeatureTitle={selectedFeatureTitle}
        storyDraft={storyDraft}
        setStoryDraft={setStoryDraft}
        canSubmitStory={canSubmitStory}
        isCreateStoryPending={isCreateStoryPending}
        isUpdateStoryPending={isUpdateStoryPending}
        onCloseStoryDialog={onCloseStoryDialog}
        onSubmitStory={onSubmitStory}
        taskDialogMode={taskDialogMode}
        selectedStoryTitle={selectedStoryTitle}
        taskDraft={taskDraft}
        setTaskDraft={setTaskDraft}
        canSubmitTask={canSubmitTask}
        isCreateTaskPending={isCreateTaskPending}
        isUpdateTaskPending={isUpdateTaskPending}
        onCloseTaskDialog={onCloseTaskDialog}
        onSubmitTask={onSubmitTask}
        formError={formError}
      />

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
