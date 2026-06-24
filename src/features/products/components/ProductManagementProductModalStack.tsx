import type { ProductManagementModalStackProps } from "./ProductManagementModalStack";
import {
  DeleteProductModal,
  ProductFormModal,
  ResetProductPlanModal,
} from "./ProductManagementProductModals";

type ProductManagementProductModalStackProps = Pick<
  ProductManagementModalStackProps,
  | "productDialogMode"
  | "productForm"
  | "productDraft"
  | "setProductForm"
  | "setProductDraft"
  | "isCreateProductPending"
  | "isUpdateProductPending"
  | "onCloseProductDialog"
  | "onSubmitProduct"
  | "deleteProductCandidate"
  | "deleteConfirmName"
  | "deleteConfirmArchive"
  | "deleteProductReady"
  | "isArchiveProductPending"
  | "onCloseDeleteProduct"
  | "onDeleteConfirmNameChange"
  | "onDeleteConfirmArchiveChange"
  | "onArchiveProduct"
  | "resetPlanCandidate"
  | "resetPlanConfirmName"
  | "resetPlanConfirmTree"
  | "resetPlanDeleteDelivery"
  | "resetPlanReady"
  | "isResetPlanPending"
  | "onCloseResetPlan"
  | "onResetPlanConfirmNameChange"
  | "onResetPlanConfirmTreeChange"
  | "onResetPlanDeleteDeliveryChange"
  | "onResetPlan"
  | "formError"
>;

export function ProductManagementProductModalStack({
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
  formError,
}: ProductManagementProductModalStackProps) {
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
    </>
  );
}
