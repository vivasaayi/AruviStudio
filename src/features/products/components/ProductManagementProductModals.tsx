import type { Dispatch, SetStateAction } from "react";

import type { Product } from "../../../lib/types";
import {
  productHealthOptions,
  productInvestmentOptions,
  productLifecycleOptions,
  type ProductFormState,
} from "../lib/productListPageState";
import { styles } from "../lib/productListPageStyles";
import { ProductManagementModalShell as ModalShell } from "./ProductManagementModalShell";

type ProductDialogMode = "create" | "edit";

type ProductFormModalProps = {
  mode: ProductDialogMode;
  productForm: ProductFormState;
  productDraft: ProductFormState;
  setProductForm: Dispatch<SetStateAction<ProductFormState>>;
  setProductDraft: Dispatch<SetStateAction<ProductFormState>>;
  formError: string | null;
  isCreatePending: boolean;
  isUpdatePending: boolean;
  onClose: () => void;
  onSubmit: () => void;
};

export function ProductFormModal({
  mode,
  productForm,
  productDraft,
  setProductForm,
  setProductDraft,
  formError,
  isCreatePending,
  isUpdatePending,
  onClose,
  onSubmit,
}: ProductFormModalProps) {
  const activeForm = mode === "create" ? productForm : productDraft;
  const setActiveForm = mode === "create" ? setProductForm : setProductDraft;
  const isSubmitting = mode === "create" ? isCreatePending : isUpdatePending;

  const updateField = <Key extends keyof ProductFormState>(field: Key, value: ProductFormState[Key]) => {
    setActiveForm((current) => ({ ...current, [field]: value }));
  };

  return (
    <ModalShell title={mode === "create" ? "Create Product" : "Edit Product"} onClose={onClose}>
      <label style={styles.label}>Name</label>
      <input
        style={styles.input}
        value={activeForm.name}
        onChange={(event) => updateField("name", event.target.value)}
      />
      <label style={styles.label}>Description</label>
      <textarea
        style={styles.textarea}
        value={activeForm.description}
        onChange={(event) => updateField("description", event.target.value)}
      />
      <label style={styles.label}>Vision</label>
      <textarea
        style={styles.textarea}
        value={activeForm.vision}
        onChange={(event) => updateField("vision", event.target.value)}
      />
      <div style={styles.formRow}>
        <div>
          <label style={styles.label}>Goals (comma-separated)</label>
          <input
            style={styles.input}
            value={activeForm.goals}
            onChange={(event) => updateField("goals", event.target.value)}
          />
        </div>
        <div>
          <label style={styles.label}>Tags (comma-separated)</label>
          <input
            style={styles.input}
            value={activeForm.tags}
            onChange={(event) => updateField("tags", event.target.value)}
          />
        </div>
      </div>
      <div style={styles.formRow}>
        <div>
          <label style={styles.label}>Lifecycle</label>
          <select
            style={styles.select}
            value={activeForm.lifecycle}
            onChange={(event) => updateField("lifecycle", event.target.value as Product["lifecycle"])}
          >
            {productLifecycleOptions.map((option) => <option key={option} value={option}>{option}</option>)}
          </select>
        </div>
        <div>
          <label style={styles.label}>Health</label>
          <select
            style={styles.select}
            value={activeForm.health}
            onChange={(event) => updateField("health", event.target.value as Product["health"])}
          >
            {productHealthOptions.map((option) => <option key={option} value={option}>{option}</option>)}
          </select>
        </div>
      </div>
      <div style={styles.formRow}>
        <div>
          <label style={styles.label}>Owner / Hat</label>
          <input
            style={styles.input}
            value={activeForm.ownerLabel}
            onChange={(event) => updateField("ownerLabel", event.target.value)}
          />
        </div>
        <div>
          <label style={styles.label}>Investment</label>
          <select
            style={styles.select}
            value={activeForm.investmentStatus}
            onChange={(event) => updateField("investmentStatus", event.target.value as Product["investment_status"])}
          >
            {productInvestmentOptions.map((option) => <option key={option} value={option}>{option}</option>)}
          </select>
        </div>
      </div>
      <label style={styles.label}>Roadmap</label>
      <textarea
        style={styles.textarea}
        value={activeForm.roadmap}
        onChange={(event) => updateField("roadmap", event.target.value)}
      />
      <label style={styles.label}>Evidence</label>
      <textarea
        style={styles.textarea}
        value={activeForm.evidence}
        onChange={(event) => updateField("evidence", event.target.value)}
      />
      {formError && <div style={styles.errorText}>{formError}</div>}
      <div style={{ display: "flex", justifyContent: "flex-end", gap: 8 }}>
        <button style={styles.ghostBtn} onClick={onClose}>Cancel</button>
        <button style={styles.btn} onClick={onSubmit} disabled={!activeForm.name}>
          {mode === "create"
            ? isSubmitting ? "Creating..." : "Create Product"
            : isSubmitting ? "Saving..." : "Save Product"}
        </button>
      </div>
    </ModalShell>
  );
}

