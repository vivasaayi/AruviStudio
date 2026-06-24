import type { Repository } from "../../../lib/types";
import type { WorkspaceBranchMode } from "../lib/workItemListPageHelpers";
import { styles } from "../lib/workItemListPageStyles";

export function WorkItemWorkspaceAssignmentPanel({
  isEditing,
  repositories,
  workspaceRepositoryId,
  workspaceBranchMode,
  workspaceBranchName,
  currentBranch,
  branchPreview,
  hasWorkspaceOverride,
  isAssignPending,
  isClearPending,
  onOpenEditor,
  onClearOverride,
  onRepositoryIdChange,
  onBranchModeChange,
  onBranchNameChange,
  onSave,
  onCancel,
}: {
  isEditing: boolean;
  repositories: Repository[];
  workspaceRepositoryId: string;
  workspaceBranchMode: WorkspaceBranchMode;
  workspaceBranchName: string;
  currentBranch: string;
  branchPreview: string;
  hasWorkspaceOverride: boolean;
  isAssignPending: boolean;
  isClearPending: boolean;
  onOpenEditor: () => void;
  onClearOverride: () => void;
  onRepositoryIdChange: (repositoryId: string) => void;
  onBranchModeChange: (branchMode: WorkspaceBranchMode) => void;
  onBranchNameChange: (branchName: string) => void;
  onSave: () => void;
  onCancel: () => void;
}) {
  return (
    <div style={{ marginTop: 12 }}>
      {!isEditing ? (
        <div style={styles.taskActions}>
          <button style={styles.ghostBtn} onClick={onOpenEditor}>
            Change Workspace
          </button>
          {hasWorkspaceOverride && (
            <button
              style={styles.ghostBtn}
              onClick={onClearOverride}
              disabled={isClearPending}
            >
              {isClearPending ? "Clearing..." : "Use Scope Default"}
            </button>
          )}
        </div>
      ) : (
        <div style={styles.infoCard}>
          <div style={styles.detailLabel}>Workspace</div>
          <select
            style={styles.filterSelect}
            value={workspaceRepositoryId}
            onChange={(event) => onRepositoryIdChange(event.target.value)}
          >
            <option value="">Select workspace</option>
            {repositories.map((repository) => (
              <option key={repository.id} value={repository.id}>
                {repository.name} - {repository.local_path}
              </option>
            ))}
          </select>

          <div style={styles.detailLabel}>Branch Option</div>
          <select
            style={styles.filterSelect}
            value={workspaceBranchMode}
            onChange={(event) => onBranchModeChange(event.target.value as WorkspaceBranchMode)}
          >
            <option value="default">Use workspace default branch</option>
            <option value="work_item">Create/use story branch</option>
            <option value="custom">Use custom branch</option>
          </select>

          {workspaceBranchMode === "custom" && (
            <input
              style={styles.input}
              value={workspaceBranchName}
              onChange={(event) => onBranchNameChange(event.target.value)}
              placeholder="feature/my-work-item"
            />
          )}

          <div style={styles.smallText}>Current branch: {currentBranch}</div>
          <div style={styles.smallText}>Next branch: {branchPreview || "Select a workspace or branch option"}</div>
          <div style={{ ...styles.taskActions, justifyContent: "flex-start", marginTop: 10 }}>
            <button
              style={styles.btn}
              onClick={onSave}
              disabled={isAssignPending || !workspaceRepositoryId || !branchPreview}
            >
              {isAssignPending ? "Saving..." : "Save Workspace"}
            </button>
            <button style={styles.ghostBtn} onClick={onCancel}>
              Cancel
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
