import { styles } from "../lib/agentRegistryPageStyles";
import type { AgentTab } from "../lib/agentRegistryPageModel";

type Props = {
  activeTab: AgentTab;
  agentCount: number;
  teamCount: number;
  assignmentCount: number;
  skillCount: number;
  onActiveTabChange: (tab: AgentTab) => void;
};

const tabs: AgentTab[] = ["agents", "teams", "assignments", "skills", "routing"];

export function AgentRegistryHeader({
  activeTab,
  agentCount,
  teamCount,
  assignmentCount,
  skillCount,
  onActiveTabChange,
}: Props) {
  return (
    <>
      <div style={styles.headerRow}>
        <div style={styles.titleWrap}>
          <h1 style={styles.title}>Agent Management</h1>
          <div style={styles.subtitle}>
            Model the company explicitly: hire agents into specialist roles, organize them into teams, assign ownership by scope, and manage reusable skills as a real catalog.
          </div>
        </div>
      </div>

      <div style={styles.summaryRow}>
        <div style={styles.statCard}>
          <div style={styles.statLabel}>Agents</div>
          <div style={styles.statValue}>{agentCount}</div>
        </div>
        <div style={styles.statCard}>
          <div style={styles.statLabel}>Teams</div>
          <div style={styles.statValue}>{teamCount}</div>
        </div>
        <div style={styles.statCard}>
          <div style={styles.statLabel}>Assignments</div>
          <div style={styles.statValue}>{assignmentCount}</div>
        </div>
        <div style={styles.statCard}>
          <div style={styles.statLabel}>Skills</div>
          <div style={styles.statValue}>{skillCount}</div>
        </div>
      </div>

      <div style={styles.tabRow}>
        {tabs.map((tab) => (
          <button
            key={tab}
            type="button"
            style={{ ...styles.tab, ...(activeTab === tab ? styles.tabActive : {}) }}
            onClick={() => onActiveTabChange(tab)}
          >
            {tab === "agents" ? "Agents" : tab === "teams" ? "Teams" : tab === "assignments" ? "Assignments" : tab === "skills" ? "Skills" : "Routing"}
          </button>
        ))}
      </div>
    </>
  );
}
