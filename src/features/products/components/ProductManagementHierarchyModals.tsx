import type { Dispatch, SetStateAction } from "react";

import {
  getHierarchyNodeKindGuidance,
  getHierarchyNodeKindLabel,
} from "../../../lib/hierarchyLabels";
import type { Capability, HierarchyNodeKind, ProductArea } from "../../../lib/types";
import { styles } from "../lib/productListPageStyles";
import { ProductManagementModalShell as ModalShell } from "./ProductManagementModalShell";

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

export function ProductAreaFormModal({
  mode,
  selectedProductArea,
  form,
  draft,
  setForm,
  setDraft,
  formError,
  selectedProductId,
  isCreatePending,
  isUpdatePending,
  onClose,
  onSubmit,
}: {
  mode: "create" | "edit";
  selectedProductArea: ProductArea | null;
  form: ProductAreaFormState;
  draft: ProductAreaFormState;
  setForm: Dispatch<SetStateAction<ProductAreaFormState>>;
  setDraft: Dispatch<SetStateAction<ProductAreaFormState>>;
  formError: string | null;
  selectedProductId: string | null;
  isCreatePending: boolean;
  isUpdatePending: boolean;
  onClose: () => void;
  onSubmit: () => void;
}) {
  const activeForm = mode === "create" ? form : draft;
  const setActiveForm = mode === "create" ? setForm : setDraft;

  const updateField = <Key extends keyof ProductAreaFormState>(field: Key, value: ProductAreaFormState[Key]) => {
    setActiveForm((current) => ({ ...current, [field]: value }));
  };

  return (
    <ModalShell
      title={mode === "create"
        ? `Create ${getHierarchyNodeKindLabel(form.nodeKind)}`
        : `Edit ${selectedProductArea ? getHierarchyNodeKindLabel(selectedProductArea.node_kind) : "Product Area"}: ${selectedProductArea?.name ?? ""}`}
      onClose={onClose}
    >
      {mode === "create" ? (
        <>
          <label style={styles.label}>Product Area Kind</label>
          <select style={styles.input} value={form.nodeKind} onChange={(event) => updateField("nodeKind", event.target.value as HierarchyNodeKind)}>
            {(["product_area"] as HierarchyNodeKind[]).map((nodeKind) => (
              <option key={nodeKind} value={nodeKind}>{getHierarchyNodeKindLabel(nodeKind)}</option>
            ))}
          </select>
          <div style={styles.contextText}>{getHierarchyNodeKindGuidance(form.nodeKind)}</div>
          <label style={styles.label}>{getHierarchyNodeKindLabel(form.nodeKind)} Name</label>
        </>
      ) : (
        <>
          <div style={styles.contextCard}>
            <div style={styles.contextLabel}>Product Area</div>
            <div style={styles.contextText}>{getHierarchyNodeKindGuidance("product_area")}</div>
          </div>
          <label style={styles.label}>Product Area Name</label>
        </>
      )}
      <input style={styles.input} value={activeForm.name} onChange={(event) => updateField("name", event.target.value)} />
      <label style={styles.label}>Description</label>
      <textarea style={styles.textarea} value={activeForm.description} onChange={(event) => updateField("description", event.target.value)} />
      <label style={styles.label}>Purpose</label>
      <input style={styles.input} value={activeForm.purpose} onChange={(event) => updateField("purpose", event.target.value)} />
      {formError && <div style={styles.errorText}>{formError}</div>}
      <div style={{ display: "flex", justifyContent: "flex-end", gap: 8 }}>
        <button style={styles.ghostBtn} onClick={onClose}>Cancel</button>
        <button
          style={styles.btn}
          onClick={onSubmit}
          disabled={!activeForm.name || !selectedProductId}
        >
          {mode === "create"
            ? isCreatePending ? "Saving..." : `Create ${getHierarchyNodeKindLabel(form.nodeKind)}`
            : isUpdatePending ? "Saving..." : "Save Product Area"}
        </button>
      </div>
    </ModalShell>
  );
}

