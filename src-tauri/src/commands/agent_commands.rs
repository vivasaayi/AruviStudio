use tauri::State;
use tracing::{debug, error, info};

use crate::domain::agent::{
    AgentDefinition, AgentModelBinding, AgentSkillLink, AgentTeam, AgentTeamMembership, Skill,
    TeamAssignment, TeamSkillLink, WorkflowStagePolicy,
};
use crate::error::AppError;
use crate::persistence::agent_repo;
use crate::state::AppState;

fn validate_json_array(label: &str, value: &str) -> Result<(), AppError> {
    let parsed = serde_json::from_str::<serde_json::Value>(value)?;
    if !parsed.is_array() {
        return Err(AppError::Validation(format!(
            "{label} must be a JSON array"
        )));
    }
    Ok(())
}

fn validate_json_object(label: &str, value: &str) -> Result<(), AppError> {
    let parsed = serde_json::from_str::<serde_json::Value>(value)?;
    if !parsed.is_object() {
        return Err(AppError::Validation(format!(
            "{label} must be a JSON object"
        )));
    }
    Ok(())
}

fn resolve_required(
    value: Option<String>,
    legacy: Option<String>,
    field_name: &str,
) -> Result<String, AppError> {
    value
        .or(legacy)
        .ok_or_else(|| AppError::Validation(format!("missing {}", field_name)))
}

#[tauri::command]
pub async fn list_agent_definitions(
    state: State<'_, AppState>,
) -> Result<Vec<AgentDefinition>, AppError> {
    debug!("list_agent_definitions requested");
    let result = agent_repo::list_agent_definitions(&state.db).await;
    if let Err(err) = &result {
        error!(error = %err, "list_agent_definitions failed");
    }
    result
}

#[tauri::command]
pub async fn list_agent_model_bindings(
    state: State<'_, AppState>,
) -> Result<Vec<AgentModelBinding>, AppError> {
    debug!("list_agent_model_bindings requested");
    let result = agent_repo::list_agent_model_bindings(&state.db).await;
    if let Err(err) = &result {
        error!(error = %err, "list_agent_model_bindings failed");
    }
    result
}

#[tauri::command]
pub async fn set_primary_agent_model_binding(
    state: State<'_, AppState>,
    agent_id: String,
    model_id: String,
) -> Result<AgentModelBinding, AppError> {
    info!(agent_id = %agent_id, model_id = %model_id, "set_primary_agent_model_binding requested");
    agent_repo::delete_agent_model_bindings_for_agent(&state.db, &agent_id).await?;
    let id = uuid::Uuid::new_v4().to_string();
    let result =
        agent_repo::create_agent_model_binding(&state.db, &id, &agent_id, &model_id, 1).await;
    match &result {
        Ok(binding) => info!(binding_id = %binding.id, "set_primary_agent_model_binding succeeded"),
        Err(err) => {
            error!(binding_id = %id, agent_id = %agent_id, model_id = %model_id, error = %err, "set_primary_agent_model_binding failed")
        }
    }
    result
}

#[tauri::command]
pub async fn create_agent_definition(
    state: State<'_, AppState>,
    name: String,
    role: String,
    description: String,
    prompt_template_ref: String,
    allowed_tools: String,
    skill_tags: String,
    boundaries: String,
    enabled: bool,
    employment_status: String,
) -> Result<AgentDefinition, AppError> {
    info!(agent_name = %name, role = %role, "create_agent_definition requested");
    validate_json_array("allowed_tools", &allowed_tools)?;
    validate_json_array("skill_tags", &skill_tags)?;
    validate_json_object("boundaries", &boundaries)?;
    let id = uuid::Uuid::new_v4().to_string();
    let result = agent_repo::create_agent_definition(
        &state.db,
        &id,
        &name,
        &role,
        &description,
        &prompt_template_ref,
        &allowed_tools,
        &skill_tags,
        &boundaries,
        enabled,
        &employment_status,
    )
    .await;
    match &result {
        Ok(agent) => info!(agent_id = %agent.id, "create_agent_definition succeeded"),
        Err(err) => error!(agent_id = %id, error = %err, "create_agent_definition failed"),
    }
    result
}

