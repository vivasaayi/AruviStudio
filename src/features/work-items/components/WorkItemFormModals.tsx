import type { Dispatch, SetStateAction } from "react";

import type { WorkItem } from "../../../lib/types";
import { statusColors } from "../lib/workItemListPageHelpers";
import { styles } from "../lib/workItemListPageStyles";
import { WorkItemModalShell as ModalShell } from "./WorkItemModalShell";

export type WorkItemCreateFormState = {
  title: string;
  problemStatement: string;
  description: string;
  acceptanceCriteria: string;
  constraints: string;
  workItemType: string;
  priority: WorkItem["priority"];
  complexity: WorkItem["complexity"];
  parentWorkItemId: string | null;
};

export type WorkItemEditDraftState = {
  title: string;
  description: string;
  status: WorkItem["status"];
  problemStatement: string;
  acceptanceCriteria: string;
  constraints: string;
};

const workItemTypeOptions = ["feature", "bug", "refactor", "test", "review", "security_fix", "performance_improvement"];
const priorityOptions: WorkItem["priority"][] = ["critical", "high", "medium", "low"];

type WorkItemCreateModalProps = {
  createForm: WorkItemCreateFormState;
  setCreateForm: Dispatch<SetStateAction<WorkItemCreateFormState>>;
  creationScopeLabel: string;
  hasActiveProduct: boolean;
  formError: string | null;
  isPending: boolean;
  onClose: () => void;
  onSubmit: () => void;
};

export function WorkItemCreateModal({
  createForm,
  setCreateForm,
  creationScopeLabel,
  hasActiveProduct,
  formError,
  isPending,
  onClose,
  onSubmit,
}: WorkItemCreateModalProps) {
  const updateField = <Key extends keyof WorkItemCreateFormState>(field: Key, value: WorkItemCreateFormState[Key]) => {
    setCreateForm((current) => ({ ...current, [field]: value }));
  };

  return (
    <ModalShell title={createForm.parentWorkItemId ? "Create Task" : "Create Story"} onClose={onClose}>
      <div style={styles.detailCard}>
        <div style={styles.detailLabel}>Creation Scope</div>
        <div style={styles.detailValue}>{creationScopeLabel}</div>
      </div>
      <label style={styles.detailLabel}>Title</label>
      <input
        style={styles.input}
        value={createForm.title}
        onChange={(event) => updateField("title", event.target.value)}
      />
      <label style={styles.detailLabel}>Problem Statement</label>
      <textarea
        style={styles.textarea}
        value={createForm.problemStatement}
        onChange={(event) => updateField("problemStatement", event.target.value)}
      />
      <label style={styles.detailLabel}>Description</label>
      <textarea
        style={styles.textarea}
        value={createForm.description}
        onChange={(event) => updateField("description", event.target.value)}
      />
      <label style={styles.detailLabel}>Acceptance Criteria</label>
      <textarea
        style={styles.textarea}
        value={createForm.acceptanceCriteria}
        onChange={(event) => updateField("acceptanceCriteria", event.target.value)}
      />
      <label style={styles.detailLabel}>Constraints</label>
      <textarea
        style={styles.textarea}
        value={createForm.constraints}
        onChange={(event) => updateField("constraints", event.target.value)}
      />
      <div style={styles.row}>
        <div>
          <label style={styles.detailLabel}>Type</label>
          <select
            style={styles.filterSelect}
            value={createForm.workItemType}
            onChange={(event) => updateField("workItemType", event.target.value)}
          >
            {workItemTypeOptions.map((type) => <option key={type} value={type}>{type}</option>)}
          </select>
        </div>
        <div>
          <label style={styles.detailLabel}>Priority</label>
          <select
            style={styles.filterSelect}
            value={createForm.priority}
            onChange={(event) => updateField("priority", event.target.value as WorkItem["priority"])}
          >
            {priorityOptions.map((priority) => <option key={priority} value={priority}>{priority}</option>)}
          </select>
        </div>
      </div>
      {!hasActiveProduct && <div style={styles.warning}>Select a product before creating a story.</div>}
      {formError && <div style={styles.errorText}>{formError}</div>}
      <div style={{ display: "flex", justifyContent: "flex-end", gap: 8 }}>
        <button style={styles.ghostBtn} onClick={onClose}>Cancel</button>
        <button style={styles.btn} onClick={onSubmit} disabled={!hasActiveProduct || !createForm.title}>
          {isPending ? "Creating..." : createForm.parentWorkItemId ? "Create Task" : "Create Story"}
        </button>
      </div>
    </ModalShell>
  );
}

type WorkItemEditModalProps = {
  draft: WorkItemEditDraftState;
  setDraft: Dispatch<SetStateAction<WorkItemEditDraftState>>;
  formError: string | null;
  isPending: boolean;
  onClose: () => void;
  onSubmit: () => void;
};

export function WorkItemEditModal({
  draft,
  setDraft,
  formError,
  isPending,
  onClose,
  onSubmit,
}: WorkItemEditModalProps) {
  const updateField = <Key extends keyof WorkItemEditDraftState>(field: Key, value: WorkItemEditDraftState[Key]) => {
    setDraft((current) => ({ ...current, [field]: value }));
  };

  return (
    <ModalShell title="Edit Story" onClose={onClose}>
      {formError && <div style={styles.errorText}>{formError}</div>}
      <label style={styles.detailLabel}>Title</label>
      <input
        style={styles.input}
        value={draft.title}
        onChange={(event) => updateField("title", event.target.value)}
      />
      <label style={styles.detailLabel}>Description</label>
      <textarea
        style={styles.textarea}
        value={draft.description}
        onChange={(event) => updateField("description", event.target.value)}
      />
      <label style={styles.detailLabel}>Problem Statement</label>
      <textarea
        style={styles.textarea}
        value={draft.problemStatement}
        onChange={(event) => updateField("problemStatement", event.target.value)}
      />
      <label style={styles.detailLabel}>Acceptance Criteria</label>
      <textarea
        style={styles.textarea}
        value={draft.acceptanceCriteria}
        onChange={(event) => updateField("acceptanceCriteria", event.target.value)}
      />
      <label style={styles.detailLabel}>Constraints</label>
      <textarea
        style={styles.textarea}
        value={draft.constraints}
        onChange={(event) => updateField("constraints", event.target.value)}
      />
      <label style={styles.detailLabel}>Status</label>
      <select
        style={styles.filterSelect}
        value={draft.status}
        onChange={(event) => updateField("status", event.target.value as WorkItem["status"])}
      >
        {Object.keys(statusColors).map((status) => (
          <option key={status} value={status}>{status.replace(/_/g, " ")}</option>
        ))}
      </select>
      <div style={{ display: "flex", justifyContent: "flex-end", gap: 8 }}>
        <button style={styles.ghostBtn} onClick={onClose}>Cancel</button>
        <button style={styles.btn} onClick={onSubmit} disabled={!draft.title}>
          {isPending ? "Saving..." : "Save Story"}
        </button>
      </div>
    </ModalShell>
  );
}
