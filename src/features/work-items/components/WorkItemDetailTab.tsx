import type React from "react";

import type { ExternalCliRun, Repository, WorkItem, WorkflowRun } from "../../../lib/types";
import {
  describeWorkItemRuntime,
  EXTERNAL_CLI_PROVIDERS,
  getWorkItemExecutionSteps,
  type ExternalCliProvider,
} from "../lib/workItemListPageHelpers";
import { styles } from "../lib/workItemListPageStyles";

type WorkflowReadiness = {
  blockers: string[];
  warnings: string[];
  checks: string[];
};

type WorkItemDetailTabProps = {
  workItem: WorkItem | null;
  workflowReadiness: WorkflowReadiness;
  isWorkflowPending: boolean;
  onStartWorkflow: () => void;
  onApprove: () => void;
  onReject: () => void;
  workflowRunId: string | null;
  isRestartWorkflowPending: boolean;
  onRestartWorkflow: () => void;
  onEditStory: () => void;
  onCreateTask: () => void;
  resolvedRepository: Repository | null;
  isExternalCliPending: boolean;
  externalCliProviderInFlight: ExternalCliProvider | null;
  onRunExternalCli: (provider: ExternalCliProvider) => void;
  latestExternalCliRun: ExternalCliRun | null;
  onOpenExternalCliRun: (runId: string) => void;
  actionError: string | null;
  actionInfo: string | null;
  workspaceAssignmentPanel: React.ReactNode;
  isCreateWorkspacePending: boolean;
  onCreateWorkspace: () => void;
  latestWorkflowRun: WorkflowRun | null | undefined;
};

export function WorkItemDetailTab({
  workItem,
  workflowReadiness,
  isWorkflowPending,
  onStartWorkflow,
  onApprove,
  onReject,
  workflowRunId,
  isRestartWorkflowPending,
  onRestartWorkflow,
  onEditStory,
  onCreateTask,
  resolvedRepository,
  isExternalCliPending,
  externalCliProviderInFlight,
  onRunExternalCli,
  latestExternalCliRun,
  onOpenExternalCliRun,
  actionError,
  actionInfo,
  workspaceAssignmentPanel,
  isCreateWorkspacePending,
  onCreateWorkspace,
  latestWorkflowRun,
}: WorkItemDetailTabProps) {
  if (!workItem) {
    return <div style={styles.empty}>Select a story from the queue to refine it.</div>;
  }

  return (
    <>
      <div style={styles.detailTitle}>{workItem.title}</div>
      <div style={{ display: "flex", gap: 8, marginBottom: 12, flexWrap: "wrap" }}>
        <button
          style={styles.btn}
          disabled={workflowReadiness.blockers.length > 0 || isWorkflowPending}
          onClick={onStartWorkflow}
        >
          {isWorkflowPending ? "Starting..." : "Start Workflow"}
        </button>
        <button style={{ ...styles.btn, backgroundColor: "#2d6a3f" }} onClick={onApprove}>Approve</button>
        <button style={styles.btnDanger} onClick={onReject}>Reject</button>
        {workflowRunId && (
          <button
            style={styles.btn}
            onClick={onRestartWorkflow}
            disabled={isRestartWorkflowPending}
          >
            {isRestartWorkflowPending ? "Restarting..." : "Restart Workflow"}
          </button>
        )}
        <button style={styles.ghostBtn} onClick={onEditStory}>
          Edit Story
        </button>
        <button style={styles.ghostBtn} onClick={onCreateTask}>
          + New Task
        </button>
        {EXTERNAL_CLI_PROVIDERS.map((entry) => (
          <button
            key={entry.provider}
            style={styles.ghostBtn}
            onClick={() => onRunExternalCli(entry.provider)}
            disabled={!resolvedRepository || isExternalCliPending}
            title={!resolvedRepository ? "Attach a workspace before launching an external CLI." : `Run ${entry.label}`}
          >
            {externalCliProviderInFlight === entry.provider ? "Running..." : `Run ${entry.label}`}
          </button>
        ))}
        {latestExternalCliRun ? (
          <button
            style={styles.ghostBtn}
            onClick={() => onOpenExternalCliRun(latestExternalCliRun.id)}
          >
            CLI: {latestExternalCliRun.status}
          </button>
        ) : null}
      </div>
      {actionError && <div style={styles.errorText}>{actionError}</div>}
      {actionInfo && <div style={{ ...styles.smallText, color: "#4ec9b0", marginBottom: 10 }}>{actionInfo}</div>}

      <WorkflowReadinessCard workflowReadiness={workflowReadiness} />
      <WorkspaceReadinessCard
        workItem={workItem}
        resolvedRepository={resolvedRepository}
        workspaceAssignmentPanel={workspaceAssignmentPanel}
        isCreateWorkspacePending={isCreateWorkspacePending}
        onCreateWorkspace={onCreateWorkspace}
      />
      <WorkItemDetailCards
        workItem={workItem}
        resolvedRepository={resolvedRepository}
        latestWorkflowRun={latestWorkflowRun}
      />
    </>
  );
}

