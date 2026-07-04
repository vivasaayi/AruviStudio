import type React from "react";

import type { Skill } from "../../../lib/types";
import type { SkillDraft } from "../lib/agentRegistryPageModel";
import { styles } from "../lib/agentRegistryPageStyles";

type AgentSkillsTabProps = {
  skills: Skill[];
  selectedSkillId: string | null;
  selectedSkill: Skill | null;
  skillDraft: SkillDraft;
  onCreateNewSkill: () => void;
  onSelectSkill: (skillId: string) => void;
  onSkillDraftChange: React.Dispatch<React.SetStateAction<SkillDraft>>;
  onDeleteSkill: (skillId: string) => void;
  onSaveSkill: () => void;
  onResetSkillForm: () => void;
  skillError: string | null;
  skillFeedback: string | null;
};

export function AgentSkillsTab({
  skills,
  selectedSkillId,
  selectedSkill,
  skillDraft,
  onCreateNewSkill,
  onSelectSkill,
  onSkillDraftChange,
  onDeleteSkill,
  onSaveSkill,
  onResetSkillForm,
  skillError,
  skillFeedback,
}: AgentSkillsTabProps) {
  return (
    <div style={styles.workspace}>
      <div style={styles.rail}>
        <div style={styles.toolbar}>
          <button
            type="button"
            style={styles.buttonPrimary}
            onClick={onCreateNewSkill}
          >
            + New Skill
          </button>
        </div>
        <div style={styles.sectionTitle}>Catalog</div>
        <div style={styles.list}>
          {skills.length === 0 ? (
            <div style={styles.empty}>No skills defined yet.</div>
          ) : (
            skills.map((skill) => (
              <button
                key={skill.id}
                type="button"
                style={{
                  ...styles.listItem,
                  ...(skill.id === selectedSkillId ? styles.listItemActive : {}),
                  textAlign: "left",
                }}
                onClick={() => onSelectSkill(skill.id)}
              >
                <div style={styles.itemTitle}>{skill.name}</div>
                <div style={styles.itemMeta}>{skill.category}</div>
                <div style={styles.badgeRow}>
                  <span style={styles.badgeMuted}>{skill.enabled ? "enabled" : "disabled"}</span>
                </div>
              </button>
            ))
          )}
        </div>
      </div>
      <div style={styles.detail}>
        <div style={styles.headerRow}>
          <div style={styles.titleWrap}>
            <h2 style={styles.title}>{selectedSkill ? "Edit Skill" : "Create Skill"}</h2>
            <div style={styles.subtitle}>Define reusable capability packs and link them to both agents and teams.</div>
          </div>
          {selectedSkill ? (
            <button type="button" style={styles.buttonDanger} onClick={() => onDeleteSkill(selectedSkill.id)}>
              Delete
            </button>
          ) : null}
        </div>
        <div style={styles.formGrid}>
          <div style={styles.field}>
            <label style={styles.label}>Name</label>
            <input style={styles.input} value={skillDraft.name} onChange={(event) => onSkillDraftChange((draft) => ({ ...draft, name: event.target.value }))} />
          </div>
          <div style={styles.field}>
            <label style={styles.label}>Category</label>
            <input style={styles.input} value={skillDraft.category} onChange={(event) => onSkillDraftChange((draft) => ({ ...draft, category: event.target.value }))} />
          </div>
          <div style={{ ...styles.field, ...styles.fullWidth }}>
            <label style={styles.label}>Description</label>
            <textarea style={styles.textarea} value={skillDraft.description} onChange={(event) => onSkillDraftChange((draft) => ({ ...draft, description: event.target.value }))} />
          </div>
          <div style={{ ...styles.field, ...styles.fullWidth }}>
            <label style={styles.label}>Instructions</label>
            <textarea style={{ ...styles.textarea, minHeight: 140 }} value={skillDraft.instructions} onChange={(event) => onSkillDraftChange((draft) => ({ ...draft, instructions: event.target.value }))} />
          </div>
        </div>
        <label style={styles.checkboxRow}>
          <input
            type="checkbox"
            checked={skillDraft.enabled}
            onChange={(event) => onSkillDraftChange((draft) => ({ ...draft, enabled: event.target.checked }))}
          />
          Skill is active
        </label>
        {skillError ? <div style={styles.error}>{skillError}</div> : null}
        {skillFeedback ? <div style={styles.success}>{skillFeedback}</div> : null}
        <div style={styles.toolbar}>
          <button type="button" style={styles.buttonPrimary} onClick={onSaveSkill}>
            {selectedSkill ? "Save Skill" : "Create Skill"}
          </button>
          <button
            type="button"
            style={styles.buttonSecondary}
            onClick={onResetSkillForm}
          >
            Reset
          </button>
        </div>
      </div>
    </div>
  );
}
