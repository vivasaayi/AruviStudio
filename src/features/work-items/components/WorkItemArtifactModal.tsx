import { useState } from "react";
import type { Artifact } from "../../../lib/types";
import { readArtifactPage, type ArtifactPage } from "../../../lib/tauri/artifacts";
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
  const [page, setPage] = useState<ArtifactPage | null>(null);
  const [offsets, setOffsets] = useState<number[]>([]);
  const [pageError, setPageError] = useState<string | null>(null);
  const [loadingPage, setLoadingPage] = useState(false);
  const traceSteps = page ? null : buildArtifactTraceSteps(artifact, content);
  const loadPage = async (offset: number, version: string | null, history: number[]) => {
    setLoadingPage(true); setPageError(null);
    try {
      const next = await readArtifactPage(artifact.id, offset, version);
      setPage(next); setOffsets(history);
    } catch (error) { setPageError(String(error)); }
    finally { setLoadingPage(false); }
  };

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
      <div>
        <button type="button" disabled={loadingPage} onClick={() => void loadPage(0, null, [])}>Read in pages</button>
        {page && <>
          <button type="button" disabled={loadingPage || offsets.length === 0} onClick={() => void loadPage(offsets[offsets.length - 1], page.version, offsets.slice(0, -1))}>Previous page</button>
          <button type="button" disabled={loadingPage || page.next_offset === null} onClick={() => void loadPage(page.next_offset!, page.version, [...offsets, page.offset])}>Next page</button>
          <span> Bytes {page.offset}–{page.next_offset ?? page.total_bytes} of {page.total_bytes}</span>
        </>}
        {pageError && <p role="alert">{pageError}</p>}
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
          {page ? page.content : content == null ? "Content is loading or unavailable. Use Read in pages for larger artifacts." : content.trim() || "Artifact content is empty."}
        </div>
      )}
    </ModalShell>
  );
}