function WorkflowReadinessCard({ workflowReadiness }: { workflowReadiness: WorkflowReadiness }) {
  return (
    <div style={styles.readinessCard}>
      <div style={styles.readinessHeading}>Workflow Readiness Check</div>
      {workflowReadiness.blockers.length === 0 && workflowReadiness.warnings.length === 0 ? (
        <div style={{ ...styles.readinessItem, ...styles.readinessOk }}>Ready to start.</div>
      ) : null}
      {workflowReadiness.blockers.map((item) => (
        <div key={`blocker-${item}`} style={styles.readinessItem}>
          <span style={styles.readinessBlocker}>Blocker:</span> {item}
        </div>
      ))}
      {workflowReadiness.warnings.map((item) => (
        <div key={`warn-${item}`} style={styles.readinessItem}>
          <span style={styles.readinessWarn}>Warning:</span> {item}
        </div>
      ))}
      {workflowReadiness.checks.map((item) => (
        <div key={`ok-${item}`} style={styles.readinessItem}>
          <span style={styles.readinessOk}>OK:</span> {item}
        </div>
      ))}
    </div>
  );
}

function WorkspaceReadinessCard({
  workItem,
  resolvedRepository,
  workspaceAssignmentPanel,
  isCreateWorkspacePending,
  onCreateWorkspace,
}: {
  workItem: WorkItem;
  resolvedRepository: Repository | null;
  workspaceAssignmentPanel: React.ReactNode;
  isCreateWorkspacePending: boolean;
  onCreateWorkspace: () => void;
}) {
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
          <div style={styles.smallText}>Branch: {workItem.branch_name || resolvedRepository.default_branch}</div>
          <div style={styles.smallText}>
            Source: {workItem.repo_override_id ? "story override" : "scope default"}
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
            Create the workspace here and AruviStudio will enable version history and attach it automatically.
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

function WorkItemDetailCards({
  workItem,
  resolvedRepository,
  latestWorkflowRun,
}: {
  workItem: WorkItem;
  resolvedRepository: Repository | null;
  latestWorkflowRun: WorkflowRun | null | undefined;
}) {
  return (
    <>
      <div style={styles.detailCard}>
        <div style={styles.detailLabel}>Description</div>
        <div style={styles.detailValue}>{workItem.description || "No description yet."}</div>
      </div>
      <div style={styles.detailCard}>
        <div style={styles.detailLabel}>Execution Steps</div>
        <div style={styles.list}>
          {getWorkItemExecutionSteps(workItem, resolvedRepository?.name ?? null).map((step, index) => (
            <div key={`${workItem.id}-step-${index}`} style={styles.listItem}>
              <div style={styles.detailValue}>{index + 1}. {step}</div>
            </div>
          ))}
        </div>
      </div>
      <div style={styles.row}>
        <div style={styles.detailCard}><div style={styles.detailLabel}>Story Status</div><div style={styles.detailValue}>{workItem.status.replace(/_/g, " ")}</div></div>
        <div style={styles.detailCard}><div style={styles.detailLabel}>Workflow Status</div><div style={styles.detailValue}>{describeWorkItemRuntime(workItem, latestWorkflowRun ?? null).detail}</div></div>
      </div>
      <div style={styles.row}>
        <div style={styles.detailCard}><div style={styles.detailLabel}>Priority</div><div style={styles.detailValue}>{workItem.priority}</div></div>
        <div style={styles.detailCard}><div style={styles.detailLabel}>Type</div><div style={styles.detailValue}>{workItem.work_item_type}</div></div>
      </div>
      <div style={styles.detailCard}><div style={styles.detailLabel}>Complexity</div><div style={styles.detailValue}>{workItem.complexity}</div></div>
      {workItem.problem_statement && <div style={styles.detailCard}><div style={styles.detailLabel}>Problem Statement</div><div style={styles.detailValue}>{workItem.problem_statement}</div></div>}
      {workItem.acceptance_criteria && <div style={styles.detailCard}><div style={styles.detailLabel}>Acceptance Criteria</div><div style={styles.detailValue}>{workItem.acceptance_criteria}</div></div>}
      {workItem.constraints && <div style={styles.detailCard}><div style={styles.detailLabel}>Constraints</div><div style={styles.detailValue}>{workItem.constraints}</div></div>}
    </>
  );
}
