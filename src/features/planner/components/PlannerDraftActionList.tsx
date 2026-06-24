import { summarizeAction, type PlannerAction } from "../lib/plannerPageModel";
import { styles } from "../lib/plannerPageStyles";

type PlannerDraftActionListProps = {
  actions: PlannerAction[];
};

export function PlannerDraftActionList({ actions }: PlannerDraftActionListProps) {
  return (
    <div style={styles.list}>
      {actions.map((action, index) => {
        const summary = summarizeAction(action);
        const symbolStyle = summary.tone === "add"
          ? styles.diffSymbolAdd
          : summary.tone === "update"
            ? styles.diffSymbolUpdate
            : styles.diffSymbolWarn;
        return (
          <div key={`${action.type}-${index}`} style={styles.diffRow}>
            <div style={symbolStyle}>{summary.symbol}</div>
            <div>
              <div style={styles.diffPrimary}>{summary.title}</div>
              <div style={styles.diffSecondary}>{summary.detail}</div>
            </div>
          </div>
        );
      })}
    </div>
  );
}
