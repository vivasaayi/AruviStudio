import type { Dispatch, SetStateAction } from "react";

import type { AgentDefinition, AgentTeam, AgentTeamMembership, ModelDefinition, Skill } from "../../../lib/types";
import { styles } from "../lib/agentRegistryPageStyles";
import type { AgentDraft } from "../lib/agentRegistryPageModel";
import { AgentOrgTreeRail } from "./AgentOrgTreeRail";
import { AgentSkillChooser } from "./AgentSkillChooser";

type AgentProfileTabProps = {
  agents: AgentDefinition[];
  teams: AgentTeam[];
  skills: Skill[];
  modelDefinitions: ModelDefinition[];
  agentsLoading: boolean;
  teamsLoading: boolean;
  selectedAgentId: string | null;
  selectedTeamId: string | null;
  selectedAgent: AgentDefinition | null;
  expandedTeams: Record<string, boolean>;
  teamMembershipsByTeam: Map<string, AgentTeamMembership[]>;
  unassignedAgents: AgentDefinition[];
  agentDraft: AgentDraft;
  selectedAgentSkillIds: string[];
  selectedAgentModelId: string;
  agentError: string | null;
  agentFeedback: string | null;
  onHireAgent: () => void;
  onBindAllAgentsToDeepSeek: () => void;
  onBindCodingAgentsToDeepSeek: () => void;
  onExpandedTeamsChange: (updater: (current: Record<string, boolean>) => Record<string, boolean>) => void;
  onSelectTeam: (teamId: string) => void;
  onEditTeam: (teamId: string) => void;
  onSelectAgent: (teamId: string | null, agentId: string) => void;
  onUnassignMembership: (teamId: string, membershipId: string) => void;
  onDeleteAgent: (agentId: string) => void;
  onAgentDraftChange: Dispatch<SetStateAction<AgentDraft>>;
  onSelectedAgentSkillIdsChange: Dispatch<SetStateAction<string[]>>;
  onSelectedAgentModelChange: (modelId: string) => void;
  onBindSelectedAgentModel: () => void;
  onSaveAgent: () => void;
  onResetAgentForm: () => void;
};

