import React from "react";
import { useUIStore } from "../../state/uiStore";

const styles: Record<string, React.CSSProperties> = {
  container: { height: "100%", backgroundColor: "#1e1e1e", borderTop: "1px solid #333", display: "flex", flexDirection: "column" },
  tabs: { display: "flex", gap: 0, backgroundColor: "#252526" },
  tab: { padding: "4px 16px", fontSize: 12, cursor: "pointer", border: "none", background: "none", color: "#888" },
  tabActive: { padding: "4px 16px", fontSize: 12, cursor: "pointer", border: "none", background: "#1e1e1e", color: "#cccccc", borderTop: "1px solid #007acc" },
  content: { flex: 1, padding: 8, overflow: "auto", fontFamily: "monospace", fontSize: 12, color: "#cccccc" },
};

export function BottomPanel() {
  const { bottomPanelTab, setBottomPanelTab, toggleBottomPanel } = useUIStore();

  return (
    <div style={styles.container}>
      <div style={styles.tabs}>
        {(["terminal", "logs", "tests"] as const).map((tab) => (
          <button
            key={tab}
            style={bottomPanelTab === tab ? styles.tabActive : styles.tab}
            onClick={() => setBottomPanelTab(tab)}
          >
            {tab.charAt(0).toUpperCase() + tab.slice(1)}
          </button>
        ))}
        <div style={{ flex: 1 }} />
        <button style={styles.tab} onClick={toggleBottomPanel}>Close</button>
      </div>
      <div style={styles.content}>
        {bottomPanelTab === "terminal" && <div>Terminal output will appear here</div>}
        {bottomPanelTab === "logs" && <div>Logs will appear here</div>}
        {bottomPanelTab === "tests" && <div>Test results will appear here</div>}
      </div>
    </div>
  );
}
