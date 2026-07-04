import type {
  Artifact,
  ExternalCliRun,
  ExternalCliRunEvent,
  Repository,
} from "../../../lib/types";
import {
  EXTERNAL_CLI_PROVIDERS,
  EXTERNAL_CLI_TRACE_LIMIT,
  formatDurationMs,
  formatExternalCliCommand,
  formatExternalCliTerminalEvent,
  formatInteger,
  type ExternalCliProvider,
} from "../lib/workItemListPageHelpers";
import { styles } from "../lib/workItemListPageStyles";

type WorkItemExternalCliTabProps = {
  selectedWorkItemId: string | null;
  resolvedRepository: Repository | null;
  isRunPending: boolean;
  providerInFlight: ExternalCliProvider | null;
  onRunProvider: (provider: ExternalCliProvider) => void;
  actionError: string | null;
  actionInfo: string | null;
  activeRun: ExternalCliRun | null | undefined;
  activeRunId: string | null;
  latestEvent: ExternalCliRunEvent | null;
  events: ExternalCliRunEvent[] | undefined;
  terminalOutput: string;
  runs: ExternalCliRun[] | undefined;
  artifacts: Artifact[] | undefined;
  onOpenArtifact: (artifact: Artifact) => void;
  onSelectRun: (runId: string) => void;
};

