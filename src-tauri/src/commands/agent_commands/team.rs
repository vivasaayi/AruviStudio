use crate::domain::agent::{AgentTeam, AgentTeamMembership, TeamAssignment};
use crate::error::AppError;
use crate::persistence::agent_repo;
use crate::state::AppState;
use tauri::State;
use tracing::{debug, error, info};

use super::payloads::resolve_required;

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
pub async fn create_agent_team(
    state: State<'_, AppState>,
    name: String,
    department: String,
    description: String,
    enabled: bool,
    max_concurrent_workflows: Option<i32>,
) -> Result<AgentTeam, AppError> {
    let max_concurrent_workflows = max_concurrent_workflows.unwrap_or(2);
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
pub async fn update_agent_team(
    state: State<'_, AppState>,
    id: String,
    name: Option<String>,
    department: Option<String>,
    description: Option<String>,
    enabled: Option<bool>,
    max_concurrent_workflows: Option<i32>,
) -> Result<AgentTeam, AppError> {
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
pub async fn add_team_member(
    state: State<'_, AppState>,
    team_id: Option<String>,
    agent_id: Option<String>,
    title: String,
    is_lead: Option<bool>,
) -> Result<AgentTeamMembership, AppError> {
    let team_id = resolve_required(team_id, "team id")?;
    let agent_id = resolve_required(agent_id, "agent id")?;
    let is_lead = is_lead.unwrap_or(false);
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
pub async fn assign_team_scope(
    state: State<'_, AppState>,
    team_id: Option<String>,
    scope_type: Option<String>,
    scope_id: Option<String>,
) -> Result<TeamAssignment, AppError> {
    let team_id = resolve_required(team_id, "team id")?;
    let scope_type = resolve_required(scope_type, "scope type")?;
    let scope_id = resolve_required(scope_id, "scope id")?;
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
