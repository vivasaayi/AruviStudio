use crate::domain::agent::{
    AgentDefinition, AgentSkillLink, AgentTeam, AgentTeamMembership, Skill, TeamAssignment,
    TeamSkillLink, WorkflowStagePolicy,
};
use crate::domain::work_item::WorkItem;
use crate::error::AppError;
pub use crate::persistence::agent_model_binding_repo::{
    create_agent_model_binding, delete_agent_model_bindings_for_agent, get_agent_model_bindings,
    list_agent_model_bindings,
};
pub use crate::persistence::agent_run_repo::{
    add_agent_run_token_usage, create_agent_run, list_agent_runs_for_workflow,
    update_agent_run_failure, update_agent_run_snapshot_paths, update_agent_run_status,
};
use sqlx::{Row, SqlitePool};

fn row_to_agent_definition(row: sqlx::sqlite::SqliteRow) -> AgentDefinition {
    AgentDefinition {
        id: row.get("id"),
        name: row.get("name"),
        role: row.get("role"),
        description: row.get("description"),
        prompt_template_ref: row.get("prompt_template_ref"),
        allowed_tools: serde_json::from_str::<Vec<String>>(
            row.get::<String, _>("allowed_tools").as_str(),
        )
        .unwrap_or_default(),
        skill_tags: serde_json::from_str::<Vec<String>>(
            row.get::<String, _>("skill_tags").as_str(),
        )
        .unwrap_or_default(),
        boundaries: serde_json::from_str::<serde_json::Value>(
            row.get::<String, _>("boundaries").as_str(),
        )
        .unwrap_or_default(),
        enabled: row.get("enabled"),
        employment_status: row.get("employment_status"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

pub async fn list_agent_definitions(pool: &SqlitePool) -> Result<Vec<AgentDefinition>, AppError> {
    sqlx::query("SELECT id,name,role,description,prompt_template_ref,allowed_tools,skill_tags,boundaries,enabled,employment_status,created_at,updated_at FROM agent_definitions ORDER BY name")
        .map(row_to_agent_definition)
        .fetch_all(pool).await.map_err(|e| e.into())
}

pub struct CreateAgentDefinitionInput<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub role: &'a str,
    pub description: &'a str,
    pub prompt_template_ref: &'a str,
    pub allowed_tools: &'a str,
    pub skill_tags: &'a str,
    pub boundaries: &'a str,
    pub enabled: bool,
    pub employment_status: &'a str,
}

pub async fn create_agent_definition(
    pool: &SqlitePool,
    input: CreateAgentDefinitionInput<'_>,
) -> Result<AgentDefinition, AppError> {
    sqlx::query("INSERT INTO agent_definitions (id,name,role,description,prompt_template_ref,allowed_tools,skill_tags,boundaries,enabled,employment_status) VALUES (?,?,?,?,?,?,?,?,?,?)")
        .bind(input.id)
        .bind(input.name)
        .bind(input.role)
        .bind(input.description)
        .bind(input.prompt_template_ref)
        .bind(input.allowed_tools)
        .bind(input.skill_tags)
        .bind(input.boundaries)
        .bind(input.enabled)
        .bind(input.employment_status)
        .execute(pool)
        .await?;
    get_agent_definition(pool, input.id).await
}

pub async fn get_agent_definition(
    pool: &SqlitePool,
    id: &str,
) -> Result<AgentDefinition, AppError> {
    sqlx::query("SELECT id,name,role,description,prompt_template_ref,allowed_tools,skill_tags,boundaries,enabled,employment_status,created_at,updated_at FROM agent_definitions WHERE id=?")
        .bind(id)
        .map(row_to_agent_definition)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Agent {id} not found")))
}

pub async fn update_agent_definition(
    pool: &SqlitePool,
    patch: UpdateAgentDefinitionPatch<'_>,
) -> Result<AgentDefinition, AppError> {
    let existing = get_agent_definition(pool, patch.id).await?;
    let existing_allowed_tools =
        serde_json::to_string(&existing.allowed_tools).unwrap_or_else(|_| "[]".to_string());
    let existing_skill_tags =
        serde_json::to_string(&existing.skill_tags).unwrap_or_else(|_| "[]".to_string());
    let existing_boundaries =
        serde_json::to_string(&existing.boundaries).unwrap_or_else(|_| "{}".to_string());

    sqlx::query(
        "UPDATE agent_definitions SET name=?, role=?, description=?, prompt_template_ref=?, allowed_tools=?, skill_tags=?, boundaries=?, enabled=?, employment_status=?, updated_at=datetime('now') WHERE id=?",
    )
    .bind(patch.name.unwrap_or(&existing.name))
    .bind(patch.role.unwrap_or(&existing.role))
    .bind(patch.description.unwrap_or(&existing.description))
    .bind(patch.prompt_template_ref.unwrap_or(&existing.prompt_template_ref))
    .bind(patch.allowed_tools.unwrap_or(&existing_allowed_tools))
    .bind(patch.skill_tags.unwrap_or(&existing_skill_tags))
    .bind(patch.boundaries.unwrap_or(&existing_boundaries))
    .bind(patch.enabled.unwrap_or(existing.enabled))
    .bind(patch.employment_status.unwrap_or(&existing.employment_status))
    .bind(patch.id)
    .execute(pool)
    .await?;

    get_agent_definition(pool, patch.id).await
}

pub struct UpdateAgentDefinitionPatch<'a> {
    pub id: &'a str,
    pub name: Option<&'a str>,
    pub role: Option<&'a str>,
    pub description: Option<&'a str>,
    pub prompt_template_ref: Option<&'a str>,
    pub allowed_tools: Option<&'a str>,
    pub skill_tags: Option<&'a str>,
    pub boundaries: Option<&'a str>,
    pub enabled: Option<bool>,
    pub employment_status: Option<&'a str>,
}

pub async fn delete_agent_definition(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM agent_definitions WHERE id=?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn list_agent_teams(pool: &SqlitePool) -> Result<Vec<AgentTeam>, AppError> {
    sqlx::query_as::<_, AgentTeam>("SELECT id,name,department,description,enabled,max_concurrent_workflows,created_at,updated_at FROM agent_teams ORDER BY name")
        .fetch_all(pool)
        .await
        .map_err(|e| e.into())
}

pub async fn create_agent_team(
    pool: &SqlitePool,
    id: &str,
    name: &str,
    department: &str,
    description: &str,
    enabled: bool,
    max_concurrent_workflows: i32,
) -> Result<AgentTeam, AppError> {
    sqlx::query("INSERT INTO agent_teams (id,name,department,description,enabled,max_concurrent_workflows) VALUES (?,?,?,?,?,?)")
        .bind(id)
        .bind(name)
        .bind(department)
        .bind(description)
        .bind(enabled)
        .bind(max_concurrent_workflows)
        .execute(pool)
        .await?;
    get_agent_team(pool, id).await
}

pub async fn get_agent_team(pool: &SqlitePool, id: &str) -> Result<AgentTeam, AppError> {
    sqlx::query_as::<_, AgentTeam>("SELECT id,name,department,description,enabled,max_concurrent_workflows,created_at,updated_at FROM agent_teams WHERE id=?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Team {id} not found")))
}

