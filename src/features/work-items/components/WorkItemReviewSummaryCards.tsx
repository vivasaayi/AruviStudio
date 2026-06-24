import type { Approval, Artifact, Finding, WorkItem } from "../../../lib/types";
import { styles } from "../lib/workItemListPageStyles";

type Props = {
  subWorkItems: WorkItem[] | undefined;
  approvals: Approval[] | undefined;
  artifacts: Artifact[] | undefined;
  findings: Finding[] | undefined;
  latestApproval: Approval | null;
  latestArtifact: Artifact | null;
  topArtifactTypes: Array<[string, number]>;
  findingSeverityCounts: Map<string, number>;
};

export function WorkItemReviewSummaryCards({
  subWorkItems,
  approvals,
  artifacts,
  findings,
  latestApproval,
  latestArtifact,
  topArtifactTypes,
  findingSeverityCounts,
}: Props) {
  return (
    <>
      <div style={styles.detailCard}>
        <div style={styles.detailLabel}>Tasks</div>
        {subWorkItems && subWorkItems.length > 0 ? (
          <div style={styles.list}>
            {subWorkItems.map((workItem) => (
              <div key={workItem.id} style={styles.listItem}>
                <div style={styles.taskTitle}>{workItem.title}</div>
                <div style={styles.smallText}>{workItem.status.replace(/_/g, " ")} · {workItem.work_item_type}</div>
              </div>
            ))}
          </div>
        ) : (
          <div style={styles.detailValue}>No tasks yet.</div>
        )}
      </div>
      <div style={styles.detailCard}>
        <div style={styles.detailLabel}>Summary</div>
        <div style={styles.list}>
          <div style={styles.listItem}>
            <div style={styles.taskTitle}>Approvals</div>
            <div style={styles.smallText}>{approvals?.length ?? 0} records</div>
            {latestApproval && (
              <div style={styles.smallText}>
                Latest: {latestApproval.approval_type} · {latestApproval.status} · {latestApproval.created_at}
              </div>
            )}
          </div>
          <div style={styles.listItem}>
            <div style={styles.taskTitle}>Artifacts</div>
            <div style={styles.smallText}>{artifacts?.length ?? 0} generated</div>
            {latestArtifact && (
              <div style={styles.smallText}>
                Latest: {latestArtifact.artifact_type} · {latestArtifact.created_at}
              </div>
            )}
            {topArtifactTypes.length > 0 && (
              <div style={styles.smallText}>
                Top types: {topArtifactTypes.map(([kind, count]) => `${kind} (${count})`).join(", ")}
              </div>
            )}
          </div>
          <div style={styles.listItem}>
            <div style={styles.taskTitle}>Findings</div>
            <div style={styles.smallText}>{findings?.length ?? 0} logged</div>
            {(findings?.length ?? 0) > 0 && (
              <div style={styles.smallText}>
                Severity: {["critical", "high", "medium", "low", "info"]
                  .filter((severity) => (findingSeverityCounts.get(severity) ?? 0) > 0)
                  .map((severity) => `${severity} (${findingSeverityCounts.get(severity)})`)
                  .join(", ")}
              </div>
            )}
          </div>
        </div>
      </div>
    </>
  );
}
