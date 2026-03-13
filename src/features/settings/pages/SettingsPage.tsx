import React, { useState, useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
import {
  clearDatabasePathOverride,
  getActiveDatabasePath,
  getDatabaseHealth,
  getDatabasePathOverride,
  getSetting,
  seedExampleProducts,
  setDatabasePathOverride,
  setSetting,
} from "../../../lib/tauri";
import type { DatabaseHealth } from "../../../lib/types";
import { useUIStore } from "../../../state/uiStore";

const AUTO_START_AFTER_APPROVAL_KEY = "workflow.auto_start_after_work_item_approval";
const AUTO_APPROVE_PLAN_KEY = "workflow.auto_approve_plan";
const AUTO_APPROVE_TEST_REVIEW_KEY = "workflow.auto_approve_test_review";
const HIDE_EXAMPLE_PRODUCTS_KEY = "catalog.hide_example_products";

function parseBooleanSetting(value: string | null | undefined, fallback: boolean) {
  if (value == null) return fallback;
  switch (value.trim().toLowerCase()) {
    case "1":
    case "true":
    case "yes":
    case "on":
      return true;
    case "0":
    case "false":
    case "no":
    case "off":
      return false;
    default:
      return fallback;
  }
}

const styles: Record<string, React.CSSProperties> = {
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
};

export function SettingsPage() {
  const queryClient = useQueryClient();
  const { leftSidebarVisible, rightSidebarVisible, bottomPanelVisible, toggleLeftSidebar, toggleRightSidebar, toggleBottomPanel } = useUIStore();
  const [dockerHost, setDockerHost] = useState("");
  const [maxRetries, setMaxRetries] = useState("3");
  const [autoStartAfterApproval, setAutoStartAfterApproval] = useState(true);
  const [autoApprovePlan, setAutoApprovePlan] = useState(true);
  const [autoApproveTestReview, setAutoApproveTestReview] = useState(true);
  const [hideExampleProducts, setHideExampleProducts] = useState(true);
  const [savedMsg, setSavedMsg] = useState<string | null>(null);
  const [dbHealth, setDbHealth] = useState<DatabaseHealth | null>(null);
  const [dbHealthError, setDbHealthError] = useState<string | null>(null);
  const [activeDbPath, setActiveDbPath] = useState("");
  const [dbPathOverrideInput, setDbPathOverrideInput] = useState("");
  const [dbPathOverrideSaved, setDbPathOverrideSaved] = useState<string | null>(null);
  const [dbPathOverrideError, setDbPathOverrideError] = useState<string | null>(null);
  const [catalogActionMsg, setCatalogActionMsg] = useState<string | null>(null);
  const [catalogActionError, setCatalogActionError] = useState<string | null>(null);

  useEffect(() => {
    getSetting("docker_host").then((v) => { if (v) setDockerHost(v); });
    getSetting("max_workflow_retries").then((v) => { if (v) setMaxRetries(v); });
    getSetting(AUTO_START_AFTER_APPROVAL_KEY).then((v) => setAutoStartAfterApproval(parseBooleanSetting(v, true)));
    getSetting(AUTO_APPROVE_PLAN_KEY).then((v) => setAutoApprovePlan(parseBooleanSetting(v, true)));
    getSetting(AUTO_APPROVE_TEST_REVIEW_KEY).then((v) => setAutoApproveTestReview(parseBooleanSetting(v, true)));
    getSetting(HIDE_EXAMPLE_PRODUCTS_KEY).then((v) => setHideExampleProducts(parseBooleanSetting(v, true)));
    getActiveDatabasePath().then(setActiveDbPath).catch((error) => setDbPathOverrideError(String(error)));
    getDatabasePathOverride().then((v) => { if (v) setDbPathOverrideInput(v); });
    getDatabaseHealth()
      .then((health) => {
        setDbHealth(health);
        setDbHealthError(null);
      })
      .catch((error) => {
        setDbHealthError(String(error));
      });
  }, []);

  const saveSetting = async (key: string, value: string) => {
    await setSetting(key, value);
    if (key === HIDE_EXAMPLE_PRODUCTS_KEY) {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["products"] }),
        queryClient.invalidateQueries({ queryKey: ["productTree"] }),
        queryClient.invalidateQueries({ queryKey: ["sidebarProductTree"] }),
        queryClient.invalidateQueries({ queryKey: ["inspectorProductTree"] }),
      ]);
    }
    setSavedMsg(key);
    setTimeout(() => setSavedMsg(null), 2000);
  };

  const saveDbOverride = async () => {
    try {
      setDbPathOverrideError(null);
      await setDatabasePathOverride(dbPathOverrideInput);
      setDbPathOverrideSaved("saved");
      setTimeout(() => setDbPathOverrideSaved(null), 2500);
    } catch (error) {
      setDbPathOverrideError(String(error));
    }
  };

  const clearDbOverride = async () => {
    try {
      setDbPathOverrideError(null);
      await clearDatabasePathOverride();
      setDbPathOverrideInput("");
      setDbPathOverrideSaved("cleared");
      setTimeout(() => setDbPathOverrideSaved(null), 2500);
    } catch (error) {
      setDbPathOverrideError(String(error));
    }
  };

  return (
    <div style={styles.page}>
      <h1 style={styles.title}>Settings</h1>
      <div style={styles.section}>
        <div style={styles.sectionTitle}>Layout</div>
        <div style={styles.row}><div><div style={styles.label}>Left Sidebar</div><div style={styles.desc}>Product tree and navigation</div></div><button style={{ ...styles.toggle, backgroundColor: leftSidebarVisible ? "#0e639c" : "#444" }} onClick={toggleLeftSidebar} /></div>
        <div style={styles.row}><div><div style={styles.label}>Right Sidebar</div><div style={styles.desc}>Context panel for work item details</div></div><button style={{ ...styles.toggle, backgroundColor: rightSidebarVisible ? "#0e639c" : "#444" }} onClick={toggleRightSidebar} /></div>
        <div style={styles.row}><div><div style={styles.label}>Bottom Panel</div><div style={styles.desc}>Terminal, logs, and test results</div></div><button style={{ ...styles.toggle, backgroundColor: bottomPanelVisible ? "#0e639c" : "#444" }} onClick={toggleBottomPanel} /></div>
      </div>
      <div style={styles.section}>
        <div style={styles.sectionTitle}>Catalog</div>
        <div style={styles.row}>
          <div>
            <div style={styles.label}>Hide Example Products</div>
            <div style={styles.desc}>Seeded example products stay in the database but remain hidden from the workspace by default.</div>
          </div>
          <button
            style={{ ...styles.toggle, backgroundColor: hideExampleProducts ? "#0e639c" : "#444" }}
            onClick={async () => {
              const next = !hideExampleProducts;
              setHideExampleProducts(next);
              await saveSetting(HIDE_EXAMPLE_PRODUCTS_KEY, String(next));
            }}
          />
        </div>
        <div style={styles.settingRow}>
          <div>
            <div style={styles.label}>Seed Example Products</div>
            <div style={styles.desc}>Create or repair the built-in example catalog in the currently active database.</div>
          </div>
          <div style={{ display: "flex", alignItems: "center" }}>
            <button
              style={styles.btn}
              onClick={async () => {
                try {
                  setCatalogActionError(null);
                  await seedExampleProducts();
                  await Promise.all([
                    queryClient.invalidateQueries({ queryKey: ["products"] }),
                    queryClient.invalidateQueries({ queryKey: ["productTree"] }),
                    queryClient.invalidateQueries({ queryKey: ["sidebarProductTree"] }),
                    queryClient.invalidateQueries({ queryKey: ["inspectorProductTree"] }),
                    queryClient.invalidateQueries({ queryKey: ["workItems"] }),
                  ]);
                  setCatalogActionMsg("Example catalog seeded.");
                  setTimeout(() => setCatalogActionMsg(null), 2500);
                } catch (error) {
                  setCatalogActionError(String(error));
                }
              }}
            >
              Seed Now
            </button>
          </div>
        </div>
        {catalogActionMsg && <div style={styles.saved}>{catalogActionMsg}</div>}
        {catalogActionError && <div style={{ ...styles.desc, color: "#f48771", marginTop: 8 }}>{catalogActionError}</div>}
      </div>
      <div style={styles.section}>
        <div style={styles.sectionTitle}>Execution</div>
        <div style={styles.settingRow}>
          <div><div style={styles.label}>Docker Host</div><div style={styles.desc}>Docker daemon URL for test execution</div></div>
          <div style={{ display: "flex", alignItems: "center" }}><input style={styles.input} value={dockerHost} onChange={(e) => setDockerHost(e.target.value)} placeholder="unix:///var/run/docker.sock" /><button style={styles.btn} onClick={() => saveSetting("docker_host", dockerHost)}>Save</button>{savedMsg === "docker_host" && <span style={styles.saved}>Saved!</span>}</div>
        </div>
        <div style={styles.settingRow}>
          <div><div style={styles.label}>Max Workflow Retries</div><div style={styles.desc}>Maximum retry attempts for failed workflow stages</div></div>
          <div style={{ display: "flex", alignItems: "center" }}><input style={{ ...styles.input, width: 80 }} type="number" value={maxRetries} onChange={(e) => setMaxRetries(e.target.value)} /><button style={styles.btn} onClick={() => saveSetting("max_workflow_retries", maxRetries)}>Save</button>{savedMsg === "max_workflow_retries" && <span style={styles.saved}>Saved!</span>}</div>
        </div>
      </div>
      <div style={styles.section}>
        <div style={styles.sectionTitle}>Workflow Automation</div>
        <div style={styles.row}>
          <div>
            <div style={styles.label}>Auto-start after work item approval</div>
            <div style={styles.desc}>When a work item is approved, queue its workflow immediately in the background. Default: on.</div>
          </div>
          <button
            style={{ ...styles.toggle, backgroundColor: autoStartAfterApproval ? "#0e639c" : "#444" }}
            onClick={async () => {
              const next = !autoStartAfterApproval;
              setAutoStartAfterApproval(next);
              await saveSetting(AUTO_START_AFTER_APPROVAL_KEY, String(next));
            }}
          />
        </div>
        <div style={styles.row}>
          <div>
            <div style={styles.label}>Auto-approve planning</div>
            <div style={styles.desc}>After planning completes, record plan approval automatically and continue into coding. Default: on.</div>
          </div>
          <button
            style={{ ...styles.toggle, backgroundColor: autoApprovePlan ? "#0e639c" : "#444" }}
            onClick={async () => {
              const next = !autoApprovePlan;
              setAutoApprovePlan(next);
              await saveSetting(AUTO_APPROVE_PLAN_KEY, String(next));
            }}
          />
        </div>
        <div style={styles.row}>
          <div>
            <div style={styles.label}>Auto-approve test review</div>
            <div style={styles.desc}>After validation, security, and performance stages complete, record test review approval automatically and continue into push preparation. Default: on.</div>
          </div>
          <button
            style={{ ...styles.toggle, backgroundColor: autoApproveTestReview ? "#0e639c" : "#444" }}
            onClick={async () => {
              const next = !autoApproveTestReview;
              setAutoApproveTestReview(next);
              await saveSetting(AUTO_APPROVE_TEST_REVIEW_KEY, String(next));
            }}
          />
        </div>
      </div>
      <div style={styles.section}>
        <div style={styles.sectionTitle}>Database Source</div>
        <div style={styles.settingRow}>
          <div>
            <div style={styles.label}>Active Database</div>
            <div style={styles.desc}>{activeDbPath || "Unknown"}</div>
          </div>
        </div>
        <div style={styles.settingRow}>
          <div>
            <div style={styles.label}>Override Database Path</div>
            <div style={styles.desc}>Set an absolute SQLite path for next app launch. Restart required.</div>
          </div>
          <div style={{ display: "flex", alignItems: "center" }}>
            <input
              style={{ ...styles.input, width: 380 }}
              value={dbPathOverrideInput}
              onChange={(e) => setDbPathOverrideInput(e.target.value)}
              placeholder="/absolute/path/to/aruvi-live.db"
            />
            <button style={styles.btn} onClick={saveDbOverride}>Save</button>
            <button style={{ ...styles.btn, backgroundColor: "#3a4556" }} onClick={clearDbOverride}>Clear</button>
          </div>
        </div>
        {dbPathOverrideSaved === "saved" && <div style={styles.saved}>DB override saved. Restart AruviStudio to apply.</div>}
        {dbPathOverrideSaved === "cleared" && <div style={styles.saved}>DB override cleared. Restart AruviStudio to use default DB.</div>}
        {dbPathOverrideError && <div style={{ ...styles.desc, color: "#f48771", marginTop: 8 }}>{dbPathOverrideError}</div>}
      </div>
      <div style={styles.section}>
        <div style={styles.sectionTitle}>Database Health</div>
        <div style={styles.healthCard}>
          {dbHealthError && <div style={{ ...styles.desc, color: "#f48771" }}>{dbHealthError}</div>}
          {dbHealth && (
            <>
              <div style={styles.healthGrid}>
                <div>
                  <div style={styles.healthLabel}>Applied Migrations</div>
                  <div style={styles.healthValue}>{dbHealth.applied_migrations}</div>
                </div>
                <div>
                  <div style={styles.healthLabel}>Latest Version</div>
                  <div style={styles.healthValue}>{dbHealth.latest_version ?? "N/A"}</div>
                </div>
              </div>
              <div style={styles.migrationList}>
                {dbHealth.migrations.map((migration) => (
                  <div key={migration.version} style={styles.migrationRow}>
                    <div>
                      <div style={styles.label}>v{migration.version} · {migration.description}</div>
                      <div style={styles.desc}>Installed {migration.installed_on}</div>
                    </div>
                    <span
                      style={{
                        ...styles.badge,
                        color: migration.success ? "#4ec9b0" : "#f48771",
                        backgroundColor: migration.success ? "rgba(78, 201, 176, 0.12)" : "rgba(244, 135, 113, 0.12)",
                      }}
                    >
                      {migration.success ? "Applied" : "Failed"}
                    </span>
                  </div>
                ))}
              </div>
            </>
          )}
          {!dbHealth && !dbHealthError && <div style={styles.desc}>Loading migration metadata…</div>}
        </div>
      </div>
    </div>
  );
}
