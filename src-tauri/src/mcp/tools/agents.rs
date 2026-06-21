use crate::error::AppError;
use crate::persistence::agent_repo;
use crate::state::AppState;
use serde_json::Value;

use super::action_args::ToolAction;
use super::{action_ok, action_result};

pub(super) async fn handle(state: &AppState, payload: Value) -> Result<Value, AppError> {
    let tool_action = ToolAction::parse(payload)?;
    let args = tool_action.args();

    match tool_action.action.as_str() {
        "list_agent_definitions" => action_result(
            "list_agent_definitions",
            agent_repo::list_agent_definitions(&state.db).await?,
        ),
        "list_agent_model_bindings" => action_result(
            "list_agent_model_bindings",
            agent_repo::list_agent_model_bindings(&state.db).await?,
        ),
        "set_primary_agent_model_binding" => {
            let agent_id = args.required_string(&["agent_id", "agentId"], "agent_id")?;
            let model_id = args.required_string(&["model_id", "modelId"], "model_id")?;
            agent_repo::delete_agent_model_bindings_for_agent(&state.db, &agent_id).await?;
            let binding = agent_repo::create_agent_model_binding(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &agent_id,
                &model_id,
                1,
            )
            .await?;
            action_result("set_primary_agent_model_binding", binding)
        }
        "create_agent_definition" => {
            let agent = agent_repo::create_agent_definition(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &args.required_string(&["name"], "name")?,
                &args.required_string(&["role"], "role")?,
                &args.string_or_default(&["description"], "")?,
                &args.string_or_default(&["prompt_template_ref", "promptTemplateRef"], "")?,
                &args
                    .optional_json_array_string(&["allowed_tools", "allowedTools"])?
                    .unwrap_or_else(|| "[]".to_string()),
                &args
                    .optional_json_array_string(&["skill_tags", "skillTags"])?
                    .unwrap_or_else(|| "[]".to_string()),
                &args
                    .optional_json_object_string(&["boundaries"])?
                    .unwrap_or_else(|| "{}".to_string()),
                args.bool_or_default(&["enabled"], true)?,
                &args.string_or_default(&["employment_status", "employmentStatus"], "active")?,
            )
            .await?;
            action_result("create_agent_definition", agent)
        }
        "update_agent_definition" => {
            let id = args.required_string(&["id"], "id")?;
            let agent = agent_repo::update_agent_definition(
                &state.db,
                &id,
                args.optional_string(&["name"])?.as_deref(),
                args.optional_string(&["role"])?.as_deref(),
                args.optional_string(&["description"])?.as_deref(),
                args.optional_string(&["prompt_template_ref", "promptTemplateRef"])?
                    .as_deref(),
                args.optional_json_array_string(&["allowed_tools", "allowedTools"])?
                    .as_deref(),
                args.optional_json_array_string(&["skill_tags", "skillTags"])?
                    .as_deref(),
                args.optional_json_object_string(&["boundaries"])?
                    .as_deref(),
                args.optional_bool(&["enabled"])?,
                args.optional_string(&["employment_status", "employmentStatus"])?
                    .as_deref(),
            )
            .await?;
            action_result("update_agent_definition", agent)
        }
        "delete_agent_definition" => {
            let id = args.required_string(&["id"], "id")?;
            agent_repo::delete_agent_definition(&state.db, &id).await?;
            Ok(action_ok("delete_agent_definition"))
        }
        "list_agent_teams" => action_result(
            "list_agent_teams",
            agent_repo::list_agent_teams(&state.db).await?,
        ),
        "create_agent_team" => {
            let team = agent_repo::create_agent_team(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &args.required_string(&["name"], "name")?,
                &args.string_or_default(&["department"], "")?,
                &args.string_or_default(&["description"], "")?,
                args.bool_or_default(&["enabled"], true)?,
                args.optional_i32(&["max_concurrent_workflows", "maxConcurrentWorkflows"])?
                    .unwrap_or(2),
            )
            .await?;
            action_result("create_agent_team", team)
        }
        "update_agent_team" => {
            let id = args.required_string(&["id"], "id")?;
            let team = agent_repo::update_agent_team(
                &state.db,
                &id,
                args.optional_string(&["name"])?.as_deref(),
                args.optional_string(&["department"])?.as_deref(),
                args.optional_string(&["description"])?.as_deref(),
                args.optional_bool(&["enabled"])?,
                args.optional_i32(&["max_concurrent_workflows", "maxConcurrentWorkflows"])?,
            )
            .await?;
            action_result("update_agent_team", team)
        }
        "delete_agent_team" => {
            let id = args.required_string(&["id"], "id")?;
            agent_repo::delete_agent_team(&state.db, &id).await?;
            Ok(action_ok("delete_agent_team"))
        }
        "list_team_memberships" => action_result(
            "list_team_memberships",
            agent_repo::list_team_memberships(&state.db).await?,
        ),
        "add_team_member" => {
            let membership = agent_repo::add_team_member(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &args.required_string(&["team_id", "teamId"], "team_id")?,
                &args.required_string(&["agent_id", "agentId"], "agent_id")?,
                &args.string_or_default(&["title"], "")?,
                args.bool_or_default(&["is_lead", "isLead"], false)?,
            )
            .await?;
            action_result("add_team_member", membership)
        }
        "remove_team_member" => {
            let id = args.required_string(&["id"], "id")?;
            agent_repo::remove_team_member(&state.db, &id).await?;
            Ok(action_ok("remove_team_member"))
        }
        "list_team_assignments" => action_result(
            "list_team_assignments",
            agent_repo::list_team_assignments(&state.db).await?,
        ),
        "assign_team_scope" => {
            let assignment = agent_repo::assign_team_scope(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &args.required_string(&["team_id", "teamId"], "team_id")?,
                &args.required_string(&["scope_type", "scopeType"], "scope_type")?,
                &args.required_string(&["scope_id", "scopeId"], "scope_id")?,
            )
            .await?;
            action_result("assign_team_scope", assignment)
        }
        "remove_team_assignment" => {
            let id = args.required_string(&["id"], "id")?;
            agent_repo::remove_team_assignment(&state.db, &id).await?;
            Ok(action_ok("remove_team_assignment"))
        }
        "list_skills" => action_result("list_skills", agent_repo::list_skills(&state.db).await?),
        "create_skill" => {
            let skill = agent_repo::create_skill(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &args.required_string(&["name"], "name")?,
                &args.string_or_default(&["category"], "")?,
                &args.string_or_default(&["description"], "")?,
                &args.string_or_default(&["instructions"], "")?,
                args.bool_or_default(&["enabled"], true)?,
            )
            .await?;
            action_result("create_skill", skill)
        }
        "update_skill" => {
            let id = args.required_string(&["id"], "id")?;
            let skill = agent_repo::update_skill(
                &state.db,
                &id,
                args.optional_string(&["name"])?.as_deref(),
                args.optional_string(&["category"])?.as_deref(),
                args.optional_string(&["description"])?.as_deref(),
                args.optional_string(&["instructions"])?.as_deref(),
                args.optional_bool(&["enabled"])?,
            )
            .await?;
            action_result("update_skill", skill)
        }
        "delete_skill" => {
            let id = args.required_string(&["id"], "id")?;
            agent_repo::delete_skill(&state.db, &id).await?;
            Ok(action_ok("delete_skill"))
        }
        "list_agent_skill_links" => action_result(
            "list_agent_skill_links",
            agent_repo::list_agent_skill_links(&state.db).await?,
        ),
        "link_skill_to_agent" => {
            let link = agent_repo::link_skill_to_agent(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &args.required_string(&["agent_id", "agentId"], "agent_id")?,
                &args.required_string(&["skill_id", "skillId"], "skill_id")?,
                &args.string_or_default(&["proficiency"], "working")?,
            )
            .await?;
            action_result("link_skill_to_agent", link)
        }
        "unlink_skill_from_agent" => {
            let id = args.required_string(&["id"], "id")?;
            agent_repo::unlink_skill_from_agent(&state.db, &id).await?;
            Ok(action_ok("unlink_skill_from_agent"))
        }
        "list_team_skill_links" => action_result(
            "list_team_skill_links",
            agent_repo::list_team_skill_links(&state.db).await?,
        ),
        "link_skill_to_team" => {
            let link = agent_repo::link_skill_to_team(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &args.required_string(&["team_id", "teamId"], "team_id")?,
                &args.required_string(&["skill_id", "skillId"], "skill_id")?,
            )
            .await?;
            action_result("link_skill_to_team", link)
        }
        "unlink_skill_from_team" => {
            let id = args.required_string(&["id"], "id")?;
            agent_repo::unlink_skill_from_team(&state.db, &id).await?;
            Ok(action_ok("unlink_skill_from_team"))
        }
        "list_workflow_stage_policies" => action_result(
            "list_workflow_stage_policies",
            agent_repo::list_workflow_stage_policies(&state.db).await?,
        ),
        "upsert_workflow_stage_policy" => {
            let primary_roles = args
                .optional_json_array_string(&["primary_roles", "primaryRoles"])?
                .unwrap_or_else(|| "[]".to_string());
            let fallback_roles = args
                .optional_json_array_string(&["fallback_roles", "fallbackRoles"])?
                .unwrap_or_else(|| "[]".to_string());
            let policy = agent_repo::upsert_workflow_stage_policy(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &args.required_string(&["stage_name", "stageName"], "stage_name")?,
                &primary_roles,
                &fallback_roles,
                args.bool_or_default(&["coordinator_required", "coordinatorRequired"], false)?,
            )
            .await?;
            action_result("upsert_workflow_stage_policy", policy)
        }
        "delete_workflow_stage_policy" => {
            let stage_name = args.required_string(&["stage_name", "stageName"], "stage_name")?;
            agent_repo::delete_workflow_stage_policy(&state.db, &stage_name).await?;
            Ok(action_ok("delete_workflow_stage_policy"))
        }
        other => Err(AppError::Validation(format!(
            "unsupported aruvi_agents action: {other}"
        ))),
    }
}
