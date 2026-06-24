import type { Dispatch, SetStateAction } from "react";

import type { AgentDefinition, AgentTeam, AgentTeamMembership, Skill, TeamAssignment } from "../../../lib/types";
import { styles } from "../lib/agentRegistryPageStyles";
import type { TeamDraft } from "../lib/agentRegistryPageModel";
import { AgentSkillChooser } from "./AgentSkillChooser";

type MembershipDraft = {
  agentId: string;
  title: string;
  isLead: boolean;
};

type AgentTeamsTabProps = {
  agents: AgentDefinition[];
  teams: AgentTeam[];
  skills: Skill[];
  teamsLoading: boolean;
  memberships: AgentTeamMembership[];
  selectedTeamId: string | null;
  selectedTeam: AgentTeam | null;
  teamDraft: TeamDraft;
  selectedTeamSkillIds: string[];
  selectedTeamMemberships: AgentTeamMembership[];
  selectedTeamAssignments: TeamAssignment[];
  membershipDraft: MembershipDraft;
  membershipError: string | null;
  teamError: string | null;
  teamFeedback: string | null;
  isCreateTeamPending: boolean;
  isUpdateTeamPending: boolean;
  onCreateNewTeam: () => void;
  onSelectTeam: (teamId: string) => void;
  onTeamDraftChange: Dispatch<SetStateAction<TeamDraft>>;
  onSelectedTeamSkillIdsChange: Dispatch<SetStateAction<string[]>>;
  onMembershipDraftChange: Dispatch<SetStateAction<MembershipDraft>>;
  onSaveTeam: () => void;
  onResetTeamForm: () => void;
  onDeleteTeam: (teamId: string) => void;
  onRemoveMembership: (membershipId: string) => void;
  onAddMembership: () => void;
};

