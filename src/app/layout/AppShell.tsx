import React from "react";
import { Allotment } from "allotment";
import "allotment/dist/style.css";
import { useNavigate, useLocation } from "react-router-dom";
import { LeftSidebar } from "./LeftSidebar";
import { RightSidebar } from "./RightSidebar";
import { BottomPanel } from "./BottomPanel";
import { useUIStore } from "../../state/uiStore";

const styles: Record<string, any> = {
  container: { display: "flex", flexDirection: "column", height: "100vh", backgroundColor: "#1e1e1e", color: "#d4d4d4" },
  titleBar: { height: 28, backgroundColor: "#323233", display: "flex", alignItems: "center", paddingLeft: 80, fontSize: 12, fontWeight: 500, color: "#cccccc", userSelect: "none", WebkitAppRegion: "drag" as any },
  navBar: { height: 32, backgroundColor: "#252526", display: "flex", alignItems: "center", gap: 2, padding: "0 8px", borderBottom: "1px solid #1e1e1e" },
  navButton: { padding: "4px 10px", fontSize: 12, background: "none", border: "none", color: "#cccccc", cursor: "pointer", borderRadius: 4 },
  navButtonActive: { padding: "4px 10px", fontSize: 12, background: "#37373d", border: "none", color: "#ffffff", cursor: "pointer", borderRadius: 4 },
  content: { flex: 1, overflow: "hidden" },
};

const navItems = [
  { key: "planner", label: "Planner" },
  { key: "products", label: "Products" },
  { key: "work-items", label: "Work Items" },
  { key: "ide", label: "IDE" },
  { key: "repositories", label: "Workspaces" },
  { key: "agents", label: "Agents" },
  { key: "models", label: "Models" },
  { key: "chat", label: "Chat" },
  { key: "settings", label: "Settings" },
] as const;

export function AppShell({ children }: { children: React.ReactNode }) {
  const {
    leftSidebarVisible,
    rightSidebarVisible,
    bottomPanelVisible,
    activeView,
    setActiveView,
    toggleBottomPanel,
    toggleRightSidebar,
  } = useUIStore();
  const navigate = useNavigate();
  const location = useLocation();

  const currentView = navItems.find((item) => location.pathname.startsWith(`/${item.key}`))?.key ?? activeView;
  const supportsHierarchyRail = currentView === "products" || currentView === "work-items";
  const supportsInspectorRail = currentView !== "ide" && currentView !== "chat";
  const showLeftSidebar = leftSidebarVisible && supportsHierarchyRail;

  return (
    <div style={styles.container}>
      <div style={styles.titleBar}>AruviStudio</div>
      <div style={{ ...styles.navBar, justifyContent: "space-between" }}>
        <div style={{ display: "flex", alignItems: "center", gap: 2 }}>
          {navItems.map((item) => (
            <button
              key={item.key}
              data-testid={`nav-${item.key}`}
              style={currentView === item.key ? styles.navButtonActive : styles.navButton}
              onClick={() => {
                setActiveView(item.key);
                navigate(`/${item.key}`);
              }}
            >
              {item.label}
            </button>
          ))}
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <button style={bottomPanelVisible ? styles.navButtonActive : styles.navButton} onClick={toggleBottomPanel}>
            {bottomPanelVisible ? "Hide Console" : "Show Console"}
          </button>
          {supportsInspectorRail && (
            <button style={rightSidebarVisible ? styles.navButtonActive : styles.navButton} onClick={toggleRightSidebar}>
              {rightSidebarVisible ? "Hide Inspector" : "Show Inspector"}
            </button>
          )}
        </div>
      </div>
      <div style={styles.content}>
        <Allotment>
          {showLeftSidebar && (
            <Allotment.Pane minSize={200} preferredSize={240}>
              <LeftSidebar />
            </Allotment.Pane>
          )}
          <Allotment.Pane>
            <Allotment vertical>
              <Allotment.Pane>
                <Allotment>
                  <Allotment.Pane>
                    <div style={{ padding: 12, overflow: "auto", height: "100%" }}>
                      {children}
                    </div>
                  </Allotment.Pane>
                  {supportsInspectorRail && rightSidebarVisible && (
                    <Allotment.Pane minSize={200} preferredSize={260}>
                      <RightSidebar />
                    </Allotment.Pane>
                  )}
                </Allotment>
              </Allotment.Pane>
              {bottomPanelVisible && (
                <Allotment.Pane minSize={90} preferredSize={160}>
                  <BottomPanel />
                </Allotment.Pane>
              )}
            </Allotment>
          </Allotment.Pane>
        </Allotment>
      </div>
    </div>
  );
}
