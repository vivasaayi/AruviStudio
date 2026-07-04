use crate::domain::agent::{
    AgentDefinition, AgentModelBinding, AgentSkillLink, Skill, TeamSkillLink, WorkflowStagePolicy,
};
use crate::error::AppError;
use crate::persistence::agent_repo;
use crate::state::AppState;
use tauri::State;
use tracing::{debug, error, info};

mod payloads;
pub mod team;

use payloads::{validate_json_array, validate_json_object};
pub use payloads::{CreateAgentDefinitionCommand, UpdateAgentDefinitionCommand};

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
    request: CreateAgentDefinitionCommand,
) -> Result<AgentDefinition, AppError> {
    info!(agent_name = %request.name, role = %request.role, "create_agent_definition requested");
    validate_json_array("allowed_tools", &request.allowed_tools)?;
    validate_json_array("skill_tags", &request.skill_tags)?;
    validate_json_object("boundaries", &request.boundaries)?;
    let id = uuid::Uuid::new_v4().to_string();
    let result = agent_repo::create_agent_definition(
        &state.db,
        agent_repo::CreateAgentDefinitionInput {
            id: &id,
            name: &request.name,
            role: &request.role,
            description: &request.description,
            prompt_template_ref: &request.prompt_template_ref,
            allowed_tools: &request.allowed_tools,
            skill_tags: &request.skill_tags,
            boundaries: &request.boundaries,
            enabled: request.enabled,
            employment_status: &request.employment_status,
        },
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
    request: UpdateAgentDefinitionCommand,
) -> Result<AgentDefinition, AppError> {
    info!(agent_id = %request.id, "update_agent_definition requested");
    if let Some(value) = request.allowed_tools.as_deref() {
        validate_json_array("allowed_tools", value)?;
    }
    if let Some(value) = request.skill_tags.as_deref() {
        validate_json_array("skill_tags", value)?;
    }
    if let Some(value) = request.boundaries.as_deref() {
        validate_json_object("boundaries", value)?;
    }
    debug!(
        agent_id = %request.id,
        has_name = request.name.is_some(),
        has_role = request.role.is_some(),
        has_description = request.description.is_some(),
        has_prompt_template_ref = request.prompt_template_ref.is_some(),
        has_allowed_tools = request.allowed_tools.is_some(),
        has_skill_tags = request.skill_tags.is_some(),
        has_boundaries = request.boundaries.is_some(),
        has_enabled = request.enabled.is_some(),
        has_employment_status = request.employment_status.is_some(),
        "update_agent_definition payload summary"
    );
    let result = agent_repo::update_agent_definition(
        &state.db,
        agent_repo::UpdateAgentDefinitionPatch {
            id: &request.id,
            name: request.name.as_deref(),
            role: request.role.as_deref(),
            description: request.description.as_deref(),
            prompt_template_ref: request.prompt_template_ref.as_deref(),
            allowed_tools: request.allowed_tools.as_deref(),
            skill_tags: request.skill_tags.as_deref(),
            boundaries: request.boundaries.as_deref(),
            enabled: request.enabled,
            employment_status: request.employment_status.as_deref(),
        },
    )
    .await;
    match &result {
        Ok(_) => info!(agent_id = %request.id, "update_agent_definition succeeded"),
        Err(err) => error!(agent_id = %request.id, error = %err, "update_agent_definition failed"),
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