export function AgentProfileTab({
  agents,
  teams,
  skills,
  modelDefinitions,
  agentsLoading,
  teamsLoading,
  selectedAgentId,
  selectedTeamId,
  selectedAgent,
  expandedTeams,
  teamMembershipsByTeam,
  unassignedAgents,
  agentDraft,
  selectedAgentSkillIds,
  selectedAgentModelId,
  agentError,
  agentFeedback,
  onHireAgent,
  onBindAllAgentsToDeepSeek,
  onBindCodingAgentsToDeepSeek,
  onExpandedTeamsChange,
  onSelectTeam,
  onEditTeam,
  onSelectAgent,
  onUnassignMembership,
  onDeleteAgent,
  onAgentDraftChange,
  onSelectedAgentSkillIdsChange,
  onSelectedAgentModelChange,
  onBindSelectedAgentModel,
  onSaveAgent,
  onResetAgentForm,
}: AgentProfileTabProps) {
  return (
    <div style={styles.workspace}>
      <AgentOrgTreeRail
        agents={agents}
        teams={teams}
        agentsLoading={agentsLoading}
        teamsLoading={teamsLoading}
        selectedAgentId={selectedAgentId}
        selectedTeamId={selectedTeamId}
        expandedTeams={expandedTeams}
        teamMembershipsByTeam={teamMembershipsByTeam}
        unassignedAgents={unassignedAgents}
        onHireAgent={onHireAgent}
        onBindAllAgentsToDeepSeek={onBindAllAgentsToDeepSeek}
        onBindCodingAgentsToDeepSeek={onBindCodingAgentsToDeepSeek}
        onExpandedTeamsChange={onExpandedTeamsChange}
        onSelectTeam={onSelectTeam}
        onEditTeam={onEditTeam}
        onSelectAgent={onSelectAgent}
        onUnassignMembership={onUnassignMembership}
      />
      <div style={styles.detail}>
        <div style={styles.headerRow}>
          <div style={styles.titleWrap}>
            <h2 style={styles.title}>{selectedAgent ? "Edit Agent" : "Hire Agent"}</h2>
            <div style={styles.subtitle}>Model explicit roles and link each agent to reusable skills instead of relying only on freeform tags.</div>
          </div>
          {selectedAgent ? (
            <button type="button" style={styles.buttonDanger} onClick={() => onDeleteAgent(selectedAgent.id)}>
              Remove
            </button>
          ) : null}
        </div>
        <div style={styles.formGrid}>
          <div style={styles.field}>
            <label style={styles.label}>Name</label>
            <input style={styles.input} value={agentDraft.name} onChange={(e) => onAgentDraftChange((draft) => ({ ...draft, name: e.target.value }))} />
          </div>
          <div style={styles.field}>
            <label style={styles.label}>Role</label>
            <input style={styles.input} value={agentDraft.role} onChange={(e) => onAgentDraftChange((draft) => ({ ...draft, role: e.target.value }))} />
          </div>
          <div style={{ ...styles.field, ...styles.fullWidth }}>
            <label style={styles.label}>Description</label>
            <textarea style={styles.textarea} value={agentDraft.description} onChange={(e) => onAgentDraftChange((draft) => ({ ...draft, description: e.target.value }))} />
          </div>
          <div style={styles.field}>
            <label style={styles.label}>Prompt Template Ref</label>
            <input style={styles.input} value={agentDraft.promptTemplateRef} onChange={(e) => onAgentDraftChange((draft) => ({ ...draft, promptTemplateRef: e.target.value }))} />
          </div>
          <div style={styles.field}>
            <label style={styles.label}>Employment Status</label>
            <select
              style={styles.select}
              value={agentDraft.employmentStatus}
              onChange={(e) =>
                onAgentDraftChange((draft) => ({ ...draft, employmentStatus: e.target.value as AgentDraft["employmentStatus"] }))
              }
            >
              <option value="active">active</option>
              <option value="inactive">inactive</option>
              <option value="terminated">terminated</option>
            </select>
          </div>
          <div style={styles.field}>
            <label style={styles.label}>Allowed Tools (comma-separated)</label>
            <input style={styles.input} value={agentDraft.allowedTools} onChange={(e) => onAgentDraftChange((draft) => ({ ...draft, allowedTools: e.target.value }))} />
          </div>
          <div style={styles.field}>
            <label style={styles.label}>Skill Tags (comma-separated)</label>
            <input style={styles.input} value={agentDraft.skillTags} onChange={(e) => onAgentDraftChange((draft) => ({ ...draft, skillTags: e.target.value }))} />
          </div>
          <div style={{ ...styles.field, ...styles.fullWidth }}>
            <label style={styles.label}>Boundaries (JSON object)</label>
            <textarea style={{ ...styles.textarea, minHeight: 120 }} value={agentDraft.boundaries} onChange={(e) => onAgentDraftChange((draft) => ({ ...draft, boundaries: e.target.value }))} />
          </div>
          <div style={{ ...styles.field, ...styles.fullWidth }}>
            <label style={styles.label}>Linked Skills</label>
            <AgentSkillChooser
              skills={skills}
              selectedIds={selectedAgentSkillIds}
              onToggle={(skillId, checked) => {
                onSelectedAgentSkillIdsChange((current) =>
                  checked ? [...new Set([...current, skillId])] : current.filter((id) => id !== skillId),
                );
              }}
            />
          </div>
          <div style={{ ...styles.field, ...styles.fullWidth }}>
            <label style={styles.label}>Primary Model</label>
            <select
              style={styles.select}
              value={selectedAgentModelId}
              onChange={(e) => onSelectedAgentModelChange(e.target.value)}
            >
              <option value="">Select a model definition</option>
              {modelDefinitions.map((model) => (
                <option key={model.id} value={model.id}>
                  {model.name}
                </option>
              ))}
            </select>
            <div style={styles.toolbar}>
              <button type="button" style={styles.buttonSecondary} onClick={onBindSelectedAgentModel} disabled={!selectedAgent || !selectedAgentModelId}>
                Bind Model
              </button>
              {selectedAgentModelId ? (
                <span style={styles.infoValue}>
                  {modelDefinitions.find((model) => model.id === selectedAgentModelId)?.name ?? "Selected model"}
                </span>
              ) : null}
            </div>
          </div>
        </div>
        <label style={styles.checkboxRow}>
          <input
            type="checkbox"
            checked={agentDraft.enabled}
            onChange={(e) => onAgentDraftChange((draft) => ({ ...draft, enabled: e.target.checked }))}
          />
          Enabled for orchestration
        </label>
        {agentError ? <div style={styles.error}>{agentError}</div> : null}
        {agentFeedback ? <div style={styles.success}>{agentFeedback}</div> : null}
        <div style={styles.toolbar}>
          <button type="button" style={styles.buttonPrimary} onClick={onSaveAgent}>
            {selectedAgent ? "Save Agent" : "Hire Agent"}
          </button>
          <button type="button" style={styles.buttonSecondary} onClick={onResetAgentForm}>
            Reset
          </button>
        </div>
      </div>
    </div>
  );
}
