import type React from "react";

export const styles: Record<string, React.CSSProperties> = {
  page: { maxWidth: 700, margin: "0 auto" },
  title: { fontSize: 20, fontWeight: 600, color: "#e0e0e0", marginBottom: 24 },
  section: { marginBottom: 32 },
  sectionTitle: { fontSize: 14, fontWeight: 600, color: "#cccccc", marginBottom: 12, borderBottom: "1px solid #333", paddingBottom: 8 },
  row: { display: "flex", justifyContent: "space-between", alignItems: "center", padding: "8px 0", borderBottom: "1px solid #2a2a2a" },
  label: { fontSize: 13, color: "#e0e0e0" },
  desc: { fontSize: 11, color: "#888" },
  toggle: { width: 40, height: 22, borderRadius: 11, border: "none", cursor: "pointer", position: "relative" as const, transition: "background 0.2s" },
  settingRow: { display: "flex", justifyContent: "space-between", alignItems: "center", padding: "12px 0", borderBottom: "1px solid #2a2a2a" },
  input: { width: 300, padding: "6px 10px", backgroundColor: "#1e1e1e", border: "1px solid #444", borderRadius: 4, color: "#e0e0e0", fontSize: 13 },
  btn: { padding: "6px 16px", fontSize: 13, backgroundColor: "#0e639c", color: "#fff", border: "none", borderRadius: 4, cursor: "pointer", marginLeft: 8 },
  saved: { fontSize: 12, color: "#4ec9b0", marginLeft: 8 },
  healthCard: { backgroundColor: "#1f1f1f", border: "1px solid #333", borderRadius: 8, padding: 16, marginTop: 12 },
  healthGrid: { display: "grid", gridTemplateColumns: "repeat(2, minmax(0, 1fr))", gap: 12, marginBottom: 16 },
  healthLabel: { fontSize: 11, color: "#888", textTransform: "uppercase" as const, letterSpacing: 0.6 },
  healthValue: { fontSize: 18, fontWeight: 700, color: "#e0e0e0", marginTop: 4 },
  migrationList: { display: "flex", flexDirection: "column" as const, gap: 8, maxHeight: 220, overflowY: "auto" as const },
  migrationRow: { display: "flex", justifyContent: "space-between", gap: 12, padding: "8px 10px", backgroundColor: "#181818", border: "1px solid #2a2a2a", borderRadius: 6 },
  badge: { padding: "2px 8px", borderRadius: 999, fontSize: 11, fontWeight: 700 },
  codeBox: { fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace", fontSize: 12, color: "#d7e3ff", backgroundColor: "#171b24", border: "1px solid #2d3a52", borderRadius: 6, padding: "8px 10px" },
};
