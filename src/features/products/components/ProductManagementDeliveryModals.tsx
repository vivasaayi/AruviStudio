import type { Dispatch, SetStateAction } from "react";

import type { WorkItem } from "../../../lib/types";
import { getHierarchyDeleteLabel } from "../lib/productHierarchyHelpers";
import {
  workItemComplexityOptions,
  workItemPriorityOptions,
  workItemStatusOptions,
  type WorkItemDraftState,
} from "../lib/productListPageState";
import { styles } from "../lib/productListPageStyles";
import { formatWorkItemMeta } from "../lib/workItemDisplay";
import { ProductManagementModalShell as ModalShell } from "./ProductManagementModalShell";

type DeleteHierarchyCandidate = {
  kind: "product_area" | "capability" | "feature";
  id: string;
  name: string;
};

type DeleteHierarchyNodeModalProps = {
  candidate: DeleteHierarchyCandidate;
  confirmName: string;
  confirmChecked: boolean;
  isReady: boolean;
  isPending: boolean;
  formError: string | null;
  onClose: () => void;
  onConfirmNameChange: (confirmName: string) => void;
  onConfirmCheckedChange: (confirmChecked: boolean) => void;
  onDelete: (candidate: DeleteHierarchyCandidate) => void;
};

export function DeleteHierarchyNodeModal({
  candidate,
  confirmName,
  confirmChecked,
  isReady,
  isPending,
  formError,
  onClose,
  onConfirmNameChange,
  onConfirmCheckedChange,
  onDelete,
}: DeleteHierarchyNodeModalProps) {
  const deleteLabel = getHierarchyDeleteLabel(candidate.kind);

  return (
    <ModalShell title={`Delete ${deleteLabel}: ${candidate.name}`} onClose={onClose}>
      <div style={styles.contextCard}>
        <div style={styles.contextLabel}>Double Confirm</div>
        <div style={styles.contextTitle}>This deletes the selected {deleteLabel.toLowerCase()}.</div>
        <div style={styles.contextText}>
          Child hierarchy under this node will also be removed. Related delivery stories may be detached by the database if they reference this scope.
        </div>
      </div>
      <label style={styles.label}>Type the name to confirm</label>
      <input
        style={styles.input}
        value={confirmName}
        onChange={(event) => onConfirmNameChange(event.target.value)}
        placeholder={candidate.name}
      />
      <label style={styles.checkboxLabel}>
        <input
          type="checkbox"
          checked={confirmChecked}
          onChange={(event) => onConfirmCheckedChange(event.target.checked)}
        />
        I understand this hierarchy node and its child hierarchy will be deleted.
      </label>
      {formError && <div style={styles.errorText}>{formError}</div>}
      <div style={{ display: "flex", justifyContent: "flex-end", gap: 8 }}>
        <button style={styles.ghostBtn} onClick={onClose}>Cancel</button>
        <button
          style={styles.btnDanger}
          onClick={() => onDelete(candidate)}
          disabled={!isReady || isPending}
        >
          {isPending ? "Deleting..." : `Delete ${deleteLabel}`}
        </button>
      </div>
    </ModalShell>
  );
}

type DeleteManagementWorkItemCandidate = {
  workItem: WorkItem;
  kind: "story" | "task";
};

type DeleteManagementWorkItemModalProps = {
  candidate: DeleteManagementWorkItemCandidate;
  confirmName: string;
  confirmChecked: boolean;
  isReady: boolean;
  isPending: boolean;
  formError: string | null;
  onClose: () => void;
  onConfirmNameChange: (confirmName: string) => void;
  onConfirmCheckedChange: (confirmChecked: boolean) => void;
  onDelete: (candidate: DeleteManagementWorkItemCandidate) => void;
};

export function DeleteManagementWorkItemModal({
  candidate,
  confirmName,
  confirmChecked,
  isReady,
  isPending,
  formError,
  onClose,
  onConfirmNameChange,
  onConfirmCheckedChange,
  onDelete,
}: DeleteManagementWorkItemModalProps) {
  return (
    <ModalShell title={`Delete ${candidate.kind}: ${candidate.workItem.title}`} onClose={onClose}>
      <div style={styles.contextCard}>
        <div style={styles.contextLabel}>Double Confirm</div>
        <div style={styles.contextTitle}>This deletes the selected {candidate.kind}.</div>
        <div style={styles.contextText}>
          {candidate.kind === "story"
            ? "Tasks under this story will also be deleted."
            : "This task will be removed from the selected story."}
        </div>
      </div>
      <label style={styles.label} htmlFor="delete-work-item-confirm-title">Type the title to confirm</label>
      <input
        id="delete-work-item-confirm-title"
        style={styles.input}
        value={confirmName}
        onChange={(event) => onConfirmNameChange(event.target.value)}
        placeholder={candidate.workItem.title}
      />
      <label style={styles.checkboxLabel}>
        <input
          type="checkbox"
          checked={confirmChecked}
          onChange={(event) => onConfirmCheckedChange(event.target.checked)}
        />
        I understand this story/task will be deleted.
      </label>
      {formError && <div style={styles.errorText}>{formError}</div>}
      <div style={{ display: "flex", justifyContent: "flex-end", gap: 8 }}>
        <button style={styles.ghostBtn} onClick={onClose}>Cancel</button>
        <button
          style={styles.btnDanger}
          onClick={() => onDelete(candidate)}
          disabled={!isReady || isPending}
        >
          {isPending ? "Deleting..." : `Delete ${candidate.kind}`}
        </button>
      </div>
    </ModalShell>
  );
}

