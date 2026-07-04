import type { Artifact } from "../../../lib/types";
import { getArtifactFileName } from "../lib/workItemListPageHelpers";
import { styles } from "../lib/workItemListPageStyles";
import { WorkItemModalShell as ModalShell } from "./WorkItemModalShell";

type ArtifactTraceStep = {
  step: number;
  events: Array<{ kind: string; payload: string }>;
};

function buildArtifactTraceSteps(artifact: Artifact, content?: string | null): ArtifactTraceStep[] | null {
  if (!content) {
    return null;
  }

  const fileName = getArtifactFileName(artifact).toLowerCase();
  const isTraceArtifact = artifact.artifact_type === "coding_tool_trace" || fileName === "tool_trace.json";
  if (!isTraceArtifact) {
    return null;
  }

  try {
    const parsed = JSON.parse(content) as Array<{ step?: number; kind?: string; payload?: string }>;
    if (!Array.isArray(parsed)) {
      return null;
    }

    const grouped = new Map<number, Array<{ kind: string; payload: string }>>();
    parsed.forEach((entry) => {
      if (!entry || typeof entry !== "object") {
        return;
      }
      const step = typeof entry.step === "number" ? entry.step : 0;
      const kind = typeof entry.kind === "string" ? entry.kind : "unknown";
      const payload = typeof entry.payload === "string" ? entry.payload : JSON.stringify(entry.payload ?? "");
      const current = grouped.get(step) ?? [];
      current.push({ kind, payload });
      grouped.set(step, current);
    });

    return Array.from(grouped.entries())
      .sort((a, b) => a[0] - b[0])
      .map(([step, events]) => ({ step, events }));
  } catch {
    return null;
  }
}

export function WorkItemArtifactModal({
  artifact,
  content,
  onClose,
}: {
  artifact: Artifact;
  content?: string | null;
  onClose: () => void;
}) {
  const traceSteps = buildArtifactTraceSteps(artifact, content);

  return (
    <ModalShell title={`Artifact: ${getArtifactFileName(artifact)}`} onClose={onClose}>
      <div style={styles.detailCard}>
        <div style={styles.detailLabel}>Type</div>
        <div style={styles.detailValue}>{artifact.artifact_type}</div>
        <div style={{ ...styles.detailLabel, marginTop: 10 }}>Path</div>
        <div style={styles.smallText}>{artifact.storage_path}</div>
        <div style={{ ...styles.detailLabel, marginTop: 10 }}>Summary</div>
        <div style={styles.smallText}>{artifact.summary}</div>
      </div>
      {traceSteps ? (
        <div>
          <div style={{ ...styles.detailLabel, marginBottom: 8 }}>
            Tool Trace Timeline ({traceSteps.length} steps)
          </div>
          {traceSteps.map((stepGroup) => (
            <div key={stepGroup.step} style={styles.traceStepCard}>
              <div style={styles.traceStepHeader}>
                <div style={styles.traceStepTitle}>Step {stepGroup.step}</div>
                <div style={styles.traceStepMeta}>{stepGroup.events.length} events</div>
              </div>
              <div style={styles.traceEventList}>
                {stepGroup.events.map((event, index) => {
                  const eventStyle = event.kind.includes("error")
                    ? styles.traceEventCardError
                    : styles.traceEventCard;
                  return (
                    <div key={`${stepGroup.step}-${event.kind}-${index}`} style={eventStyle}>
                      <div style={styles.traceEventKind}>{event.kind.replace(/_/g, " ")}</div>
                      <pre style={styles.traceEventPayload}>{event.payload}</pre>
                    </div>
                  );
                })}
              </div>
            </div>
          ))}
        </div>
      ) : (
        <div style={styles.previewBox}>
          {(content ?? "").trim() || "Artifact content is empty."}
        </div>
      )}
    </ModalShell>
  );
}
