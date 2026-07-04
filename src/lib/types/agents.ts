export interface AgentModelBinding {
  id: string;
  agent_id: string;
  model_id: string;
  priority: number;
  created_at: string;
}

export interface AgentDefinition {
  id: string;
  name: string;
  role: string;
  description: string;
  prompt_template_ref: string;
  allowed_tools: string[];
  skill_tags: string[];
  boundaries: Record<string, unknown>;
  enabled: boolean;
  employment_status: "active" | "inactive" | "terminated";
  created_at: string;
  updated_at: string;
}

export interface AgentTeam {
  id: string;
  name: string;
  department: string;
  description: string;
  enabled: boolean;
  max_concurrent_workflows: number;
  created_at: string;
  updated_at: string;
}

export interface AgentTeamMembership {
  id: string;
  team_id: string;
  agent_id: string;
  title: string;
  is_lead: boolean;
  created_at: string;
}

export interface TeamAssignment {
  id: string;
  team_id: string;
  scope_type: "product" | "product_area" | "capability";
  scope_id: string;
  created_at: string;
}

export interface Skill {
  id: string;
  name: string;
  category: string;
  description: string;
  instructions: string;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface AgentSkillLink {
  id: string;
  agent_id: string;
  skill_id: string;
  proficiency: "learning" | "working" | "expert";
  created_at: string;
}

export interface TeamSkillLink {
  id: string;
  team_id: string;
  skill_id: string;
  created_at: string;
}