export function CapabilityFormModal({
  mode,
  selectedProductArea,
  selectedCapability,
  form,
  draft,
  setForm,
  setDraft,
  createKindGroups,
  editKindGroups,
  formError,
  activeProductAreaId,
  isCreatePending,
  isUpdatePending,
  onClose,
  onSubmit,
}: {
  mode: "create" | "edit";
  selectedProductArea: ProductArea | null;
  selectedCapability: Capability | null;
  form: CapabilityFormState;
  draft: CapabilityFormState;
  setForm: Dispatch<SetStateAction<CapabilityFormState>>;
  setDraft: Dispatch<SetStateAction<CapabilityFormState>>;
  createKindGroups: NodeKindGroup[];
  editKindGroups: NodeKindGroup[];
  formError: string | null;
  activeProductAreaId: string | null;
  isCreatePending: boolean;
  isUpdatePending: boolean;
  onClose: () => void;
  onSubmit: () => void;
}) {
  const activeForm = mode === "create" ? form : draft;
  const setActiveForm = mode === "create" ? setForm : setDraft;
  const kindGroups = mode === "create" ? createKindGroups : editKindGroups;

  const updateField = <Key extends keyof CapabilityFormState>(field: Key, value: CapabilityFormState[Key]) => {
    setActiveForm((current) => ({ ...current, [field]: value }));
  };

  return (
    <ModalShell
      title={mode === "create"
        ? `Create ${getHierarchyNodeKindLabel(form.nodeKind)}`
        : `Edit ${selectedCapability ? getHierarchyNodeKindLabel(selectedCapability.node_kind) : "Node"}: ${selectedCapability?.name ?? ""}`}
      onClose={onClose}
    >
      {mode === "create" ? (
        <>
          <label style={styles.label}>Parent Product Area</label>
          <input style={styles.input} value={selectedProductArea?.name ?? ""} readOnly />
          <label style={styles.label}>Parent Node</label>
          <input
            style={styles.input}
            value={selectedCapability?.name ?? ""}
            readOnly
            placeholder={`Create a top-level child under ${selectedProductArea?.name ?? "the selected product area"}`}
          />
          <label style={styles.label}>Node Kind</label>
        </>
      ) : (
        <label style={styles.label}>Node Kind</label>
      )}
      <select style={styles.input} value={activeForm.nodeKind} onChange={(event) => updateField("nodeKind", event.target.value as HierarchyNodeKind)}>
        {kindGroups.map((group) => (
          <optgroup key={group.label} label={group.label}>
            {group.kinds.map((nodeKind) => (
              <option key={nodeKind} value={nodeKind}>{getHierarchyNodeKindLabel(nodeKind)}</option>
            ))}
          </optgroup>
        ))}
      </select>
      <div style={styles.contextText}>
        {getHierarchyNodeKindGuidance(activeForm.nodeKind)}
        {mode === "create"
          ? ` Allowed here: ${createKindGroups.flatMap((group) => group.kinds).map((nodeKind) => getHierarchyNodeKindLabel(nodeKind)).join(", ")}.`
          : ""}
      </div>
      <label style={styles.label}>{mode === "create" ? `${getHierarchyNodeKindLabel(form.nodeKind)} Name` : "Name"}</label>
      <input style={styles.input} value={activeForm.name} onChange={(event) => updateField("name", event.target.value)} />
      <label style={styles.label}>Description</label>
      <textarea style={styles.textarea} value={activeForm.description} onChange={(event) => updateField("description", event.target.value)} />
      <label style={styles.label}>Acceptance Criteria</label>
      <textarea style={styles.textarea} value={activeForm.acceptanceCriteria} onChange={(event) => updateField("acceptanceCriteria", event.target.value)} />
      <label style={styles.label}>Technical Notes</label>
      <textarea style={styles.textarea} value={activeForm.technicalNotes} onChange={(event) => updateField("technicalNotes", event.target.value)} />
      {formError && <div style={styles.errorText}>{formError}</div>}
      <div style={{ display: "flex", justifyContent: "flex-end", gap: 8 }}>
        <button style={styles.ghostBtn} onClick={onClose}>Cancel</button>
        <button
          style={styles.btn}
          onClick={onSubmit}
          disabled={!activeForm.name || !activeProductAreaId}
        >
          {mode === "create"
            ? isCreatePending ? "Saving..." : `Create ${getHierarchyNodeKindLabel(form.nodeKind)}`
            : isUpdatePending ? "Saving..." : `Save ${selectedCapability ? getHierarchyNodeKindLabel(selectedCapability.node_kind) : "Node"}`}
        </button>
      </div>
    </ModalShell>
  );
}