export function AgentTeamsTab({
  agents,
  teams,
  skills,
  teamsLoading,
  memberships,
  selectedTeamId,
  selectedTeam,
  teamDraft,
  selectedTeamSkillIds,
  selectedTeamMemberships,
  selectedTeamAssignments,
  membershipDraft,
  membershipError,
  teamError,
  teamFeedback,
  isCreateTeamPending,
  isUpdateTeamPending,
  onCreateNewTeam,
  onSelectTeam,
  onTeamDraftChange,
  onSelectedTeamSkillIdsChange,
  onMembershipDraftChange,
  onSaveTeam,
  onResetTeamForm,
  onDeleteTeam,
  onRemoveMembership,
  onAddMembership,
}: AgentTeamsTabProps) {
  const capacity = selectedTeam ? selectedTeam.max_concurrent_workflows : teamDraft.maxConcurrentWorkflows;

  return (
    <div style={styles.workspace}>
      <div style={styles.rail}>
        <div style={styles.toolbar}>
          <button type="button" style={styles.buttonPrimary} onClick={onCreateNewTeam}>
            + New Team
          </button>
        </div>
        <div style={styles.sectionTitle}>Teams</div>
        <div style={styles.list}>
          {teamsLoading ? (
            <div style={styles.empty}>Loading teams...</div>
          ) : teams.length === 0 ? (
            <div style={styles.empty}>No teams created yet.</div>
          ) : (
            teams.map((team) => {
              const memberCount = memberships.filter((membership) => membership.team_id === team.id).length;
              return (
                <button
                  key={team.id}
                  type="button"
                  style={{
                    ...styles.listItem,
                    ...(team.id === selectedTeamId ? styles.listItemActive : {}),
                    textAlign: "left",
                  }}
                  onClick={() => onSelectTeam(team.id)}
                >
                  <div style={styles.itemTitle}>{team.name}</div>
                  <div style={styles.itemMeta}>
                    {team.department} · {memberCount} member{memberCount === 1 ? "" : "s"}
                  </div>
                  <div style={styles.badgeRow}>
                    <span style={styles.badgeMuted}>{team.enabled ? "enabled" : "disabled"}</span>
                  </div>
                </button>
              );
            })
          )}
        </div>
      </div>
      <div style={styles.detail}>
        <div style={styles.headerRow}>
          <div style={styles.titleWrap}>
            <h2 style={styles.title}>{selectedTeam ? "Edit Team" : "Create Team"}</h2>
            <div style={styles.subtitle}>Set team profile, save, then manage members. Keep this page focused on one team at a time.</div>
          </div>
          <div style={styles.headerActions}>
            <button
              type="button"
              style={{
                ...styles.buttonPrimary,
                opacity: !teamDraft.name.trim() ? 0.55 : 1,
                cursor: !teamDraft.name.trim() ? "not-allowed" : "pointer",
              }}
              disabled={!teamDraft.name.trim() || isCreateTeamPending || isUpdateTeamPending}
              onClick={onSaveTeam}
            >
              {selectedTeam ? "Save Team" : "Create Team"}
            </button>
            <button type="button" style={styles.buttonSecondary} onClick={onResetTeamForm}>
              Reset
            </button>
            {selectedTeam ? (
              <button type="button" style={styles.buttonDanger} onClick={() => onDeleteTeam(selectedTeam.id)}>
                Delete
              </button>
            ) : null}
          </div>
        </div>

        <div style={styles.teamStatsRow}>
          <div style={styles.teamStatChip}>
            <div style={styles.sectionTitle}>Roster</div>
            <div style={styles.infoValue}>{selectedTeamMemberships.length} member{selectedTeamMemberships.length === 1 ? "" : "s"}</div>
          </div>
          <div style={styles.teamStatChip}>
            <div style={styles.sectionTitle}>Assignments</div>
            <div style={styles.infoValue}>{selectedTeamAssignments.length} scope assignment{selectedTeamAssignments.length === 1 ? "" : "s"}</div>
          </div>
          <div style={styles.teamStatChip}>
            <div style={styles.sectionTitle}>Skills</div>
            <div style={styles.infoValue}>{selectedTeamSkillIds.length} linked skill{selectedTeamSkillIds.length === 1 ? "" : "s"}</div>
          </div>
          <div style={styles.teamStatChip}>
            <div style={styles.sectionTitle}>Capacity</div>
            <div style={styles.infoValue}>
              {capacity} concurrent workflow{capacity === 1 ? "" : "s"}
            </div>
          </div>
        </div>

        <div style={styles.teamPanel}>
          <div style={styles.teamPanelTitle}>Team Profile</div>
          <div style={styles.formGrid}>
            <div style={styles.field}>
              <label style={styles.label}>Team Name</label>
              <input style={styles.input} value={teamDraft.name} onChange={(e) => onTeamDraftChange((draft) => ({ ...draft, name: e.target.value }))} />
              {!teamDraft.name.trim() ? <div style={styles.error}>Team name is required.</div> : null}
            </div>
            <div style={styles.field}>
              <label style={styles.label}>Department</label>
              <input style={styles.input} value={teamDraft.department} onChange={(e) => onTeamDraftChange((draft) => ({ ...draft, department: e.target.value }))} />
            </div>
            <div style={{ ...styles.field, ...styles.fullWidth }}>
              <label style={styles.label}>Description</label>
              <textarea style={styles.textarea} value={teamDraft.description} onChange={(e) => onTeamDraftChange((draft) => ({ ...draft, description: e.target.value }))} />
            </div>
            <div style={styles.field}>
              <label style={styles.label}>Max Concurrent Workflows</label>
              <input
                type="number"
                min={1}
                style={styles.input}
                value={teamDraft.maxConcurrentWorkflows}
                onChange={(e) =>
                  onTeamDraftChange((draft) => ({
                    ...draft,
                    maxConcurrentWorkflows: Number.isFinite(Number(e.target.value)) ? Math.max(1, Number(e.target.value)) : 1,
                  }))
                }
              />
            </div>
            <div style={{ ...styles.field, ...styles.fullWidth }}>
              <label style={styles.label}>Team Skills</label>
              <AgentSkillChooser
                skills={skills}
                selectedIds={selectedTeamSkillIds}
                onToggle={(skillId, checked) => {
                  onSelectedTeamSkillIdsChange((current) =>
                    checked ? [...new Set([...current, skillId])] : current.filter((id) => id !== skillId),
                  );
                }}
              />
            </div>
          </div>
          <label style={styles.checkboxRow}>
            <input
              type="checkbox"
              checked={teamDraft.enabled}
              onChange={(e) => onTeamDraftChange((draft) => ({ ...draft, enabled: e.target.checked }))}
            />
            Team is active
          </label>
          {teamError ? <div style={styles.error}>{teamError}</div> : null}
          {teamFeedback ? <div style={styles.success}>{teamFeedback}</div> : null}
        </div>

        <div style={styles.teamManagementGrid}>
          <div style={styles.teamPanel}>
            <div style={styles.teamPanelTitle}>Current Members</div>
            <div style={styles.subList}>
              {selectedTeamMemberships.length === 0 ? (
                <div style={styles.empty}>No one assigned to this team yet.</div>
              ) : (
                selectedTeamMemberships.map((membership) => {
                  const agent = agents.find((entry) => entry.id === membership.agent_id);
                  return (
                    <div key={membership.id} style={styles.listItem}>
                      <div style={styles.itemTitle}>{agent?.name ?? "Unknown agent"}</div>
                      <div style={styles.itemMeta}>
                        {membership.title}
                        {membership.is_lead ? " · team lead" : ""}
                      </div>
                      <div style={styles.toolbar}>
                        <button type="button" style={styles.buttonSecondary} onClick={() => onRemoveMembership(membership.id)}>
                          Remove
                        </button>
                      </div>
                    </div>
                  );
                })
              )}
            </div>
          </div>
          <div style={styles.teamPanel}>
            <div style={styles.teamPanelTitle}>Add Member</div>
            <div style={styles.field}>
              <label style={styles.label}>Agent</label>
              <select
                style={styles.select}
                value={membershipDraft.agentId}
                onChange={(e) => onMembershipDraftChange((draft) => ({ ...draft, agentId: e.target.value }))}
              >
                <option value="">Select an agent</option>
                {agents.map((agent) => (
                  <option key={agent.id} value={agent.id}>
                    {agent.name} · {agent.role}
                  </option>
                ))}
              </select>
            </div>
            <div style={styles.field}>
              <label style={styles.label}>Title in Team</label>
              <input
                style={styles.input}
                value={membershipDraft.title}
                onChange={(e) => onMembershipDraftChange((draft) => ({ ...draft, title: e.target.value }))}
                placeholder="Staff Engineer, QA Lead, Architect"
              />
            </div>
            <label style={styles.checkboxRow}>
              <input
                type="checkbox"
                checked={membershipDraft.isLead}
                onChange={(e) => onMembershipDraftChange((draft) => ({ ...draft, isLead: e.target.checked }))}
              />
              Team lead
            </label>
            {membershipError ? <div style={styles.error}>{membershipError}</div> : null}
            <button type="button" style={styles.buttonPrimary} onClick={onAddMembership}>
              Add To Team
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