type DeleteProductModalProps = {
  product: Product;
  confirmName: string;
  confirmArchive: boolean;
  isReady: boolean;
  isPending: boolean;
  formError: string | null;
  onClose: () => void;
  onConfirmNameChange: (confirmName: string) => void;
  onConfirmArchiveChange: (confirmArchive: boolean) => void;
  onArchive: (productId: string) => void;
};

export function DeleteProductModal({
  product,
  confirmName,
  confirmArchive,
  isReady,
  isPending,
  formError,
  onClose,
  onConfirmNameChange,
  onConfirmArchiveChange,
  onArchive,
}: DeleteProductModalProps) {
  return (
    <ModalShell title={`Delete Product: ${product.name}`} onClose={onClose}>
      <div style={styles.contextCard}>
        <div style={styles.contextLabel}>Double Confirmation</div>
        <div style={styles.contextTitle}>This will archive the product and remove it from active product workflows.</div>
        <div style={styles.contextText}>
          The current backend exposes archive as the supported product removal operation. Type the product name and confirm the archive action to continue.
        </div>
      </div>
      <label style={styles.label}>Type product name</label>
      <input
        style={styles.input}
        value={confirmName}
        onChange={(event) => onConfirmNameChange(event.target.value)}
        placeholder={product.name}
      />
      <label style={styles.checkboxLabel}>
        <input
          type="checkbox"
          checked={confirmArchive}
          onChange={(event) => onConfirmArchiveChange(event.target.checked)}
        />
        I understand this product will be archived.
      </label>
      {formError && <div style={{ ...styles.errorText, marginTop: 10 }}>{formError}</div>}
      <div style={{ display: "flex", justifyContent: "flex-end", gap: 8, marginTop: 16 }}>
        <button style={styles.ghostBtn} onClick={onClose}>Cancel</button>
        <button
          style={styles.btnDanger}
          onClick={() => onArchive(product.id)}
          disabled={!isReady || isPending}
        >
          {isPending ? "Archiving..." : "Delete Product"}
        </button>
      </div>
    </ModalShell>
  );
}

type ResetProductPlanModalProps = {
  product: Product;
  confirmName: string;
  confirmTree: boolean;
  deleteDelivery: boolean;
  isReady: boolean;
  isPending: boolean;
  formError: string | null;
  onClose: () => void;
  onConfirmNameChange: (confirmName: string) => void;
  onConfirmTreeChange: (confirmTree: boolean) => void;
  onDeleteDeliveryChange: (deleteDelivery: boolean) => void;
  onReset: (data: { productId: string; deleteDelivery: boolean }) => void;
};

export function ResetProductPlanModal({
  product,
  confirmName,
  confirmTree,
  deleteDelivery,
  isReady,
  isPending,
  formError,
  onClose,
  onConfirmNameChange,
  onConfirmTreeChange,
  onDeleteDeliveryChange,
  onReset,
}: ResetProductPlanModalProps) {
  return (
    <ModalShell title={`Reset Product Plan: ${product.name}`} onClose={onClose}>
      <div style={styles.contextCard}>
        <div style={styles.contextLabel}>Double Confirm</div>
        <div style={styles.contextTitle}>This removes the current product management tree.</div>
        <div style={styles.contextText}>
          Product areas, capabilities, and features will be deleted so this product can be replanned from a clean management tree.
          Delivery stories and tasks are preserved unless you explicitly include them below.
        </div>
      </div>
      <label style={styles.label}>Type the product name to confirm</label>
      <input
        style={styles.input}
        value={confirmName}
        onChange={(event) => onConfirmNameChange(event.target.value)}
        placeholder={product.name}
      />
      <label style={styles.checkboxLabel}>
        <input
          type="checkbox"
          checked={confirmTree}
          onChange={(event) => onConfirmTreeChange(event.target.checked)}
        />
        I understand the product areas, capabilities, and features will be deleted.
      </label>
      <label style={styles.checkboxLabel}>
        <input
          type="checkbox"
          checked={deleteDelivery}
          onChange={(event) => onDeleteDeliveryChange(event.target.checked)}
        />
        Also delete existing delivery stories, tasks, and agent-work import ledger rows for this product.
      </label>
      {formError && <div style={styles.errorText}>{formError}</div>}
      <div style={{ display: "flex", justifyContent: "flex-end", gap: 8 }}>
        <button style={styles.ghostBtn} onClick={onClose}>Cancel</button>
        <button
          style={styles.btnDanger}
          onClick={() => onReset({ productId: product.id, deleteDelivery })}
          disabled={!isReady || isPending}
        >
          {isPending ? "Resetting..." : "Reset Plan"}
        </button>
      </div>
    </ModalShell>
  );
}
