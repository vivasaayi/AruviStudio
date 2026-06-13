import React from "react";
import { useNavigate, useLocation } from "react-router-dom";
import { useUIStore } from "../../state/uiStore";

const styles: Record<string, any> = {
  container: { display: "flex", flexDirection: "column", height: "100vh", backgroundColor: "#1e1e1e", color: "#d4d4d4" },
  navBar: { height: 32, backgroundColor: "#252526", display: "flex", alignItems: "center", gap: 2, padding: "0 8px", borderBottom: "1px solid #1e1e1e" },
  navButton: { padding: "4px 10px", fontSize: 12, background: "none", border: "none", color: "#cccccc", cursor: "pointer", borderRadius: 4 },
  navButtonActive: { padding: "4px 10px", fontSize: 12, background: "#37373d", border: "none", color: "#ffffff", cursor: "pointer", borderRadius: 4 },
  content: { flex: 1, overflow: "hidden" },
};

const navItems = [
  { key: "portfolio", label: "Portfolio" },
  { key: "planner", label: "Planner" },
  { key: "product-overview", label: "Product Overview" },
  { key: "products", label: "Products" },
  { key: "work-items", label: "Delivery / Builder" },
  { key: "ide", label: "IDE" },
  { key: "repositories", label: "Workspaces" },
  { key: "agents", label: "Agents" },
  { key: "models", label: "Models" },
  { key: "chat", label: "Chat" },
  { key: "voice-chat", label: "Voice Chat" },
  { key: "calls", label: "Calls" },
  { key: "settings", label: "Settings" },
] as const;

export function AppShell({ children }: { children: React.ReactNode }) {
  const {
    activeView,
    setActiveView,
  } = useUIStore();
  const navigate = useNavigate();
  const location = useLocation();

  const currentView = navItems.find((item) => location.pathname.startsWith(`/${item.key}`))?.key ?? activeView;

  return (
    <div style={styles.container}>
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
      </div>
      <div style={styles.content}>
        <div style={{ padding: 12, overflow: "auto", height: "100%" }}>
          {children}
        </div>
      </div>
    </div>
  );
}