#[tauri::command]
pub async fn update_agent_definition(
    state: State<'_, AppState>,
    id: String,
    name: Option<String>,
    role: Option<String>,
    description: Option<String>,
    prompt_template_ref: Option<String>,
    allowed_tools: Option<String>,
    skill_tags: Option<String>,
    boundaries: Option<String>,
    enabled: Option<bool>,
    employment_status: Option<String>,
) -> Result<AgentDefinition, AppError> {
    info!(agent_id = %id, "update_agent_definition requested");
    if let Some(value) = allowed_tools.as_deref() {
        validate_json_array("allowed_tools", value)?;
    }
    if let Some(value) = skill_tags.as_deref() {
        validate_json_array("skill_tags", value)?;
    }
    if let Some(value) = boundaries.as_deref() {
        validate_json_object("boundaries", value)?;
    }
    debug!(
        agent_id = %id,
        has_name = name.is_some(),
        has_role = role.is_some(),
        has_description = description.is_some(),
        has_prompt_template_ref = prompt_template_ref.is_some(),
        has_allowed_tools = allowed_tools.is_some(),
        has_skill_tags = skill_tags.is_some(),
        has_boundaries = boundaries.is_some(),
        has_enabled = enabled.is_some(),
        has_employment_status = employment_status.is_some(),
        "update_agent_definition payload summary"
    );
    let result = agent_repo::update_agent_definition(
        &state.db,
        &id,
        name.as_deref(),
        role.as_deref(),
        description.as_deref(),
        prompt_template_ref.as_deref(),
        allowed_tools.as_deref(),
        skill_tags.as_deref(),
        boundaries.as_deref(),
        enabled,
        employment_status.as_deref(),
    )
    .await;
    match &result {
        Ok(_) => info!(agent_id = %id, "update_agent_definition succeeded"),
        Err(err) => error!(agent_id = %id, error = %err, "update_agent_definition failed"),
    }
    result
}

#[tauri::command]
pub async fn delete_agent_definition(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), AppError> {
    info!(agent_id = %id, "delete_agent_definition requested");
    let result = agent_repo::delete_agent_definition(&state.db, &id).await;
    match &result {
        Ok(_) => info!(agent_id = %id, "delete_agent_definition succeeded"),
        Err(err) => error!(agent_id = %id, error = %err, "delete_agent_definition failed"),
    }
    result
}

