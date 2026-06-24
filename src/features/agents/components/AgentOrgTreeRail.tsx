import type { AgentDefinition, AgentTeam, AgentTeamMembership } from "../../../lib/types";
import { styles } from "../lib/agentRegistryPageStyles";

type Props = {
  agents: AgentDefinition[];
  teams: AgentTeam[];
  agentsLoading: boolean;
  teamsLoading: boolean;
  selectedAgentId: string | null;
  selectedTeamId: string | null;
  expandedTeams: Record<string, boolean>;
  teamMembershipsByTeam: Map<string, AgentTeamMembership[]>;
  unassignedAgents: AgentDefinition[];
  onHireAgent: () => void;
  onBindAllAgentsToDeepSeek: () => void;
  onBindCodingAgentsToDeepSeek: () => void;
  onExpandedTeamsChange: (updater: (current: Record<string, boolean>) => Record<string, boolean>) => void;
  onSelectTeam: (teamId: string) => void;
  onEditTeam: (teamId: string) => void;
  onSelectAgent: (teamId: string | null, agentId: string) => void;
  onUnassignMembership: (teamId: string, membershipId: string) => void;
};

export function AgentOrgTreeRail({
  agents,
  teams,
  agentsLoading,
  teamsLoading,
  selectedAgentId,
  selectedTeamId,
  expandedTeams,
  teamMembershipsByTeam,
  unassignedAgents,
  onHireAgent,
  onBindAllAgentsToDeepSeek,
  onBindCodingAgentsToDeepSeek,
  onExpandedTeamsChange,
  onSelectTeam,
  onEditTeam,
  onSelectAgent,
  onUnassignMembership,
}: Props) {
  return (
    <div style={styles.rail}>
      <div style={styles.toolbar}>
        <button type="button" style={styles.buttonPrimary} onClick={onHireAgent}>
          + Hire Agent
        </button>
        <button type="button" style={styles.buttonSecondary} onClick={onBindAllAgentsToDeepSeek}>
          Use DeepSeek for All Enabled Agents
        </button>
        <button type="button" style={styles.buttonSecondary} onClick={onBindCodingAgentsToDeepSeek}>
          Use DeepSeek for Coding Agents
        </button>
      </div>
      <div style={styles.sectionTitle}>Org Tree</div>
      {agentsLoading || teamsLoading ? (
        <div style={styles.empty}>Loading organization...</div>
      ) : (
        <div style={styles.treeTable}>
          <div style={{ ...styles.treeHeader, gridTemplateColumns: "minmax(0, 1.25fr) 100px 130px 110px 180px" }}>
            <div>Name</div>
            <div>Type</div>
            <div>Role / Lead</div>
            <div>Status</div>
            <div style={{ textAlign: "right" }}>Actions</div>
          </div>
          {teams.length === 0 && unassignedAgents.length === 0 ? (
            <div style={{ ...styles.empty, padding: 14 }}>No agents hired yet.</div>
          ) : (
            <>
              {teams.map((team) => {
                const isExpanded = expandedTeams[team.id] ?? true;
                const memberRows = teamMembershipsByTeam.get(team.id) ?? [];
                return (
                  <div key={team.id}>
                    <div
                      style={{
                        ...styles.treeRow,
                        gridTemplateColumns: "minmax(0, 1.25fr) 100px 130px 110px 180px",
                        ...(team.id === selectedTeamId && !selectedAgentId ? styles.treeRowActive : {}),
                      }}
                      onClick={() => onSelectTeam(team.id)}
                    >
                      <div style={styles.treeNameCell}>
                        <span style={styles.treeCaret}>{isExpanded ? "▾" : "▸"}</span>
                        <span style={styles.treeName}>{team.name}</span>
                      </div>
                      <div style={styles.treeCell}>Team</div>
                      <div style={styles.treeCell}>
                        {memberRows.find((membership) => membership.is_lead)?.title ?? "No lead set"}
                      </div>
                      <div><span style={styles.treeMetaBadge}>{team.enabled ? "enabled" : "disabled"}</span></div>
                      <div style={styles.treeActions}>
                        <button
                          type="button"
                          style={styles.treeActionBtn}
                          onClick={(event) => {
                            event.stopPropagation();
                            onExpandedTeamsChange((current) => ({ ...current, [team.id]: !isExpanded }));
                          }}
                        >
                          {isExpanded ? "Collapse" : "Expand"}
                        </button>
                        <button
                          type="button"
                          style={styles.treeActionBtn}
                          onClick={(event) => {
                            event.stopPropagation();
                            onEditTeam(team.id);
                          }}
                        >
                          Edit
                        </button>
                      </div>
                    </div>
                    {isExpanded ? memberRows.map((membership) => {
                      const agent = agents.find((entry) => entry.id === membership.agent_id);
                      if (!agent) {
                        return null;
                      }
                      return (
                        <div
                          key={membership.id}
                          style={{
                            ...styles.treeRow,
                            gridTemplateColumns: "minmax(0, 1.25fr) 100px 130px 110px 180px",
                            ...(agent.id === selectedAgentId ? styles.treeRowActive : {}),
                          }}
                          onClick={() => onSelectAgent(team.id, agent.id)}
                        >
                          <div style={styles.treeNameCell}>
                            <span style={styles.treeIndent} />
                            <span style={styles.treeCaret}>•</span>
                            <span style={styles.treeSubName}>{agent.name}</span>
                          </div>
                          <div style={styles.treeCell}>Agent</div>
                          <div style={styles.treeCell}>
                            {agent.role}{membership.is_lead ? " (lead)" : ""}
                          </div>
                          <div><span style={styles.treeMetaBadge}>{agent.employment_status}</span></div>
                          <div style={styles.treeActions}>
                            <button
                              type="button"
                              style={styles.treeActionBtn}
                              onClick={(event) => {
                                event.stopPropagation();
                                onSelectAgent(team.id, agent.id);
                              }}
                            >
                              Edit
                            </button>
                            <button
                              type="button"
                              style={styles.treeActionBtn}
                              onClick={(event) => {
                                event.stopPropagation();
                                onUnassignMembership(team.id, membership.id);
                              }}
                            >
                              Unassign
                            </button>
                          </div>
                        </div>
                      );
                    }) : null}
                  </div>
                );
              })}
              {unassignedAgents.length > 0 ? (
                <>
                  <div style={{ ...styles.treeRow, gridTemplateColumns: "minmax(0, 1.25fr) 100px 130px 110px 180px" }}>
                    <div style={styles.treeNameCell}>
                      <span style={styles.treeCaret}>▾</span>
                      <span style={styles.treeName}>Unassigned</span>
                    </div>
                    <div style={styles.treeCell}>Group</div>
                    <div style={styles.treeCell}>No team</div>
                    <div />
                    <div />
                  </div>
                  {unassignedAgents.map((agent) => (
                    <div
                      key={agent.id}
                      style={{
                        ...styles.treeRow,
                        gridTemplateColumns: "minmax(0, 1.25fr) 100px 130px 110px 180px",
                        ...(agent.id === selectedAgentId ? styles.treeRowActive : {}),
                      }}
                      onClick={() => onSelectAgent(null, agent.id)}
                    >
                      <div style={styles.treeNameCell}>
                        <span style={styles.treeIndent} />
                        <span style={styles.treeCaret}>•</span>
                        <span style={styles.treeSubName}>{agent.name}</span>
                      </div>
                      <div style={styles.treeCell}>Agent</div>
                      <div style={styles.treeCell}>{agent.role}</div>
                      <div><span style={styles.treeMetaBadge}>{agent.employment_status}</span></div>
                      <div style={styles.treeActions}>
                        <button
                          type="button"
                          style={styles.treeActionBtn}
                          onClick={(event) => {
                            event.stopPropagation();
                            onSelectAgent(null, agent.id);
                          }}
                        >
                          Edit
                        </button>
                      </div>
                    </div>
                  ))}
                </>
              ) : null}
            </>
          )}
        </div>
      )}
    </div>
  );
}
