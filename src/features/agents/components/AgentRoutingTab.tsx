import type React from "react";

import type { WorkflowStagePolicy } from "../../../lib/types";
import type { RoutingDraft } from "../lib/agentRegistryPageModel";
import { workflowStageOptions } from "../lib/agentRegistryPageModel";
import { styles } from "../lib/agentRegistryPageStyles";

type AgentRoutingTabProps = {
  routingPolicies: WorkflowStagePolicy[];
  selectedPolicyStage: string;
  selectedPolicy: WorkflowStagePolicy | null;
  routingDraft: RoutingDraft;
  onSelectedPolicyStageChange: (stageName: string) => void;
  onRoutingDraftChange: React.Dispatch<React.SetStateAction<RoutingDraft>>;
  onDeleteRoutingPolicy: (stageName: string) => void;
  onSaveRoutingPolicy: () => void;
  onResetRoutingForm: () => void;
  routingError: string | null;
  routingFeedback: string | null;
};

export function AgentRoutingTab({
  routingPolicies,
  selectedPolicyStage,
  selectedPolicy,
  routingDraft,
  onSelectedPolicyStageChange,
  onRoutingDraftChange,
  onDeleteRoutingPolicy,
  onSaveRoutingPolicy,
  onResetRoutingForm,
  routingError,
  routingFeedback,
}: AgentRoutingTabProps) {
  return (
    <div style={styles.workspace}>
      <div style={styles.rail}>
        <div style={styles.sectionTitle}>Workflow Stages</div>
        <div style={styles.list}>
          {workflowStageOptions.map((stageName) => {
            const policy = routingPolicies.find((entry) => entry.stage_name === stageName);
            return (
              <button
                key={stageName}
                type="button"
                style={{
                  ...styles.listItem,
                  ...(selectedPolicyStage === stageName ? styles.listItemActive : {}),
                  textAlign: "left",
                }}
                onClick={() => onSelectedPolicyStageChange(stageName)}
              >
                <div style={styles.itemTitle}>{stageName}</div>
                <div style={styles.itemMeta}>
                  {policy ? `${policy.primary_roles.length} primary / ${policy.fallback_roles.length} fallback` : "using defaults"}
                </div>
                <div style={styles.badgeRow}>
                  <span style={styles.badgeMuted}>
                    {policy?.coordinator_required ?? true ? "coordinator on" : "coordinator off"}
                  </span>
                </div>
              </button>
            );
          })}
        </div>
      </div>
      <div style={styles.detail}>
        <div style={styles.headerRow}>
          <div style={styles.titleWrap}>
            <h2 style={styles.title}>Routing Policy Editor</h2>
            <div style={styles.subtitle}>Map each work item delivery stage to preferred and fallback roles, and control whether coordinator review is required before the specialist runs.</div>
          </div>
          {selectedPolicy ? (
            <button type="button" style={styles.buttonDanger} onClick={() => onDeleteRoutingPolicy(selectedPolicy.stage_name)}>
              Reset To Default
            </button>
          ) : null}
        </div>
        <div style={styles.formGrid}>
          <div style={styles.field}>
            <label style={styles.label}>Stage</label>
            <select
              style={styles.select}
              value={routingDraft.stageName}
              onChange={(event) => {
                onSelectedPolicyStageChange(event.target.value);
                onRoutingDraftChange((draft) => ({ ...draft, stageName: event.target.value }));
              }}
            >
              {workflowStageOptions.map((stageName) => (
                <option key={stageName} value={stageName}>
                  {stageName}
                </option>
              ))}
            </select>
          </div>
          <div style={styles.field}>
            <label style={styles.label}>Coordinator Review</label>
            <select
              style={styles.select}
              value={routingDraft.coordinatorRequired ? "required" : "skipped"}
              onChange={(event) => onRoutingDraftChange((draft) => ({ ...draft, coordinatorRequired: event.target.value === "required" }))}
            >
              <option value="required">required</option>
              <option value="skipped">skipped</option>
            </select>
          </div>
          <div style={{ ...styles.field, ...styles.fullWidth }}>
            <label style={styles.label}>Primary Roles (comma-separated)</label>
            <input
              style={styles.input}
              value={routingDraft.primaryRoles}
              onChange={(event) => onRoutingDraftChange((draft) => ({ ...draft, primaryRoles: event.target.value }))}
              placeholder="developer, architect, manager"
            />
          </div>
          <div style={{ ...styles.field, ...styles.fullWidth }}>
            <label style={styles.label}>Fallback Roles (comma-separated)</label>
            <input
              style={styles.input}
              value={routingDraft.fallbackRoles}
              onChange={(event) => onRoutingDraftChange((draft) => ({ ...draft, fallbackRoles: event.target.value }))}
              placeholder="coding, planning"
            />
          </div>
        </div>
        {routingError ? <div style={styles.error}>{routingError}</div> : null}
        {routingFeedback ? <div style={styles.success}>{routingFeedback}</div> : null}
        <div style={styles.toolbar}>
          <button type="button" style={styles.buttonPrimary} onClick={onSaveRoutingPolicy}>
            Save Policy
          </button>
          <button
            type="button"
            style={styles.buttonSecondary}
            onClick={onResetRoutingForm}
          >
            Reset Form
          </button>
        </div>
      </div>
    </div>
  );
}