export function WorkItemExternalCliTab({
  selectedWorkItemId,
  resolvedRepository,
  isRunPending,
  providerInFlight,
  onRunProvider,
  actionError,
  actionInfo,
  activeRun,
  activeRunId,
  latestEvent,
  events,
  terminalOutput,
  runs,
  artifacts,
  onOpenArtifact,
  onSelectRun,
}: WorkItemExternalCliTabProps) {
  return (
    <>
      <div style={styles.detailTitle}>External CLI</div>
      <div style={{ display: "flex", gap: 8, marginBottom: 12, flexWrap: "wrap" }}>
        {EXTERNAL_CLI_PROVIDERS.map((entry) => (
          <button
            key={entry.provider}
            style={styles.btn}
            onClick={() => onRunProvider(entry.provider)}
            disabled={!selectedWorkItemId || !resolvedRepository || isRunPending}
            title={!resolvedRepository ? "Attach a workspace before launching an external CLI." : `Run ${entry.label}`}
          >
            {providerInFlight === entry.provider ? "Running..." : `Run ${entry.label}`}
          </button>
        ))}
      </div>
      {actionError && <div style={styles.errorText}>{actionError}</div>}
      {actionInfo && <div style={{ ...styles.smallText, color: "#4ec9b0", marginBottom: 10 }}>{actionInfo}</div>}

      <div style={styles.detailCard}>
        <div style={styles.detailLabel}>Active Run</div>
        {!resolvedRepository ? (
          <div style={styles.warning}>Attach a workspace before launching an external CLI.</div>
        ) : activeRun ? (
          <>
            <div style={styles.taskTitle}>{activeRun.label} - {activeRun.status}</div>
            <div style={styles.smallText}>Run: <code>{activeRun.id}</code></div>
            <div style={styles.smallText}>Command: {formatExternalCliCommand(activeRun)}</div>
            <div style={styles.smallText}>CWD: {activeRun.cwd}</div>
            {activeRun.session_log_path ? (
              <div style={styles.smallText}>Session file: <code>{activeRun.session_log_path}</code></div>
            ) : null}
            <div style={styles.smallText}>
              Started: {activeRun.started_at}{activeRun.ended_at ? ` - Ended: ${activeRun.ended_at}` : ""}
            </div>
            <div style={styles.smallText}>
              Exit {activeRun.exit_code ?? "n/a"} - Duration {formatDurationMs(activeRun.duration_ms)} - stdout {formatInteger(activeRun.stdout_chars)} chars - stderr {formatInteger(activeRun.stderr_chars)} chars
            </div>
            {activeRun.error_message && <div style={styles.warning}>{activeRun.error_message}</div>}
            <RunOutputArtifactButton
              artifactId={activeRun.output_artifact_id}
              artifacts={artifacts}
              onOpenArtifact={onOpenArtifact}
            />
          </>
        ) : (
          <div style={styles.detailValue}>No external CLI run has been launched for this story.</div>
        )}
      </div>

      <div style={styles.detailCard}>
        <div style={styles.detailLabel}>Latest Log</div>
        {latestEvent ? (
          <pre style={{ ...styles.terminalOutput, maxHeight: 120 }}>{formatExternalCliTerminalEvent(latestEvent)}</pre>
        ) : activeRun?.status === "running" ? (
          <div style={styles.detailValue}>Waiting for the first CLI event...</div>
        ) : (
          <div style={styles.detailValue}>No log events recorded yet.</div>
        )}
      </div>

      <div style={styles.detailCard}>
        <div style={styles.detailLabel}>Console Output</div>
        <div style={styles.smallText}>Showing latest {formatInteger(EXTERNAL_CLI_TRACE_LIMIT)} events for the selected run as a combined terminal transcript.</div>
        {events && events.length > 0 ? (
          <pre style={styles.terminalOutput}>{terminalOutput}</pre>
        ) : activeRun ? (
          <div style={styles.detailValue}>Trace events are loading...</div>
        ) : (
          <div style={styles.detailValue}>No trace is available until a CLI run starts.</div>
        )}
      </div>

      <div style={styles.detailCard}>
        <div style={styles.detailLabel}>Run History</div>
        {runs && runs.length > 0 ? (
          <div style={styles.list}>
            {runs.map((run) => {
              const outputArtifact = (artifacts ?? []).find((artifact) => artifact.id === run.output_artifact_id) ?? null;
              return (
                <div key={run.id} style={run.id === activeRunId ? { ...styles.listItem, borderColor: "#0e639c" } : styles.listItem}>
                  <div style={styles.taskTitle}>{run.label} - {run.status}</div>
                  <div style={styles.smallText}>Run: {run.id}</div>
                  <div style={styles.smallText}>Command: {formatExternalCliCommand(run)}</div>
                  <div style={styles.smallText}>CWD: {run.cwd}</div>
                  {run.session_log_path ? <div style={styles.smallText}>Session file: {run.session_log_path}</div> : null}
                  <div style={styles.smallText}>
                    Started: {run.started_at}{run.ended_at ? ` - Ended: ${run.ended_at}` : ""}
                  </div>
                  <div style={styles.smallText}>
                    Exit {run.exit_code ?? "n/a"} - Duration {formatDurationMs(run.duration_ms)} - stdout {formatInteger(run.stdout_chars)} chars - stderr {formatInteger(run.stderr_chars)} chars
                  </div>
                  {run.error_message && <div style={styles.warning}>{run.error_message}</div>}
                  <div style={{ display: "flex", gap: 8, marginTop: 8, flexWrap: "wrap" }}>
                    <button style={styles.ghostBtn} onClick={() => onSelectRun(run.id)}>
                      View trace
                    </button>
                    {outputArtifact ? (
                      <button
                        style={styles.ghostBtn}
                        onClick={() => onOpenArtifact(outputArtifact)}
                      >
                        Open captured output
                      </button>
                    ) : null}
                  </div>
                </div>
              );
            })}
          </div>
        ) : (
          <div style={styles.detailValue}>No external CLI runs yet.</div>
        )}
      </div>
    </>
  );
}

function RunOutputArtifactButton({
  artifactId,
  artifacts,
  onOpenArtifact,
}: {
  artifactId: string | null;
  artifacts: Artifact[] | undefined;
  onOpenArtifact: (artifact: Artifact) => void;
}) {
  const outputArtifact = (artifacts ?? []).find((artifact) => artifact.id === artifactId) ?? null;
  return outputArtifact ? (
    <button
      style={{ ...styles.ghostBtn, marginTop: 8 }}
      onClick={() => onOpenArtifact(outputArtifact)}
    >
      Open captured output
    </button>
  ) : (
    <div style={styles.smallText}>Captured artifact pending.</div>
  );
}
