import type React from "react";

import type { Repository, WorkItem } from "../../../lib/types";
import { styles } from "../lib/workItemListPageStyles";

type Props = {
  resolvedRepository: Repository | null;
  selectedWorkItem: WorkItem | null | undefined;
  workspaceAssignmentPanel: React.ReactNode;
  isCreateWorkspacePending: boolean;
  onCreateWorkspace: () => void;
};

export function WorkItemWorkspaceReadinessCard({
  resolvedRepository,
  selectedWorkItem,
  workspaceAssignmentPanel,
  isCreateWorkspacePending,
  onCreateWorkspace,
}: Props) {
  return (
    <div style={styles.detailCard}>
      <div style={styles.detailLabel}>Workspace Readiness</div>
      {resolvedRepository ? (
        <>
          <div style={styles.detailValue}>{resolvedRepository.name}</div>
          <div style={styles.smallText}>{resolvedRepository.local_path}</div>
          <div style={styles.smallText}>
            {resolvedRepository.remote_url
              ? `Remote configured: ${resolvedRepository.remote_url}`
              : "Remote: not configured"}
          </div>
          <div style={styles.smallText}>Branch: {selectedWorkItem?.branch_name || resolvedRepository.default_branch}</div>
          <div style={styles.smallText}>
            Source: {selectedWorkItem?.repo_override_id ? "story override" : "scope default"}
          </div>
          <div style={styles.smallText}>Version history: enabled</div>
          {workspaceAssignmentPanel}
        </>
      ) : (
        <>
          <div style={styles.warning}>
            No workspace is attached to the current story scope.
          </div>
          <div style={styles.smallText}>
            Delivery stages will be blocked until a workspace exists for this scope.
          </div>
          <div style={{ marginTop: 10 }}>
            <button
              style={styles.btn}
              onClick={onCreateWorkspace}
              disabled={isCreateWorkspacePending}
            >
              {isCreateWorkspacePending ? "Creating Workspace..." : "Create Workspace"}
            </button>
          </div>
          {workspaceAssignmentPanel}
        </>
      )}
    </div>
  );
}
