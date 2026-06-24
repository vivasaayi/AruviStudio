import type { PlannerTraceEvent } from "../../../lib/types";
import { styles } from "../lib/plannerPageStyles";

export function PlannerTraceView({ events }: { events: PlannerTraceEvent[] }) {
  return (
    <div style={styles.draftWorkspaceMain}>
      <div style={styles.draftCanvas}>
        <div style={styles.draftCanvasHeader}>
          <div>
            <div style={styles.draftCanvasTitle}>Latest Planner Turn Trace</div>
            <div style={styles.helper}>
              Inspect the raw planning flow: input context, model completions, tool calls, parsed plan, and any backend validation failure.
            </div>
          </div>
          <div style={styles.chipRow}>
            <div style={styles.chip}>{events.length} events</div>
          </div>
        </div>
        {events.length > 0 ? (
          <div style={styles.list}>
            {events.map((event) => (
              <div key={`${event.step}-${event.title}`} style={styles.listItem}>
                <div style={styles.listItemTitle}>
                  {event.step}. {event.title}
                </div>
                <div style={styles.helper}>{event.stage}</div>
                <div style={{ ...styles.listItemMeta, marginTop: 8 }}>{event.detail}</div>
              </div>
            ))}
          </div>
        ) : (
          <div style={styles.emptyState}>
            No trace captured yet. Send a planner turn, then open this view to inspect the latest model/tool trace.
          </div>
        )}
      </div>
    </div>
  );
}