pub async fn update_agent_team(
    pool: &SqlitePool,
    id: &str,
    name: Option<&str>,
    department: Option<&str>,
    description: Option<&str>,
    enabled: Option<bool>,
    max_concurrent_workflows: Option<i32>,
) -> Result<AgentTeam, AppError> {
    let existing = get_agent_team(pool, id).await?;
    sqlx::query("UPDATE agent_teams SET name=?, department=?, description=?, enabled=?, max_concurrent_workflows=?, updated_at=datetime('now') WHERE id=?")
        .bind(name.unwrap_or(&existing.name))
        .bind(department.unwrap_or(&existing.department))
        .bind(description.unwrap_or(&existing.description))
        .bind(enabled.unwrap_or(existing.enabled))
        .bind(max_concurrent_workflows.unwrap_or(existing.max_concurrent_workflows))
        .bind(id)
        .execute(pool)
        .await?;
    get_agent_team(pool, id).await
}

pub async fn delete_agent_team(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM agent_teams WHERE id=?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn list_team_memberships(
    pool: &SqlitePool,
) -> Result<Vec<AgentTeamMembership>, AppError> {
    sqlx::query_as::<_, AgentTeamMembership>("SELECT id,team_id,agent_id,title,is_lead,created_at FROM agent_team_memberships ORDER BY created_at DESC")
        .fetch_all(pool)
        .await
        .map_err(|e| e.into())
}

pub async fn add_team_member(
    pool: &SqlitePool,
    id: &str,
    team_id: &str,
    agent_id: &str,
    title: &str,
    is_lead: bool,
) -> Result<AgentTeamMembership, AppError> {
    sqlx::query(
        "INSERT INTO agent_team_memberships (id,team_id,agent_id,title,is_lead) VALUES (?,?,?,?,?)",
    )
    .bind(id)
    .bind(team_id)
    .bind(agent_id)
    .bind(title)
    .bind(is_lead)
    .execute(pool)
    .await?;
    sqlx::query_as::<_, AgentTeamMembership>("SELECT id,team_id,agent_id,title,is_lead,created_at FROM agent_team_memberships WHERE id=?")
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(|e| e.into())
}

pub async fn remove_team_member(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM agent_team_memberships WHERE id=?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn list_team_assignments(pool: &SqlitePool) -> Result<Vec<TeamAssignment>, AppError> {
    sqlx::query_as::<_, TeamAssignment>("SELECT id,team_id,scope_type,scope_id,created_at FROM team_assignments ORDER BY created_at DESC")
        .fetch_all(pool)
        .await
        .map_err(|e| e.into())
}

pub async fn assign_team_scope(
    pool: &SqlitePool,
    id: &str,
    team_id: &str,
    scope_type: &str,
    scope_id: &str,
) -> Result<TeamAssignment, AppError> {
    sqlx::query("INSERT INTO team_assignments (id,team_id,scope_type,scope_id) VALUES (?,?,?,?)")
        .bind(id)
        .bind(team_id)
        .bind(scope_type)
        .bind(scope_id)
        .execute(pool)
        .await?;
    sqlx::query_as::<_, TeamAssignment>(
        "SELECT id,team_id,scope_type,scope_id,created_at FROM team_assignments WHERE id=?",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|e| e.into())
}

pub async fn remove_team_assignment(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM team_assignments WHERE id=?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn resolve_team_for_work_item(
    pool: &SqlitePool,
    work_item: &WorkItem,
) -> Result<Option<AgentTeam>, AppError> {
    let scope_candidates = [
        work_item
            .capability_id
            .as_deref()
            .map(|id| ("capability", id)),
        work_item
            .product_area_id
            .as_deref()
            .map(|id| ("product_area", id)),
        work_item.product_id.as_deref().map(|id| ("product", id)),
    ];

    for candidate in scope_candidates.into_iter().flatten() {
        if let Some(team) = sqlx::query_as::<_, AgentTeam>(
            "SELECT t.id,t.name,t.department,t.description,t.enabled,t.max_concurrent_workflows,t.created_at,t.updated_at
             FROM team_assignments ta
             JOIN agent_teams t ON t.id = ta.team_id
             WHERE ta.scope_type=? AND ta.scope_id=? AND t.enabled=1
             ORDER BY ta.created_at ASC
             LIMIT 1",
        )
        .bind(candidate.0)
        .bind(candidate.1)
        .fetch_optional(pool)
        .await?
        {
            return Ok(Some(team));
        }
    }

    Ok(None)
}

pub async fn count_active_workflows_for_team(
    pool: &SqlitePool,
    team_id: &str,
) -> Result<i64, AppError> {
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM workflow_runs
         WHERE assigned_team_id=?
           AND status='running'
           AND current_stage NOT IN ('done','failed','cancelled')",
    )
    .bind(team_id)
    .fetch_one(pool)
    .await?;
    Ok(count)
}

pub async fn list_agents_for_team(
    pool: &SqlitePool,
    team_id: &str,
) -> Result<Vec<AgentDefinition>, AppError> {
    sqlx::query(
        "SELECT ad.id,ad.name,ad.role,ad.description,ad.prompt_template_ref,ad.allowed_tools,ad.skill_tags,ad.boundaries,ad.enabled,ad.employment_status,ad.created_at,ad.updated_at
         FROM agent_team_memberships atm
         JOIN agent_definitions ad ON ad.id = atm.agent_id
         WHERE atm.team_id=?
         ORDER BY atm.is_lead DESC, ad.name ASC",
    )
    .bind(team_id)
    .map(row_to_agent_definition)
    .fetch_all(pool)
    .await
    .map_err(|e| e.into())
}

pub async fn find_team_coordinator(
    pool: &SqlitePool,
    team_id: &str,
) -> Result<Option<AgentDefinition>, AppError> {
    let lead = sqlx::query(
        "SELECT ad.id,ad.name,ad.role,ad.description,ad.prompt_template_ref,ad.allowed_tools,ad.skill_tags,ad.boundaries,ad.enabled,ad.employment_status,ad.created_at,ad.updated_at
         FROM agent_team_memberships atm
         JOIN agent_definitions ad ON ad.id = atm.agent_id
         WHERE atm.team_id=? AND ad.enabled=1 AND ad.employment_status='active'
           AND lower(ad.role) IN ('manager','team_lead','coordinator')
         ORDER BY atm.is_lead DESC, CASE WHEN lower(ad.role)='manager' THEN 0 WHEN lower(ad.role)='team_lead' THEN 1 ELSE 2 END, ad.name ASC
         LIMIT 1",
    )
    .bind(team_id)
    .map(row_to_agent_definition)
    .fetch_optional(pool)
    .await?;

    Ok(lead)
}

fn row_to_workflow_stage_policy(row: sqlx::sqlite::SqliteRow) -> WorkflowStagePolicy {
    WorkflowStagePolicy {
        id: row.get("id"),
        stage_name: row.get("stage_name"),
        primary_roles: serde_json::from_str::<Vec<String>>(
            row.get::<String, _>("primary_roles").as_str(),
        )
        .unwrap_or_default(),
        fallback_roles: serde_json::from_str::<Vec<String>>(
            row.get::<String, _>("fallback_roles").as_str(),
        )
        .unwrap_or_default(),
        coordinator_required: row.get("coordinator_required"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

pub async fn list_workflow_stage_policies(
    pool: &SqlitePool,
) -> Result<Vec<WorkflowStagePolicy>, AppError> {
    sqlx::query(
        "SELECT id,stage_name,primary_roles,fallback_roles,coordinator_required,created_at,updated_at FROM workflow_stage_policies ORDER BY stage_name ASC"
    )
    .map(row_to_workflow_stage_policy)
    .fetch_all(pool)
    .await
    .map_err(|e| e.into())
}

pub async fn get_workflow_stage_policy(
    pool: &SqlitePool,
    stage_name: &str,
) -> Result<Option<WorkflowStagePolicy>, AppError> {
    sqlx::query(
        "SELECT id,stage_name,primary_roles,fallback_roles,coordinator_required,created_at,updated_at FROM workflow_stage_policies WHERE stage_name=?"
    )
    .bind(stage_name)
    .map(row_to_workflow_stage_policy)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.into())
}

pub async fn upsert_workflow_stage_policy(
    pool: &SqlitePool,
    id: &str,
    stage_name: &str,
    primary_roles: &str,
    fallback_roles: &str,
    coordinator_required: bool,
) -> Result<WorkflowStagePolicy, AppError> {
    sqlx::query(
        "INSERT INTO workflow_stage_policies (id,stage_name,primary_roles,fallback_roles,coordinator_required)
         VALUES (?,?,?,?,?)
         ON CONFLICT(stage_name) DO UPDATE SET
            primary_roles=excluded.primary_roles,
            fallback_roles=excluded.fallback_roles,
            coordinator_required=excluded.coordinator_required,
            updated_at=datetime('now')"
    )
    .bind(id)
    .bind(stage_name)
    .bind(primary_roles)
    .bind(fallback_roles)
    .bind(coordinator_required)
    .execute(pool)
    .await?;

    get_workflow_stage_policy(pool, stage_name)
        .await?
        .ok_or_else(|| {
            AppError::NotFound(format!(
                "Workflow stage policy for {stage_name} not found after upsert"
            ))
        })
}

pub async fn delete_workflow_stage_policy(
    pool: &SqlitePool,
    stage_name: &str,
) -> Result<(), AppError> {
    sqlx::query("DELETE FROM workflow_stage_policies WHERE stage_name=?")
        .bind(stage_name)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn list_skills(pool: &SqlitePool) -> Result<Vec<Skill>, AppError> {
    sqlx::query_as::<_, Skill>(
        "SELECT id,name,category,description,instructions,enabled,created_at,updated_at FROM skills ORDER BY name ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.into())
}

pub async fn create_skill(
    pool: &SqlitePool,
    id: &str,
    name: &str,
    category: &str,
    description: &str,
    instructions: &str,
    enabled: bool,
) -> Result<Skill, AppError> {
    sqlx::query("INSERT INTO skills (id,name,category,description,instructions,enabled) VALUES (?,?,?,?,?,?)")
        .bind(id)
        .bind(name)
        .bind(category)
        .bind(description)
        .bind(instructions)
        .bind(enabled)
        .execute(pool)
        .await?;
    get_skill(pool, id).await
}

pub async fn get_skill(pool: &SqlitePool, id: &str) -> Result<Skill, AppError> {
    sqlx::query_as::<_, Skill>("SELECT id,name,category,description,instructions,enabled,created_at,updated_at FROM skills WHERE id=?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Skill {id} not found")))
}

pub async fn update_skill(
    pool: &SqlitePool,
    id: &str,
    name: Option<&str>,
    category: Option<&str>,
    description: Option<&str>,
    instructions: Option<&str>,
    enabled: Option<bool>,
) -> Result<Skill, AppError> {
    let existing = get_skill(pool, id).await?;
    sqlx::query(
        "UPDATE skills SET name=?, category=?, description=?, instructions=?, enabled=?, updated_at=datetime('now') WHERE id=?",
    )
    .bind(name.unwrap_or(&existing.name))
    .bind(category.unwrap_or(&existing.category))
    .bind(description.unwrap_or(&existing.description))
    .bind(instructions.unwrap_or(&existing.instructions))
    .bind(enabled.unwrap_or(existing.enabled))
    .bind(id)
    .execute(pool)
    .await?;
    get_skill(pool, id).await
}

pub async fn delete_skill(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM skills WHERE id=?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn list_agent_skill_links(pool: &SqlitePool) -> Result<Vec<AgentSkillLink>, AppError> {
    sqlx::query_as::<_, AgentSkillLink>(
        "SELECT id,agent_id,skill_id,proficiency,created_at FROM agent_skill_links ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.into())
}

pub async fn link_skill_to_agent(
    pool: &SqlitePool,
    id: &str,
    agent_id: &str,
    skill_id: &str,
    proficiency: &str,
) -> Result<AgentSkillLink, AppError> {
    sqlx::query(
        "INSERT OR REPLACE INTO agent_skill_links (id,agent_id,skill_id,proficiency,created_at) VALUES (?,?,?,?,datetime('now'))",
    )
    .bind(id)
    .bind(agent_id)
    .bind(skill_id)
    .bind(proficiency)
    .execute(pool)
    .await?;
    sqlx::query_as::<_, AgentSkillLink>(
        "SELECT id,agent_id,skill_id,proficiency,created_at FROM agent_skill_links WHERE agent_id=? AND skill_id=? LIMIT 1",
    )
    .bind(agent_id)
    .bind(skill_id)
    .fetch_one(pool)
    .await
    .map_err(|e| e.into())
}

pub async fn unlink_skill_from_agent(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM agent_skill_links WHERE id=?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn list_team_skill_links(pool: &SqlitePool) -> Result<Vec<TeamSkillLink>, AppError> {
    sqlx::query_as::<_, TeamSkillLink>(
        "SELECT id,team_id,skill_id,created_at FROM team_skill_links ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.into())
}

pub async fn link_skill_to_team(
    pool: &SqlitePool,
    id: &str,
    team_id: &str,
    skill_id: &str,
) -> Result<TeamSkillLink, AppError> {
    sqlx::query(
        "INSERT OR REPLACE INTO team_skill_links (id,team_id,skill_id,created_at) VALUES (?,?,?,datetime('now'))",
    )
    .bind(id)
    .bind(team_id)
    .bind(skill_id)
    .execute(pool)
    .await?;
    sqlx::query_as::<_, TeamSkillLink>(
        "SELECT id,team_id,skill_id,created_at FROM team_skill_links WHERE team_id=? AND skill_id=? LIMIT 1",
    )
    .bind(team_id)
    .bind(skill_id)
    .fetch_one(pool)
    .await
    .map_err(|e| e.into())
}

pub async fn unlink_skill_from_team(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM team_skill_links WHERE id=?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