#[tauri::command]
pub async fn list_agent_teams(state: State<'_, AppState>) -> Result<Vec<AgentTeam>, AppError> {
    debug!("list_agent_teams requested");
    let result = agent_repo::list_agent_teams(&state.db).await;
    if let Err(err) = &result {
        error!(error = %err, "list_agent_teams failed");
    }
    result
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn create_agent_team(
    state: State<'_, AppState>,
    name: String,
    department: String,
    description: String,
    enabled: bool,
    max_concurrent_workflows: Option<i32>,
    maxConcurrentWorkflows: Option<i32>,
) -> Result<AgentTeam, AppError> {
    let max_concurrent_workflows = max_concurrent_workflows
        .or(maxConcurrentWorkflows)
        .unwrap_or(2);
    info!(team_name = %name, department = %department, "create_agent_team requested");
    let id = uuid::Uuid::new_v4().to_string();
    let result = agent_repo::create_agent_team(
        &state.db,
        &id,
        &name,
        &department,
        &description,
        enabled,
        max_concurrent_workflows,
    )
    .await;
    match &result {
        Ok(team) => info!(team_id = %team.id, "create_agent_team succeeded"),
        Err(err) => error!(team_id = %id, error = %err, "create_agent_team failed"),
    }
    result
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn update_agent_team(
    state: State<'_, AppState>,
    id: String,
    name: Option<String>,
    department: Option<String>,
    description: Option<String>,
    enabled: Option<bool>,
    max_concurrent_workflows: Option<i32>,
    maxConcurrentWorkflows: Option<i32>,
) -> Result<AgentTeam, AppError> {
    let max_concurrent_workflows = max_concurrent_workflows.or(maxConcurrentWorkflows);
    info!(team_id = %id, "update_agent_team requested");
    let result = agent_repo::update_agent_team(
        &state.db,
        &id,
        name.as_deref(),
        department.as_deref(),
        description.as_deref(),
        enabled,
        max_concurrent_workflows,
    )
    .await;
    match &result {
        Ok(_) => info!(team_id = %id, "update_agent_team succeeded"),
        Err(err) => error!(team_id = %id, error = %err, "update_agent_team failed"),
    }
    result
}

#[tauri::command]
pub async fn delete_agent_team(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    info!(team_id = %id, "delete_agent_team requested");
    let result = agent_repo::delete_agent_team(&state.db, &id).await;
    match &result {
        Ok(_) => info!(team_id = %id, "delete_agent_team succeeded"),
        Err(err) => error!(team_id = %id, error = %err, "delete_agent_team failed"),
    }
    result
}

#[tauri::command]
pub async fn list_team_memberships(
    state: State<'_, AppState>,
) -> Result<Vec<AgentTeamMembership>, AppError> {
    debug!("list_team_memberships requested");
    let result = agent_repo::list_team_memberships(&state.db).await;
    if let Err(err) = &result {
        error!(error = %err, "list_team_memberships failed");
    }
    result
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn add_team_member(
    state: State<'_, AppState>,
    team_id: Option<String>,
    teamId: Option<String>,
    agent_id: Option<String>,
    agentId: Option<String>,
    title: String,
    is_lead: Option<bool>,
    isLead: Option<bool>,
) -> Result<AgentTeamMembership, AppError> {
    let team_id = resolve_required(team_id, teamId, "team id")?;
    let agent_id = resolve_required(agent_id, agentId, "agent id")?;
    let is_lead = is_lead.or(isLead).unwrap_or(false);
    info!(team_id = %team_id, agent_id = %agent_id, title = %title, is_lead, "add_team_member requested");
    let id = uuid::Uuid::new_v4().to_string();
    let result =
        agent_repo::add_team_member(&state.db, &id, &team_id, &agent_id, &title, is_lead).await;
    match &result {
        Ok(membership) => info!(membership_id = %membership.id, "add_team_member succeeded"),
        Err(err) => {
            error!(membership_id = %id, team_id = %team_id, agent_id = %agent_id, error = %err, "add_team_member failed")
        }
    }
    result
}

#[tauri::command]
pub async fn remove_team_member(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    info!(membership_id = %id, "remove_team_member requested");
    let result = agent_repo::remove_team_member(&state.db, &id).await;
    match &result {
        Ok(_) => info!(membership_id = %id, "remove_team_member succeeded"),
        Err(err) => error!(membership_id = %id, error = %err, "remove_team_member failed"),
    }
    result
}

#[tauri::command]
pub async fn list_team_assignments(
    state: State<'_, AppState>,
) -> Result<Vec<TeamAssignment>, AppError> {
    debug!("list_team_assignments requested");
    let result = agent_repo::list_team_assignments(&state.db).await;
    if let Err(err) = &result {
        error!(error = %err, "list_team_assignments failed");
    }
    result
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn assign_team_scope(
    state: State<'_, AppState>,
    team_id: Option<String>,
    teamId: Option<String>,
    scope_type: Option<String>,
    scopeType: Option<String>,
    scope_id: Option<String>,
    scopeId: Option<String>,
) -> Result<TeamAssignment, AppError> {
    let team_id = resolve_required(team_id, teamId, "team id")?;
    let scope_type = resolve_required(scope_type, scopeType, "scope type")?;
    let scope_id = resolve_required(scope_id, scopeId, "scope id")?;
    info!(team_id = %team_id, scope_type = %scope_type, scope_id = %scope_id, "assign_team_scope requested");
    let id = uuid::Uuid::new_v4().to_string();
    let result =
        agent_repo::assign_team_scope(&state.db, &id, &team_id, &scope_type, &scope_id).await;
    match &result {
        Ok(assignment) => info!(assignment_id = %assignment.id, "assign_team_scope succeeded"),
        Err(err) => {
            error!(assignment_id = %id, team_id = %team_id, scope_type = %scope_type, scope_id = %scope_id, error = %err, "assign_team_scope failed")
        }
    }
    result
}

#[tauri::command]
pub async fn remove_team_assignment(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), AppError> {
    info!(assignment_id = %id, "remove_team_assignment requested");
    let result = agent_repo::remove_team_assignment(&state.db, &id).await;
    match &result {
        Ok(_) => info!(assignment_id = %id, "remove_team_assignment succeeded"),
        Err(err) => error!(assignment_id = %id, error = %err, "remove_team_assignment failed"),
    }
    result
}

#[tauri::command]
pub async fn list_skills(state: State<'_, AppState>) -> Result<Vec<Skill>, AppError> {
    debug!("list_skills requested");
    let result = agent_repo::list_skills(&state.db).await;
    if let Err(err) = &result {
        error!(error = %err, "list_skills failed");
    }
    result
}

#[tauri::command]
pub async fn create_skill(
    state: State<'_, AppState>,
    name: String,
    category: String,
    description: String,
    instructions: String,
    enabled: bool,
) -> Result<Skill, AppError> {
    info!(skill_name = %name, category = %category, "create_skill requested");
    let id = uuid::Uuid::new_v4().to_string();
    let result = agent_repo::create_skill(
        &state.db,
        &id,
        &name,
        &category,
        &description,
        &instructions,
        enabled,
    )
    .await;
    match &result {
        Ok(skill) => info!(skill_id = %skill.id, "create_skill succeeded"),
        Err(err) => error!(skill_id = %id, error = %err, "create_skill failed"),
    }
    result
}

#[tauri::command]
pub async fn update_skill(
    state: State<'_, AppState>,
    id: String,
    name: Option<String>,
    category: Option<String>,
    description: Option<String>,
    instructions: Option<String>,
    enabled: Option<bool>,
) -> Result<Skill, AppError> {
    info!(skill_id = %id, "update_skill requested");
    let result = agent_repo::update_skill(
        &state.db,
        &id,
        name.as_deref(),
        category.as_deref(),
        description.as_deref(),
        instructions.as_deref(),
        enabled,
    )
    .await;
    match &result {
        Ok(_) => info!(skill_id = %id, "update_skill succeeded"),
        Err(err) => error!(skill_id = %id, error = %err, "update_skill failed"),
    }
    result
}

#[tauri::command]
pub async fn delete_skill(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    info!(skill_id = %id, "delete_skill requested");
    let result = agent_repo::delete_skill(&state.db, &id).await;
    match &result {
        Ok(_) => info!(skill_id = %id, "delete_skill succeeded"),
        Err(err) => error!(skill_id = %id, error = %err, "delete_skill failed"),
    }
    result
}

#[tauri::command]
pub async fn list_agent_skill_links(
    state: State<'_, AppState>,
) -> Result<Vec<AgentSkillLink>, AppError> {
    debug!("list_agent_skill_links requested");
    let result = agent_repo::list_agent_skill_links(&state.db).await;
    if let Err(err) = &result {
        error!(error = %err, "list_agent_skill_links failed");
    }
    result
}

#[tauri::command]
pub async fn link_skill_to_agent(
    state: State<'_, AppState>,
    agent_id: String,
    skill_id: String,
    proficiency: String,
) -> Result<AgentSkillLink, AppError> {
    info!(agent_id = %agent_id, skill_id = %skill_id, proficiency = %proficiency, "link_skill_to_agent requested");
    let id = uuid::Uuid::new_v4().to_string();
    let result =
        agent_repo::link_skill_to_agent(&state.db, &id, &agent_id, &skill_id, &proficiency).await;
    match &result {
        Ok(link) => info!(link_id = %link.id, "link_skill_to_agent succeeded"),
        Err(err) => {
            error!(link_id = %id, agent_id = %agent_id, skill_id = %skill_id, error = %err, "link_skill_to_agent failed")
        }
    }
    result
}

#[tauri::command]
pub async fn unlink_skill_from_agent(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), AppError> {
    info!(link_id = %id, "unlink_skill_from_agent requested");
    let result = agent_repo::unlink_skill_from_agent(&state.db, &id).await;
    match &result {
        Ok(_) => info!(link_id = %id, "unlink_skill_from_agent succeeded"),
        Err(err) => error!(link_id = %id, error = %err, "unlink_skill_from_agent failed"),
    }
    result
}

#[tauri::command]
pub async fn list_team_skill_links(
    state: State<'_, AppState>,
) -> Result<Vec<TeamSkillLink>, AppError> {
    debug!("list_team_skill_links requested");
    let result = agent_repo::list_team_skill_links(&state.db).await;
    if let Err(err) = &result {
        error!(error = %err, "list_team_skill_links failed");
    }
    result
}

#[tauri::command]
pub async fn link_skill_to_team(
    state: State<'_, AppState>,
    team_id: String,
    skill_id: String,
) -> Result<TeamSkillLink, AppError> {
    info!(team_id = %team_id, skill_id = %skill_id, "link_skill_to_team requested");
    let id = uuid::Uuid::new_v4().to_string();
    let result = agent_repo::link_skill_to_team(&state.db, &id, &team_id, &skill_id).await;
    match &result {
        Ok(link) => info!(link_id = %link.id, "link_skill_to_team succeeded"),
        Err(err) => {
            error!(link_id = %id, team_id = %team_id, skill_id = %skill_id, error = %err, "link_skill_to_team failed")
        }
    }
    result
}

#[tauri::command]
pub async fn unlink_skill_from_team(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), AppError> {
    info!(link_id = %id, "unlink_skill_from_team requested");
    let result = agent_repo::unlink_skill_from_team(&state.db, &id).await;
    match &result {
        Ok(_) => info!(link_id = %id, "unlink_skill_from_team succeeded"),
        Err(err) => error!(link_id = %id, error = %err, "unlink_skill_from_team failed"),
    }
    result
}

#[tauri::command]
pub async fn list_workflow_stage_policies(
    state: State<'_, AppState>,
) -> Result<Vec<WorkflowStagePolicy>, AppError> {
    debug!("list_workflow_stage_policies requested");
    let result = agent_repo::list_workflow_stage_policies(&state.db).await;
    if let Err(err) = &result {
        error!(error = %err, "list_workflow_stage_policies failed");
    }
    result
}

#[tauri::command]
pub async fn upsert_workflow_stage_policy(
    state: State<'_, AppState>,
    stage_name: String,
    primary_roles: String,
    fallback_roles: String,
    coordinator_required: bool,
) -> Result<WorkflowStagePolicy, AppError> {
    validate_json_array("primary_roles", &primary_roles)?;
    validate_json_array("fallback_roles", &fallback_roles)?;
    let id = uuid::Uuid::new_v4().to_string();
    let result = agent_repo::upsert_workflow_stage_policy(
        &state.db,
        &id,
        &stage_name,
        &primary_roles,
        &fallback_roles,
        coordinator_required,
    )
    .await;
    match &result {
        Ok(policy) => {
            info!(policy_stage = %policy.stage_name, "upsert_workflow_stage_policy succeeded")
        }
        Err(err) => {
            error!(policy_stage = %stage_name, error = %err, "upsert_workflow_stage_policy failed")
        }
    }
    result
}

#[tauri::command]
pub async fn delete_workflow_stage_policy(
    state: State<'_, AppState>,
    stage_name: String,
) -> Result<(), AppError> {
    info!(policy_stage = %stage_name, "delete_workflow_stage_policy requested");
    let result = agent_repo::delete_workflow_stage_policy(&state.db, &stage_name).await;
    match &result {
        Ok(_) => info!(policy_stage = %stage_name, "delete_workflow_stage_policy succeeded"),
        Err(err) => {
            error!(policy_stage = %stage_name, error = %err, "delete_workflow_stage_policy failed")
        }
    }
    result
}