type ManagementWorkItemFormModalProps = {
  kind: "story" | "task";
  mode: "create" | "edit";
  contextLabel: string;
  contextTitle: string;
  draft: WorkItemDraftState;
  setDraft: Dispatch<SetStateAction<WorkItemDraftState>>;
  canSubmit: boolean;
  isCreatePending: boolean;
  isUpdatePending: boolean;
  formError: string | null;
  onClose: () => void;
  onSubmit: () => void;
};

export function ManagementWorkItemFormModal({
  kind,
  mode,
  contextLabel,
  contextTitle,
  draft,
  setDraft,
  canSubmit,
  isCreatePending,
  isUpdatePending,
  formError,
  onClose,
  onSubmit,
}: ManagementWorkItemFormModalProps) {
  const titleKind = kind === "story" ? "Story" : "Task";
  const modalTitle = mode === "edit" ? `Edit ${titleKind}` : `Add ${titleKind}`;
  const fieldPrefix = `management-${kind}`;
  const isEdit = mode === "edit";
  const isSubmitting = isCreatePending || isUpdatePending;

  const updateField = <Key extends keyof WorkItemDraftState>(field: Key, value: WorkItemDraftState[Key]) => {
    setDraft((current) => ({ ...current, [field]: value }));
  };

  return (
    <ModalShell title={modalTitle} onClose={onClose}>
      <div style={styles.contextCard}>
        <div style={styles.contextLabel}>{contextLabel}</div>
        <div style={styles.contextTitle}>{contextTitle}</div>
      </div>
      <label style={styles.label} htmlFor={`${fieldPrefix}-title`}>{titleKind} title</label>
      <input
        id={`${fieldPrefix}-title`}
        style={styles.input}
        value={draft.title}
        onChange={(event) => updateField("title", event.target.value)}
      />
      <div style={styles.formRow}>
        <div>
          <label style={styles.label} htmlFor={`${fieldPrefix}-status`}>Status</label>
          <select
            id={`${fieldPrefix}-status`}
            style={styles.input}
            value={draft.status}
            onChange={(event) => updateField("status", event.target.value as WorkItem["status"])}
          >
            {workItemStatusOptions.map((status) => <option key={status} value={status}>{formatWorkItemMeta(status)}</option>)}
          </select>
        </div>
        <div>
          <label style={styles.label} htmlFor={`${fieldPrefix}-priority`}>Priority</label>
          <select
            id={`${fieldPrefix}-priority`}
            style={styles.input}
            value={draft.priority}
            onChange={(event) => updateField("priority", event.target.value as WorkItem["priority"])}
            disabled={isEdit}
          >
            {workItemPriorityOptions.map((priority) => <option key={priority} value={priority}>{priority}</option>)}
          </select>
        </div>
      </div>
      <label style={styles.label} htmlFor={`${fieldPrefix}-problem`}>Problem Statement</label>
      <textarea
        id={`${fieldPrefix}-problem`}
        style={styles.textarea}
        value={draft.problemStatement}
        onChange={(event) => updateField("problemStatement", event.target.value)}
      />
      <label style={styles.label} htmlFor={`${fieldPrefix}-description`}>Description</label>
      <textarea
        id={`${fieldPrefix}-description`}
        style={styles.textarea}
        value={draft.description}
        onChange={(event) => updateField("description", event.target.value)}
      />
      <label style={styles.label} htmlFor={`${fieldPrefix}-acceptance-criteria`}>Acceptance Criteria</label>
      <textarea
        id={`${fieldPrefix}-acceptance-criteria`}
        style={styles.textarea}
        value={draft.acceptanceCriteria}
        onChange={(event) => updateField("acceptanceCriteria", event.target.value)}
      />
      <div style={styles.formRow}>
        <div>
          <label style={styles.label} htmlFor={`${fieldPrefix}-constraints`}>Constraints</label>
          <textarea
            id={`${fieldPrefix}-constraints`}
            style={styles.textarea}
            value={draft.constraints}
            onChange={(event) => updateField("constraints", event.target.value)}
          />
        </div>
        <div>
          <label style={styles.label} htmlFor={`${fieldPrefix}-complexity`}>Complexity</label>
          <select
            id={`${fieldPrefix}-complexity`}
            style={styles.input}
            value={draft.complexity}
            onChange={(event) => updateField("complexity", event.target.value as WorkItem["complexity"])}
            disabled={isEdit}
          >
            {workItemComplexityOptions.map((complexity) => <option key={complexity} value={complexity}>{formatWorkItemMeta(complexity)}</option>)}
          </select>
          {isEdit && <div style={styles.contextText}>Priority and complexity are currently set when the {kind} is created.</div>}
        </div>
      </div>
      {formError && <div style={styles.errorText}>{formError}</div>}
      <div style={{ display: "flex", justifyContent: "flex-end", gap: 8 }}>
        <button style={styles.ghostBtn} onClick={onClose}>Cancel</button>
        <button
          style={styles.btn}
          onClick={onSubmit}
          disabled={!canSubmit || !draft.title.trim() || isSubmitting}
        >
          {isSubmitting ? "Saving..." : isEdit ? `Save ${titleKind}` : `Add ${titleKind}`}
        </button>
      </div>
    </ModalShell>
  );
}
