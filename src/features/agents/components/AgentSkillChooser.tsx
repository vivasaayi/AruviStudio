import type { Skill } from "../../../lib/types";
import { styles } from "../lib/agentRegistryPageStyles";

type Props = {
  skills: Skill[];
  selectedIds: string[];
  onToggle: (skillId: string, checked: boolean) => void;
};

export function AgentSkillChooser({ skills, selectedIds, onToggle }: Props) {
  return (
    <div style={styles.skillList}>
      {skills.length === 0 ? (
        <div style={styles.empty}>Add skills in the Skills tab first.</div>
      ) : (
        skills.map((skill) => (
          <label key={skill.id} style={styles.skillPill}>
            <input
              type="checkbox"
              checked={selectedIds.includes(skill.id)}
              onChange={(event) => onToggle(skill.id, event.target.checked)}
            />
            <span style={{ color: "#374151", fontSize: 12 }}>
              {skill.name} <span style={{ color: "#6b7280" }}>({skill.category})</span>
            </span>
          </label>
        ))
      )}
    </div>
  );
}
