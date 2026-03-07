import React from "react";
import { useEditorStore } from "../../state/editorStore";

const styles: Record<string, React.CSSProperties> = {
  container: { display: "flex", backgroundColor: "#252526", height: 35, alignItems: "center", overflow: "auto" },
  tab: { padding: "0 12px", height: "100%", display: "flex", alignItems: "center", gap: 6, fontSize: 12, cursor: "pointer", color: "#999", borderRight: "1px solid #1e1e1e", whiteSpace: "nowrap" as const },
  tabActive: { padding: "0 12px", height: "100%", display: "flex", alignItems: "center", gap: 6, fontSize: 12, cursor: "pointer", color: "#fff", backgroundColor: "#1e1e1e", borderRight: "1px solid #1e1e1e", whiteSpace: "nowrap" as const },
  close: { fontSize: 14, opacity: 0.5, cursor: "pointer", lineHeight: 1 },
};

export function TabBar() {
  const { openFiles, activeFileId, setActiveFile, closeFile } = useEditorStore();

  if (openFiles.length === 0) return null;

  return (
    <div style={styles.container}>
      {openFiles.map((f) => (
        <div key={f.id} style={f.id === activeFileId ? styles.tabActive : styles.tab} onClick={() => setActiveFile(f.id)}>
          <span>{f.isDirty ? "\u25CF " : ""}{f.name}</span>
          <span style={styles.close} onClick={(e) => { e.stopPropagation(); closeFile(f.id); }}>&times;</span>
        </div>
      ))}
    </div>
  );
}
