import type { DatabaseHealth } from "../../../lib/types";
import { styles } from "../lib/settingsPageStyles";

interface SettingsDatabaseSectionsProps {
  activeDbPath: string;
  dbPathOverrideInput: string;
  dbPathOverrideSaved: string | null;
  dbPathOverrideError: string | null;
  dbHealth: DatabaseHealth | null;
  dbHealthError: string | null;
  onDbPathOverrideInputChange: (value: string) => void;
  onSaveDbOverride: () => void;
  onClearDbOverride: () => void;
}

export function SettingsDatabaseSections({
  activeDbPath,
  dbPathOverrideInput,
  dbPathOverrideSaved,
  dbPathOverrideError,
  dbHealth,
  dbHealthError,
  onDbPathOverrideInputChange,
  onSaveDbOverride,
  onClearDbOverride,
}: SettingsDatabaseSectionsProps) {
  return (
    <>
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
              onChange={(event) => onDbPathOverrideInputChange(event.target.value)}
              placeholder="/absolute/path/to/aruvi-live.db"
            />
            <button style={styles.btn} onClick={onSaveDbOverride}>Save</button>
            <button style={{ ...styles.btn, backgroundColor: "#3a4556" }} onClick={onClearDbOverride}>Clear</button>
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
          {!dbHealth && !dbHealthError && <div style={styles.desc}>Loading migration metadata...</div>}
        </div>
      </div>
    </>
  );
}
