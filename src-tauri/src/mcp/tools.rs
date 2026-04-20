use crate::commands::model_commands::upsert_local_runtime_registration;
use crate::commands::repository_commands::create_local_workspace_for_scope;
use crate::commands::settings_commands::{DatabaseHealth, MigrationStatus};
use crate::domain::model::{ModelDefinition, ProviderType};
use crate::domain::workflow::UserAction;
use crate::error::AppError;
use crate::persistence::{
    agent_repo, approval_repo, artifact_repo, finding_repo, model_repo, observability_repo,
    product_repo, repository_repo, settings_repo, work_item_repo, workflow_repo,
};
use crate::providers::gateway::ModelGateway;
use crate::providers::openai_compatible::OpenAiCompatibleProvider;
use crate::providers::types::CompletionRequest;
use crate::secrets;
use crate::services::channel_service::{self, PlannerContactRequest};
use crate::services::planner_service::{
    add_planner_draft_child, analyze_repository_for_planner, clear_planner_pending,
    confirm_planner_plan, create_planner_session, delete_planner_draft_node,
    rename_planner_draft_node, submit_planner_turn, submit_planner_voice_turn,
    update_planner_session,
};
use crate::services::product_service::{self, HIDE_EXAMPLE_PRODUCTS_KEY};
use crate::services::speech_service::{
    looks_like_transcription_model, speak_text_natively, transcribe_audio_with_provider,
    TextToSpeechRequest,
};
use crate::services::webhook_service;
use crate::state::AppState;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Map, Value};
use sqlx::Row;
use tracing::error;

const AUTO_START_AFTER_WORK_ITEM_APPROVAL_KEY: &str =
    "workflow.auto_start_after_work_item_approval";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

pub fn definitions() -> Vec<ToolDefinition> {
    vec![
        action_tool(
            "aruvi_catalog",
            "Manage products, modules, capabilities, and capability rollouts (child capabilities) for planning.",
            &[
                "create_product",
                "get_product",
                "list_products",
                "seed_example_products",
                "update_product",
                "archive_product",
                "create_module",
                "list_modules",
                "update_module",
                "delete_module",
                "reorder_modules",
                "create_capability",
                "list_capabilities",
                "update_capability",
                "delete_capability",
                "reorder_capabilities",
                "get_product_tree",
            ],
        ),
        action_tool(
            "aruvi_work_items",
            "Manage work items and their hierarchy for execution planning. Work items attach directly to capability_id, including capability rollout nodes.",
            &[
                "create_work_item",
                "get_work_item",
                "list_work_items",
                "summarize_work_items_by_product",
                "update_work_item",
                "delete_work_item",
                "get_sub_work_items",
                "reorder_work_items",
            ],
        ),
        action_tool(
            "aruvi_repositories",
            "Register repositories, attach them to scope, create workspaces, and edit files safely.",
            &[
                "register_repository",
                "list_repositories",
                "delete_repository",
                "attach_repository",
                "resolve_repository_for_work_item",
                "resolve_repository_for_scope",
                "create_local_workspace",
                "list_repository_tree",
                "read_repository_file",
                "write_repository_file",
                "get_repository_file_sha256",
                "apply_repository_patch",
            ],
        ),
        action_tool(
            "aruvi_planner",
            "Drive planner sessions, draft trees, repository analysis, and plan confirmation.",
            &[
                "create_planner_session",
                "update_planner_session",
                "clear_planner_pending",
                "submit_planner_turn",
                "submit_planner_voice_turn",
                "confirm_planner_plan",
                "rename_planner_draft_node",
                "add_planner_draft_child",
                "delete_planner_draft_node",
                "analyze_repository_for_planner",
            ],
        ),
        action_tool(
            "aruvi_workflows",
            "Control workflow execution and inspect workflow/agent run state.",
            &[
                "start_work_item_workflow",
                "get_workflow_run",
                "get_latest_workflow_run_for_work_item",
                "get_workflow_history",
                "handle_workflow_user_action",
                "advance_workflow",
                "list_agent_runs_for_workflow",
                "mark_workflow_run_failed",
                "restart_workflow_run",
            ],
        ),
        action_tool(
            "aruvi_checkpoints",
            "Handle approvals, artifacts, findings, and logs for checkpointing and review.",
            &[
                "approve_work_item",
                "reject_work_item",
                "approve_work_item_plan",
                "reject_work_item_plan",
                "approve_work_item_test_review",
                "get_work_item_approvals",
                "list_work_item_artifacts",
                "read_artifact_content",
                "list_work_item_findings",
                "get_logs",
            ],
        ),
        action_tool(
            "aruvi_agents",
            "Manage agents, teams, skills, model bindings, and workflow stage routing policies.",
            &[
                "list_agent_definitions",
                "list_agent_model_bindings",
                "set_primary_agent_model_binding",
                "create_agent_definition",
                "update_agent_definition",
                "delete_agent_definition",
                "list_agent_teams",
                "create_agent_team",
                "update_agent_team",
                "delete_agent_team",
                "list_team_memberships",
                "add_team_member",
                "remove_team_member",
                "list_team_assignments",
                "assign_team_scope",
                "remove_team_assignment",
                "list_skills",
                "create_skill",
                "update_skill",
                "delete_skill",
                "list_agent_skill_links",
                "link_skill_to_agent",
                "unlink_skill_from_agent",
                "list_team_skill_links",
                "link_skill_to_team",
                "unlink_skill_from_team",
                "list_workflow_stage_policies",
                "upsert_workflow_stage_policy",
                "delete_workflow_stage_policy",
            ],
        ),
        action_tool(
            "aruvi_models",
            "Manage providers and models, test connectivity, register local runtimes, and run chat completion.",
            &[
                "create_provider",
                "list_providers",
                "update_provider",
                "delete_provider",
                "create_model_definition",
                "list_model_definitions",
                "update_model_definition",
                "delete_model_definition",
                "test_provider_connectivity",
                "register_local_runtime_model",
                "install_managed_local_model",
                "run_model_chat_completion",
            ],
        ),
        action_tool(
            "aruvi_settings",
            "Inspect and update operational settings, mobile and MCP bridge status, and database configuration.",
            &[
                "get_setting",
                "set_setting",
                "get_mobile_bridge_status",
                "get_mcp_bridge_status",
                "get_database_health",
                "get_active_database_path",
                "get_database_path_override",
                "set_database_path_override",
                "clear_database_path_override",
            ],
        ),
        action_tool(
            "aruvi_channels",
            "Use Twilio-backed outbound channels and planner contact routing.",
            &[
                "send_twilio_whatsapp_message",
                "start_twilio_voice_call",
                "route_planner_contact",
            ],
        ),
        action_tool(
            "aruvi_speech",
            "Transcribe audio and trigger native speech output for voice-driven planning flows.",
            &["transcribe_audio", "speak_text_natively"],
        ),
    ]
}

pub async fn dispatch_tool(
    state: &AppState,
    tool_name: &str,
    payload: Value,
) -> Result<Value, AppError> {
    match tool_name {
        "aruvi_catalog" => handle_catalog(state, payload).await,
        "aruvi_work_items" => handle_work_items(state, payload).await,
        "aruvi_repositories" => handle_repositories(state, payload).await,
        "aruvi_planner" => handle_planner(state, payload).await,
        "aruvi_workflows" => handle_workflows(state, payload).await,
        "aruvi_checkpoints" => handle_checkpoints(state, payload).await,
        "aruvi_agents" => handle_agents(state, payload).await,
        "aruvi_models" => handle_models(state, payload).await,
        "aruvi_settings" => handle_settings(state, payload).await,
        "aruvi_channels" => handle_channels(state, payload).await,
        "aruvi_speech" => handle_speech(state, payload).await,
        _ => Err(AppError::Validation(format!(
            "Unknown MCP tool: {tool_name}"
        ))),
    }
}

fn action_tool(name: &str, description: &str, actions: &[&str]) -> ToolDefinition {
    ToolDefinition {
        name: name.to_string(),
        description: description.to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": actions,
                    "description": "The operation to perform with this tool."
                },
                "arguments": {
                    "type": "object",
                    "description": "Action-specific arguments. Use snake_case or camelCase keys."
                }
            },
            "required": ["action"],
            "additionalProperties": false
        }),
    }
}

struct ToolAction {
    action: String,
    arguments: Value,
}

impl ToolAction {
    fn parse(payload: Value) -> Result<Self, AppError> {
        let object = payload.as_object().ok_or_else(|| {
            AppError::Validation("tool payload must be a JSON object".to_string())
        })?;
        let root = ActionArgs { object };
        let action = root.required_string(&["action"], "action")?;
        let arguments = match object.get("arguments") {
            Some(Value::Object(map)) => Value::Object(map.clone()),
            Some(Value::Null) | None => Value::Object(Map::new()),
            Some(_) => {
                return Err(AppError::Validation(
                    "tool payload arguments must be a JSON object".to_string(),
                ))
            }
        };

        Ok(Self { action, arguments })
    }

    fn args(&self) -> ActionArgs<'_> {
        ActionArgs {
            object: self
                .arguments
                .as_object()
                .expect("arguments must be object"),
        }
    }
}

struct ActionArgs<'a> {
    object: &'a Map<String, Value>,
}

impl<'a> ActionArgs<'a> {
    fn get(&self, keys: &[&str]) -> Option<&Value> {
        for key in keys {
            if let Some(value) = self.object.get(*key) {
                return Some(value);
            }
        }
        None
    }

    fn required_string(&self, keys: &[&str], label: &str) -> Result<String, AppError> {
        self.optional_string(keys)?
            .ok_or_else(|| AppError::Validation(format!("missing {label}")))
    }

    fn optional_string(&self, keys: &[&str]) -> Result<Option<String>, AppError> {
        match self.get(keys) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::String(value)) => Ok(Some(value.to_string())),
            Some(Value::Number(value)) => Ok(Some(value.to_string())),
            Some(Value::Bool(value)) => Ok(Some(value.to_string())),
            Some(_) => Err(AppError::Validation(format!(
                "{} must be a string",
                keys[0]
            ))),
        }
    }

    fn string_or_default(&self, keys: &[&str], default: &str) -> Result<String, AppError> {
        Ok(self
            .optional_string(keys)?
            .unwrap_or_else(|| default.to_string()))
    }

    fn optional_bool(&self, keys: &[&str]) -> Result<Option<bool>, AppError> {
        match self.get(keys) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::Bool(value)) => Ok(Some(*value)),
            Some(Value::String(value)) => match value.trim().to_ascii_lowercase().as_str() {
                "true" => Ok(Some(true)),
                "false" => Ok(Some(false)),
                _ => Err(AppError::Validation(format!(
                    "{} must be a boolean",
                    keys[0]
                ))),
            },
            Some(_) => Err(AppError::Validation(format!(
                "{} must be a boolean",
                keys[0]
            ))),
        }
    }

    fn bool_or_default(&self, keys: &[&str], default: bool) -> Result<bool, AppError> {
        Ok(self.optional_bool(keys)?.unwrap_or(default))
    }

    fn optional_i64(&self, keys: &[&str]) -> Result<Option<i64>, AppError> {
        match self.get(keys) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::Number(value)) => value
                .as_i64()
                .ok_or_else(|| AppError::Validation(format!("{} must be an integer", keys[0])))
                .map(Some),
            Some(Value::String(value)) => value
                .trim()
                .parse::<i64>()
                .map(Some)
                .map_err(|_| AppError::Validation(format!("{} must be an integer", keys[0]))),
            Some(_) => Err(AppError::Validation(format!(
                "{} must be an integer",
                keys[0]
            ))),
        }
    }

    fn optional_i32(&self, keys: &[&str]) -> Result<Option<i32>, AppError> {
        self.optional_i64(keys)?
            .map(|value| {
                i32::try_from(value).map_err(|_| {
                    AppError::Validation(format!("{} is out of range for i32", keys[0]))
                })
            })
            .transpose()
    }

    fn optional_f64(&self, keys: &[&str]) -> Result<Option<f64>, AppError> {
        match self.get(keys) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::Number(value)) => value
                .as_f64()
                .ok_or_else(|| AppError::Validation(format!("{} must be a number", keys[0])))
                .map(Some),
            Some(Value::String(value)) => value
                .trim()
                .parse::<f64>()
                .map(Some)
                .map_err(|_| AppError::Validation(format!("{} must be a number", keys[0]))),
            Some(_) => Err(AppError::Validation(format!(
                "{} must be a number",
                keys[0]
            ))),
        }
    }

    fn required_string_list(&self, keys: &[&str], label: &str) -> Result<Vec<String>, AppError> {
        self.optional_string_list(keys)?
            .ok_or_else(|| AppError::Validation(format!("missing {label}")))
    }

    fn optional_string_list(&self, keys: &[&str]) -> Result<Option<Vec<String>>, AppError> {
        match self.get(keys) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::Array(values)) => values
                .iter()
                .map(|value| {
                    value
                        .as_str()
                        .map(|value| value.to_string())
                        .ok_or_else(|| {
                            AppError::Validation(format!("{} must contain only strings", keys[0]))
                        })
                })
                .collect::<Result<Vec<_>, _>>()
                .map(Some),
            Some(Value::String(value)) => {
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    return Ok(Some(Vec::new()));
                }
                if trimmed.starts_with('[') {
                    let parsed = serde_json::from_str::<Vec<String>>(trimmed)?;
                    Ok(Some(parsed))
                } else {
                    Ok(Some(
                        trimmed
                            .split(',')
                            .map(str::trim)
                            .filter(|item| !item.is_empty())
                            .map(ToString::to_string)
                            .collect(),
                    ))
                }
            }
            Some(_) => Err(AppError::Validation(format!(
                "{} must be an array of strings",
                keys[0]
            ))),
        }
    }

    fn optional_json_array_string(&self, keys: &[&str]) -> Result<Option<String>, AppError> {
        self.optional_string_list(keys)?
            .map(|value| serde_json::to_string(&value).map_err(AppError::from))
            .transpose()
    }

    fn optional_json_object_string(&self, keys: &[&str]) -> Result<Option<String>, AppError> {
        match self.get(keys) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::Object(value)) => serde_json::to_string(value)
                .map(Some)
                .map_err(AppError::from),
            Some(Value::String(value)) => {
                let parsed = serde_json::from_str::<Value>(value)?;
                if !parsed.is_object() {
                    return Err(AppError::Validation(format!(
                        "{} must be a JSON object",
                        keys[0]
                    )));
                }
                serde_json::to_string(&parsed)
                    .map(Some)
                    .map_err(AppError::from)
            }
            Some(_) => Err(AppError::Validation(format!(
                "{} must be a JSON object",
                keys[0]
            ))),
        }
    }

    fn optional_deserialize<T: DeserializeOwned>(
        &self,
        keys: &[&str],
        label: &str,
    ) -> Result<Option<T>, AppError> {
        match self.get(keys) {
            None | Some(Value::Null) => Ok(None),
            Some(value) => serde_json::from_value::<T>(value.clone())
                .map(Some)
                .map_err(|error| AppError::Validation(format!("invalid {label}: {error}"))),
        }
    }

    fn required_deserialize<T: DeserializeOwned>(
        &self,
        keys: &[&str],
        label: &str,
    ) -> Result<T, AppError> {
        self.optional_deserialize(keys, label)?
            .ok_or_else(|| AppError::Validation(format!("missing {label}")))
    }
}

fn action_result<T: Serialize>(action: &str, result: T) -> Result<Value, AppError> {
    Ok(json!({
        "action": action,
        "result": serde_json::to_value(result)?
    }))
}

fn action_ok(action: &str) -> Value {
    json!({
        "action": action,
        "result": { "ok": true }
    })
}

async fn handle_catalog(state: &AppState, payload: Value) -> Result<Value, AppError> {
    let tool_action = ToolAction::parse(payload)?;
    let args = tool_action.args();

    match tool_action.action.as_str() {
        "create_product" => {
            let name = args.required_string(&["name"], "name")?;
            let description = args.string_or_default(&["description"], "")?;
            let vision = args.string_or_default(&["vision"], "")?;
            let goals = args.optional_string_list(&["goals"])?.unwrap_or_default();
            let tags = args.optional_string_list(&["tags"])?.unwrap_or_default();
            let product = product_repo::create_product(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &name,
                &description,
                &vision,
                &serde_json::to_string(&goals)?,
                &serde_json::to_string(&tags)?,
            )
            .await?;
            action_result("create_product", product)
        }
        "get_product" => {
            let id = args.required_string(&["id"], "id")?;
            action_result(
                "get_product",
                product_repo::get_product(&state.db, &id).await?,
            )
        }
        "list_products" => {
            let hide_examples =
                settings_repo::get_bool_setting(&state.db, HIDE_EXAMPLE_PRODUCTS_KEY, true).await?;
            let mut products = product_repo::list_products(&state.db).await?;
            if hide_examples {
                products.retain(|product| !product.is_example_product());
            }
            action_result("list_products", products)
        }
        "seed_example_products" => {
            product_service::initialize_example_catalog(&state.db).await?;
            Ok(action_ok("seed_example_products"))
        }
        "update_product" => {
            let id = args.required_string(&["id"], "id")?;
            let goals = args.optional_json_array_string(&["goals"])?;
            let tags = args.optional_json_array_string(&["tags"])?;
            let product = product_repo::update_product(
                &state.db,
                &id,
                args.optional_string(&["name"])?.as_deref(),
                args.optional_string(&["description"])?.as_deref(),
                args.optional_string(&["vision"])?.as_deref(),
                goals.as_deref(),
                tags.as_deref(),
            )
            .await?;
            action_result("update_product", product)
        }
        "archive_product" => {
            let id = args.required_string(&["id"], "id")?;
            action_result(
                "archive_product",
                product_repo::archive_product(&state.db, &id).await?,
            )
        }
        "create_module" => {
            let product_id = args.required_string(&["product_id", "productId"], "product_id")?;
            let name = args.required_string(&["name"], "name")?;
            let module = product_repo::create_module(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &product_id,
                &name,
                &args.string_or_default(&["description"], "")?,
                &args.string_or_default(&["purpose"], "")?,
            )
            .await?;
            action_result("create_module", module)
        }
        "list_modules" => {
            let product_id = args.required_string(&["product_id", "productId"], "product_id")?;
            action_result(
                "list_modules",
                product_repo::list_modules(&state.db, &product_id).await?,
            )
        }
        "update_module" => {
            let id = args.required_string(&["id"], "id")?;
            let module = product_repo::update_module(
                &state.db,
                &id,
                args.optional_string(&["name"])?.as_deref(),
                args.optional_string(&["description"])?.as_deref(),
                args.optional_string(&["purpose"])?.as_deref(),
            )
            .await?;
            action_result("update_module", module)
        }
        "delete_module" => {
            let id = args.required_string(&["id"], "id")?;
            product_repo::delete_module(&state.db, &id).await?;
            Ok(action_ok("delete_module"))
        }
        "reorder_modules" => {
            let product_id = args.required_string(&["product_id", "productId"], "product_id")?;
            let ordered_ids =
                args.required_string_list(&["ordered_ids", "orderedIds"], "ordered_ids")?;
            product_repo::reorder_modules(&state.db, &product_id, &ordered_ids).await?;
            Ok(action_ok("reorder_modules"))
        }
        "create_capability" => {
            let module_id = args.required_string(&["module_id", "moduleId"], "module_id")?;
            let name = args.required_string(&["name"], "name")?;
            let capability = product_repo::create_capability(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &module_id,
                args.optional_string(&["parent_capability_id", "parentCapabilityId"])?
                    .as_deref(),
                &name,
                &args.string_or_default(&["description"], "")?,
                &args.string_or_default(&["acceptance_criteria", "acceptanceCriteria"], "")?,
                &args.string_or_default(&["priority"], "medium")?,
                &args.string_or_default(&["risk"], "medium")?,
                &args.string_or_default(&["technical_notes", "technicalNotes"], "")?,
            )
            .await?;
            action_result("create_capability", capability)
        }
        "list_capabilities" => {
            let module_id = args.required_string(&["module_id", "moduleId"], "module_id")?;
            action_result(
                "list_capabilities",
                product_repo::list_capabilities(&state.db, &module_id).await?,
            )
        }
        "update_capability" => {
            let id = args.required_string(&["id"], "id")?;
            let capability = product_repo::update_capability(
                &state.db,
                &id,
                args.optional_string(&["name"])?.as_deref(),
                args.optional_string(&["description"])?.as_deref(),
                args.optional_string(&["acceptance_criteria", "acceptanceCriteria"])?
                    .as_deref(),
                args.optional_string(&["priority"])?.as_deref(),
                args.optional_string(&["risk"])?.as_deref(),
                args.optional_string(&["technical_notes", "technicalNotes"])?
                    .as_deref(),
            )
            .await?;
            action_result("update_capability", capability)
        }
        "delete_capability" => {
            let id = args.required_string(&["id"], "id")?;
            product_repo::delete_capability(&state.db, &id).await?;
            Ok(action_ok("delete_capability"))
        }
        "reorder_capabilities" => {
            let module_id = args.required_string(&["module_id", "moduleId"], "module_id")?;
            let parent_capability_id =
                args.optional_string(&["parent_capability_id", "parentCapabilityId"])?;
            let ordered_ids =
                args.required_string_list(&["ordered_ids", "orderedIds"], "ordered_ids")?;
            product_repo::reorder_capabilities(
                &state.db,
                &module_id,
                parent_capability_id.as_deref(),
                &ordered_ids,
            )
            .await?;
            Ok(action_ok("reorder_capabilities"))
        }
        "get_product_tree" => {
            let product_id = args.required_string(&["product_id", "productId"], "product_id")?;
            action_result(
                "get_product_tree",
                product_repo::get_product_tree(&state.db, &product_id).await?,
            )
        }
        other => Err(AppError::Validation(format!(
            "unsupported aruvi_catalog action: {other}"
        ))),
    }
}

async fn handle_work_items(state: &AppState, payload: Value) -> Result<Value, AppError> {
    let tool_action = ToolAction::parse(payload)?;
    let args = tool_action.args();

    match tool_action.action.as_str() {
        "create_work_item" => {
            let product_id = args.required_string(&["product_id", "productId"], "product_id")?;
            let title = args.required_string(&["title"], "title")?;
            let work_item = work_item_repo::create_work_item(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &product_id,
                args.optional_string(&["module_id", "moduleId"])?.as_deref(),
                args.optional_string(&["capability_id", "capabilityId"])?
                    .as_deref(),
                args.optional_string(&["parent_work_item_id", "parentWorkItemId"])?
                    .as_deref(),
                &title,
                &args.string_or_default(&["problem_statement", "problemStatement"], "")?,
                &args.string_or_default(&["description"], "")?,
                &args.string_or_default(&["acceptance_criteria", "acceptanceCriteria"], "")?,
                &args.string_or_default(&["constraints"], "")?,
                &args.string_or_default(&["work_item_type", "workItemType"], "feature")?,
                &args.string_or_default(&["priority"], "medium")?,
                &args.string_or_default(&["complexity"], "medium")?,
            )
            .await?;
            action_result("create_work_item", work_item)
        }
        "get_work_item" => {
            let id = args.required_string(&["id"], "id")?;
            action_result(
                "get_work_item",
                work_item_repo::get_work_item(&state.db, &id).await?,
            )
        }
        "list_work_items" => action_result(
            "list_work_items",
            work_item_repo::list_work_items(
                &state.db,
                args.optional_string(&["product_id", "productId"])?
                    .as_deref(),
                args.optional_string(&["module_id", "moduleId"])?.as_deref(),
                args.optional_string(&["capability_id", "capabilityId"])?
                    .as_deref(),
                args.optional_string(&["status"])?.as_deref(),
            )
            .await?,
        ),
        "summarize_work_items_by_product" => action_result(
            "summarize_work_items_by_product",
            work_item_repo::summarize_work_items_by_product(&state.db).await?,
        ),
        "update_work_item" => {
            let id = args.required_string(&["id"], "id")?;
            let work_item = work_item_repo::update_work_item(
                &state.db,
                &id,
                args.optional_string(&["title"])?.as_deref(),
                args.optional_string(&["description"])?.as_deref(),
                args.optional_string(&["status"])?.as_deref(),
                args.optional_string(&["problem_statement", "problemStatement"])?
                    .as_deref(),
                args.optional_string(&["acceptance_criteria", "acceptanceCriteria"])?
                    .as_deref(),
                args.optional_string(&["constraints"])?.as_deref(),
            )
            .await?;
            action_result("update_work_item", work_item)
        }
        "delete_work_item" => {
            let id = args.required_string(&["id"], "id")?;
            work_item_repo::delete_work_item(&state.db, &id).await?;
            Ok(action_ok("delete_work_item"))
        }
        "get_sub_work_items" => {
            let work_item_id =
                args.required_string(&["work_item_id", "workItemId"], "work_item_id")?;
            action_result(
                "get_sub_work_items",
                work_item_repo::get_sub_work_items(&state.db, &work_item_id).await?,
            )
        }
        "reorder_work_items" => {
            let ordered_ids =
                args.required_string_list(&["ordered_ids", "orderedIds"], "ordered_ids")?;
            work_item_repo::reorder_work_items(&state.db, &ordered_ids).await?;
            Ok(action_ok("reorder_work_items"))
        }
        other => Err(AppError::Validation(format!(
            "unsupported aruvi_work_items action: {other}"
        ))),
    }
}

async fn handle_repositories(state: &AppState, payload: Value) -> Result<Value, AppError> {
    let tool_action = ToolAction::parse(payload)?;
    let args = tool_action.args();

    match tool_action.action.as_str() {
        "register_repository" => {
            let repository = repository_repo::create_repository(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &args.required_string(&["name"], "name")?,
                &args.required_string(&["local_path", "localPath"], "local_path")?,
                &args.string_or_default(&["remote_url", "remoteUrl"], "")?,
                &args.string_or_default(&["default_branch", "defaultBranch"], "main")?,
            )
            .await?;
            action_result("register_repository", repository)
        }
        "list_repositories" => action_result(
            "list_repositories",
            repository_repo::list_repositories(&state.db).await?,
        ),
        "delete_repository" => {
            let id = args.required_string(&["id"], "id")?;
            repository_repo::delete_repository(&state.db, &id).await?;
            Ok(action_ok("delete_repository"))
        }
        "attach_repository" => {
            let attachment_id = uuid::Uuid::new_v4().to_string();
            repository_repo::attach_repository(
                &state.db,
                &attachment_id,
                &args.required_string(&["scope_type", "scopeType"], "scope_type")?,
                &args.required_string(&["scope_id", "scopeId"], "scope_id")?,
                &args.required_string(&["repository_id", "repositoryId"], "repository_id")?,
                args.bool_or_default(&["is_default", "isDefault"], false)?,
            )
            .await?;
            Ok(json!({
                "action": "attach_repository",
                "result": {
                    "ok": true,
                    "attachment_id": attachment_id
                }
            }))
        }
        "resolve_repository_for_work_item" => {
            let work_item_id =
                args.required_string(&["work_item_id", "workItemId"], "work_item_id")?;
            action_result(
                "resolve_repository_for_work_item",
                repository_repo::resolve_repository_for_work_item(&state.db, &work_item_id).await?,
            )
        }
        "resolve_repository_for_scope" => action_result(
            "resolve_repository_for_scope",
            repository_repo::resolve_repository_for_scope(
                &state.db,
                args.optional_string(&["product_id", "productId"])?
                    .as_deref(),
                args.optional_string(&["module_id", "moduleId"])?.as_deref(),
            )
            .await?,
        ),
        "create_local_workspace" => {
            let workspace = create_local_workspace_for_scope(
                state,
                args.optional_string(&["product_id", "productId"])?,
                args.optional_string(&["module_id", "moduleId"])?,
                args.optional_string(&["work_item_id", "workItemId"])?,
                args.optional_string(&["preferred_path", "preferredPath"])?,
            )
            .await?;
            action_result("create_local_workspace", workspace)
        }
        "list_repository_tree" => {
            let repository_id =
                args.required_string(&["repository_id", "repositoryId"], "repository_id")?;
            let repository = repository_repo::get_repository(&state.db, &repository_id).await?;
            let max_depth = args.optional_i64(&["max_depth", "maxDepth"])?;
            let tree = crate::services::repo_service::list_repository_tree(
                &repository.local_path,
                args.bool_or_default(&["include_hidden", "includeHidden"], false)?,
                max_depth.map(|value| value.clamp(1, 32) as usize),
            )?;
            action_result("list_repository_tree", tree)
        }
        "read_repository_file" => {
            let repository_id =
                args.required_string(&["repository_id", "repositoryId"], "repository_id")?;
            let repository = repository_repo::get_repository(&state.db, &repository_id).await?;
            let content = crate::services::repo_service::read_repository_file(
                &repository.local_path,
                &args.required_string(&["relative_path", "relativePath"], "relative_path")?,
            )?;
            action_result("read_repository_file", json!({ "content": content }))
        }
        "write_repository_file" => {
            let repository_id =
                args.required_string(&["repository_id", "repositoryId"], "repository_id")?;
            let repository = repository_repo::get_repository(&state.db, &repository_id).await?;
            crate::services::repo_service::write_repository_file(
                &repository.local_path,
                &args.required_string(&["relative_path", "relativePath"], "relative_path")?,
                &args.required_string(&["content"], "content")?,
            )?;
            Ok(action_ok("write_repository_file"))
        }
        "get_repository_file_sha256" => {
            let repository_id =
                args.required_string(&["repository_id", "repositoryId"], "repository_id")?;
            let repository = repository_repo::get_repository(&state.db, &repository_id).await?;
            let sha = crate::services::repo_service::get_repository_file_sha256(
                &repository.local_path,
                &args.required_string(&["relative_path", "relativePath"], "relative_path")?,
            )?;
            action_result("get_repository_file_sha256", json!({ "sha256": sha }))
        }
        "apply_repository_patch" => {
            let repository_id =
                args.required_string(&["repository_id", "repositoryId"], "repository_id")?;
            let repository = repository_repo::get_repository(&state.db, &repository_id).await?;
            let sha = crate::services::repo_service::apply_repository_patch(
                &repository.local_path,
                &args.required_string(&["relative_path", "relativePath"], "relative_path")?,
                &args.required_string(&["patch"], "patch")?,
                args.optional_string(&["base_sha256", "baseSha256"])?
                    .as_deref(),
            )?;
            action_result("apply_repository_patch", json!({ "sha256": sha }))
        }
        other => Err(AppError::Validation(format!(
            "unsupported aruvi_repositories action: {other}"
        ))),
    }
}

async fn handle_planner(state: &AppState, payload: Value) -> Result<Value, AppError> {
    let tool_action = ToolAction::parse(payload)?;
    let args = tool_action.args();

    match tool_action.action.as_str() {
        "create_planner_session" => action_result(
            "create_planner_session",
            create_planner_session(
                state.planner_service.clone(),
                &state.db,
                args.optional_string(&["provider_id", "providerId"])?,
                args.optional_string(&["model_name", "modelName"])?,
            )
            .await?,
        ),
        "update_planner_session" => action_result(
            "update_planner_session",
            update_planner_session(
                state.planner_service.clone(),
                &state.db,
                args.required_string(&["session_id", "sessionId"], "session_id")?,
                args.optional_string(&["provider_id", "providerId"])?,
                args.optional_string(&["model_name", "modelName"])?,
            )
            .await?,
        ),
        "clear_planner_pending" => action_result(
            "clear_planner_pending",
            clear_planner_pending(
                state.planner_service.clone(),
                &state.db,
                args.required_string(&["session_id", "sessionId"], "session_id")?,
            )
            .await?,
        ),
        "submit_planner_turn" => action_result(
            "submit_planner_turn",
            submit_planner_turn(
                state.planner_service.clone(),
                state,
                args.required_string(&["session_id", "sessionId"], "session_id")?,
                args.required_string(&["user_input", "userInput"], "user_input")?,
                args.optional_string(&["selected_draft_node_id", "selectedDraftNodeId"])?,
            )
            .await?,
        ),
        "submit_planner_voice_turn" => action_result(
            "submit_planner_voice_turn",
            submit_planner_voice_turn(
                state.planner_service.clone(),
                state,
                args.required_string(&["session_id", "sessionId"], "session_id")?,
                args.required_string(&["transcript", "user_input", "userInput"], "transcript")?,
                args.optional_string(&["selected_draft_node_id", "selectedDraftNodeId"])?,
            )
            .await?,
        ),
        "confirm_planner_plan" => action_result(
            "confirm_planner_plan",
            confirm_planner_plan(
                state.planner_service.clone(),
                state,
                args.required_string(&["session_id", "sessionId"], "session_id")?,
            )
            .await?,
        ),
        "rename_planner_draft_node" => action_result(
            "rename_planner_draft_node",
            rename_planner_draft_node(
                state.planner_service.clone(),
                &state.db,
                args.required_string(&["session_id", "sessionId"], "session_id")?,
                args.required_string(&["node_id", "nodeId"], "node_id")?,
                args.required_string(&["name"], "name")?,
            )
            .await?,
        ),
        "add_planner_draft_child" => action_result(
            "add_planner_draft_child",
            add_planner_draft_child(
                state.planner_service.clone(),
                &state.db,
                args.required_string(&["session_id", "sessionId"], "session_id")?,
                args.required_string(&["parent_node_id", "parentNodeId"], "parent_node_id")?,
                args.required_string(&["child_type", "childType"], "child_type")?,
                args.required_string(&["name"], "name")?,
                args.optional_string(&["summary"])?,
            )
            .await?,
        ),
        "delete_planner_draft_node" => action_result(
            "delete_planner_draft_node",
            delete_planner_draft_node(
                state.planner_service.clone(),
                &state.db,
                args.required_string(&["session_id", "sessionId"], "session_id")?,
                args.required_string(&["node_id", "nodeId"], "node_id")?,
            )
            .await?,
        ),
        "analyze_repository_for_planner" => action_result(
            "analyze_repository_for_planner",
            analyze_repository_for_planner(
                state.planner_service.clone(),
                &state.db,
                args.required_string(&["session_id", "sessionId"], "session_id")?,
                args.required_string(&["repository_id", "repositoryId"], "repository_id")?,
                args.optional_string(&["selected_draft_node_id", "selectedDraftNodeId"])?,
            )
            .await?,
        ),
        other => Err(AppError::Validation(format!(
            "unsupported aruvi_planner action: {other}"
        ))),
    }
}

async fn handle_workflows(state: &AppState, payload: Value) -> Result<Value, AppError> {
    let tool_action = ToolAction::parse(payload)?;
    let args = tool_action.args();

    match tool_action.action.as_str() {
        "start_work_item_workflow" => {
            let work_item_id =
                args.required_string(&["work_item_id", "workItemId"], "work_item_id")?;
            let workflow_service = state.workflow_service.lock().await;
            let run = workflow_service
                .start_work_item_workflow(&work_item_id)
                .await?;
            action_result(
                "start_work_item_workflow",
                json!({ "workflow_run_id": run.id }),
            )
        }
        "get_workflow_run" => {
            let workflow_run_id =
                args.required_string(&["workflow_run_id", "workflowRunId"], "workflow_run_id")?;
            let workflow_service = state.workflow_service.lock().await;
            action_result(
                "get_workflow_run",
                workflow_service.get_workflow_run(&workflow_run_id).await?,
            )
        }
        "get_latest_workflow_run_for_work_item" => {
            let work_item_id =
                args.required_string(&["work_item_id", "workItemId"], "work_item_id")?;
            action_result(
                "get_latest_workflow_run_for_work_item",
                workflow_repo::get_latest_workflow_run_for_work_item(&state.db, &work_item_id)
                    .await?,
            )
        }
        "get_workflow_history" => {
            let workflow_run_id =
                args.required_string(&["workflow_run_id", "workflowRunId"], "workflow_run_id")?;
            let workflow_service = state.workflow_service.lock().await;
            action_result(
                "get_workflow_history",
                workflow_service
                    .get_workflow_history(&workflow_run_id)
                    .await?,
            )
        }
        "handle_workflow_user_action" => {
            let workflow_run_id =
                args.required_string(&["workflow_run_id", "workflowRunId"], "workflow_run_id")?;
            let action = match args.required_string(&["action"], "action")?.as_str() {
                "approve" => UserAction::Approve,
                "reject" => UserAction::Reject,
                "pause" => UserAction::Pause,
                "resume" => UserAction::Resume,
                "cancel" => UserAction::Cancel,
                other => {
                    return Err(AppError::Validation(format!(
                        "Unsupported workflow action: {other}"
                    )))
                }
            };
            let workflow_service = state.workflow_service.lock().await;
            workflow_service
                .handle_user_action(&workflow_run_id, action, args.optional_string(&["notes"])?)
                .await?;
            Ok(action_ok("handle_workflow_user_action"))
        }
        "advance_workflow" => {
            let workflow_run_id =
                args.required_string(&["workflow_run_id", "workflowRunId"], "workflow_run_id")?;
            let workflow_service = state.workflow_service.lock().await;
            workflow_service.advance_workflow(&workflow_run_id).await?;
            Ok(action_ok("advance_workflow"))
        }
        "list_agent_runs_for_workflow" => {
            let workflow_run_id =
                args.required_string(&["workflow_run_id", "workflowRunId"], "workflow_run_id")?;
            action_result(
                "list_agent_runs_for_workflow",
                agent_repo::list_agent_runs_for_workflow(&state.db, &workflow_run_id).await?,
            )
        }
        "mark_workflow_run_failed" => {
            let workflow_run_id =
                args.required_string(&["workflow_run_id", "workflowRunId"], "workflow_run_id")?;
            let run = workflow_repo::get_workflow_run(&state.db, &workflow_run_id).await?;
            if run.current_stage != "failed" {
                workflow_repo::update_workflow_stage(&state.db, &workflow_run_id, "failed").await?;
                workflow_repo::record_stage_transition(
                    &state.db,
                    &uuid::Uuid::new_v4().to_string(),
                    &workflow_run_id,
                    &run.current_stage,
                    "failed",
                    "user_override",
                    args.optional_string(&["reason"])?
                        .as_deref()
                        .unwrap_or("Marked failed by MCP operator"),
                )
                .await?;
            }
            workflow_repo::update_workflow_lifecycle(
                &state.db,
                &workflow_run_id,
                "failed",
                args.optional_string(&["reason"])?.as_deref(),
                true,
            )
            .await?;
            Ok(action_ok("mark_workflow_run_failed"))
        }
        "restart_workflow_run" => {
            let workflow_run_id =
                args.required_string(&["workflow_run_id", "workflowRunId"], "workflow_run_id")?;
            let run = workflow_repo::get_workflow_run(&state.db, &workflow_run_id).await?;
            let workflow_service = state.workflow_service.lock().await;
            let next = workflow_service
                .start_work_item_workflow(&run.work_item_id)
                .await?;
            action_result(
                "restart_workflow_run",
                json!({ "workflow_run_id": next.id }),
            )
        }
        other => Err(AppError::Validation(format!(
            "unsupported aruvi_workflows action: {other}"
        ))),
    }
}

async fn handle_checkpoints(state: &AppState, payload: Value) -> Result<Value, AppError> {
    let tool_action = ToolAction::parse(payload)?;
    let args = tool_action.args();

    match tool_action.action.as_str() {
        "approve_work_item" => {
            let work_item_id =
                args.required_string(&["work_item_id", "workItemId"], "work_item_id")?;
            let approval = approval_repo::create_approval(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &work_item_id,
                None,
                "task_approval",
                "approved",
                &args.optional_string(&["notes"])?.unwrap_or_default(),
            )
            .await?;
            work_item_repo::update_work_item(
                &state.db,
                &work_item_id,
                None,
                None,
                Some("approved"),
                None,
                None,
                None,
            )
            .await?;

            let auto_start = settings_repo::get_bool_setting(
                &state.db,
                AUTO_START_AFTER_WORK_ITEM_APPROVAL_KEY,
                true,
            )
            .await?;
            if auto_start {
                let workflow_service = state.workflow_service.lock().await;
                if let Err(err) = workflow_service
                    .start_work_item_workflow(&work_item_id)
                    .await
                {
                    error!(work_item_id = %work_item_id, error = %err, "auto-start after approval failed");
                }
            }

            action_result("approve_work_item", approval)
        }
        "reject_work_item" => {
            let work_item_id =
                args.required_string(&["work_item_id", "workItemId"], "work_item_id")?;
            let approval = approval_repo::create_approval(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &work_item_id,
                None,
                "task_approval",
                "rejected",
                &args.required_string(&["notes"], "notes")?,
            )
            .await?;
            work_item_repo::update_work_item(
                &state.db,
                &work_item_id,
                None,
                None,
                Some("draft"),
                None,
                None,
                None,
            )
            .await?;
            action_result("reject_work_item", approval)
        }
        "approve_work_item_plan" => action_result(
            "approve_work_item_plan",
            approval_repo::create_approval(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &args.required_string(&["work_item_id", "workItemId"], "work_item_id")?,
                None,
                "plan_approval",
                "approved",
                &args.optional_string(&["notes"])?.unwrap_or_default(),
            )
            .await?,
        ),
        "reject_work_item_plan" => action_result(
            "reject_work_item_plan",
            approval_repo::create_approval(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &args.required_string(&["work_item_id", "workItemId"], "work_item_id")?,
                None,
                "plan_approval",
                "rejected",
                &args.required_string(&["notes"], "notes")?,
            )
            .await?,
        ),
        "approve_work_item_test_review" => action_result(
            "approve_work_item_test_review",
            approval_repo::create_approval(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &args.required_string(&["work_item_id", "workItemId"], "work_item_id")?,
                None,
                "test_review",
                "approved",
                &args.optional_string(&["notes"])?.unwrap_or_default(),
            )
            .await?,
        ),
        "get_work_item_approvals" => action_result(
            "get_work_item_approvals",
            approval_repo::list_approvals(
                &state.db,
                &args.required_string(&["work_item_id", "workItemId"], "work_item_id")?,
            )
            .await?,
        ),
        "list_work_item_artifacts" => action_result(
            "list_work_item_artifacts",
            artifact_repo::list_work_item_artifacts(
                &state.db,
                &args.required_string(&["work_item_id", "workItemId"], "work_item_id")?,
            )
            .await?,
        ),
        "read_artifact_content" => {
            let artifact_id =
                args.required_string(&["artifact_id", "artifactId"], "artifact_id")?;
            let artifact = artifact_repo::get_artifact(&state.db, &artifact_id).await?;
            let content = tokio::fs::read_to_string(&artifact.storage_path).await?;
            action_result(
                "read_artifact_content",
                json!({
                    "artifact": artifact,
                    "content": content
                }),
            )
        }
        "list_work_item_findings" => action_result(
            "list_work_item_findings",
            finding_repo::list_work_item_findings(
                &state.db,
                &args.required_string(&["work_item_id", "workItemId"], "work_item_id")?,
            )
            .await?,
        ),
        "get_logs" => action_result(
            "get_logs",
            observability_repo::get_logs(
                &state.db,
                args.optional_string(&["level"])?.as_deref(),
                args.optional_string(&["target"])?.as_deref(),
                args.optional_string(&["workflow_run_id", "workflowRunId"])?
                    .as_deref(),
                args.optional_i64(&["limit"])?.unwrap_or(100),
            )
            .await?,
        ),
        other => Err(AppError::Validation(format!(
            "unsupported aruvi_checkpoints action: {other}"
        ))),
    }
}

async fn handle_agents(state: &AppState, payload: Value) -> Result<Value, AppError> {
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

async fn handle_models(state: &AppState, payload: Value) -> Result<Value, AppError> {
    let tool_action = ToolAction::parse(payload)?;
    let args = tool_action.args();

    match tool_action.action.as_str() {
        "create_provider" => {
            let id = uuid::Uuid::new_v4().to_string();
            let stored_secret_ref = secrets::store_provider_secret(
                &id,
                args.optional_string(&["auth_secret_ref", "authSecretRef"])?
                    .as_deref(),
            )?;
            let provider = model_repo::create_provider(
                &state.db,
                &id,
                &args.required_string(&["name"], "name")?,
                &args.string_or_default(&["provider_type", "providerType"], "openai_compatible")?,
                &args.required_string(&["base_url", "baseUrl"], "base_url")?,
                stored_secret_ref.as_deref(),
            )
            .await?;
            action_result("create_provider", provider)
        }
        "list_providers" => action_result(
            "list_providers",
            model_repo::list_providers(&state.db).await?,
        ),
        "update_provider" => {
            let id = args.required_string(&["id"], "id")?;
            let stored_secret_ref = if let Some(secret_input) =
                args.optional_string(&["auth_secret_ref", "authSecretRef"])?
            {
                secrets::store_provider_secret(&id, Some(&secret_input))?
            } else {
                None
            };
            let provider = model_repo::update_provider(
                &state.db,
                &id,
                args.optional_string(&["name"])?.as_deref(),
                args.optional_string(&["provider_type", "providerType"])?
                    .as_deref(),
                args.optional_string(&["base_url", "baseUrl"])?.as_deref(),
                stored_secret_ref.as_deref(),
                args.optional_bool(&["enabled"])?,
            )
            .await?;
            action_result("update_provider", provider)
        }
        "delete_provider" => {
            let id = args.required_string(&["id"], "id")?;
            model_repo::delete_provider(&state.db, &id).await?;
            Ok(action_ok("delete_provider"))
        }
        "create_model_definition" => {
            let model = model_repo::create_model_definition(
                &state.db,
                &uuid::Uuid::new_v4().to_string(),
                &args.required_string(&["provider_id", "providerId"], "provider_id")?,
                &args.required_string(&["name"], "name")?,
                args.optional_i64(&["context_window", "contextWindow"])?,
                args.optional_json_array_string(&["capability_tags", "capabilityTags"])?
                    .as_deref(),
                args.optional_string(&["notes"])?.as_deref(),
            )
            .await?;
            action_result("create_model_definition", model)
        }
        "list_model_definitions" => action_result(
            "list_model_definitions",
            model_repo::list_model_definitions(&state.db).await?,
        ),
        "update_model_definition" => {
            let id = args.required_string(&["id"], "id")?;
            let model = model_repo::update_model_definition(
                &state.db,
                &id,
                args.optional_string(&["provider_id", "providerId"])?
                    .as_deref(),
                args.optional_string(&["name"])?.as_deref(),
                args.optional_i64(&["context_window", "contextWindow"])?,
                args.optional_json_array_string(&["capability_tags", "capabilityTags"])?
                    .as_deref(),
                args.optional_string(&["notes"])?.as_deref(),
                args.optional_bool(&["enabled"])?,
            )
            .await?;
            action_result("update_model_definition", model)
        }
        "delete_model_definition" => {
            let id = args.required_string(&["id"], "id")?;
            model_repo::delete_model_definition(&state.db, &id).await?;
            Ok(action_ok("delete_model_definition"))
        }
        "test_provider_connectivity" => {
            let id = args.required_string(&["id"], "id")?;
            let provider = model_repo::get_provider(&state.db, &id).await?;
            let message = if matches!(provider.provider_type, ProviderType::LocalRuntime) {
                let model_path = crate::services::speech_service::resolve_local_runtime_model_path(
                    &provider.base_url,
                )?;
                format!(
                    "Local speech runtime is configured at {}. Whisper models transcribe audio; they do not perform speech synthesis.",
                    model_path.display()
                )
            } else {
                let api_key = secrets::resolve_provider_secret(&provider)?;
                let gateway = OpenAiCompatibleProvider::new(provider.base_url.clone(), api_key);
                match gateway.health_check().await {
                    Ok(true) => "Connection successful".to_string(),
                    Ok(false) => "Connection failed - server responded but not healthy".to_string(),
                    Err(error) => return Err(error),
                }
            };
            action_result("test_provider_connectivity", json!({ "message": message }))
        }
        "register_local_runtime_model" => {
            let registration = upsert_local_runtime_registration(
                state,
                &args.required_string(&["provider_name", "providerName"], "provider_name")?,
                &args.required_string(&["model_name", "modelName"], "model_name")?,
                &args.required_string(&["model_path", "modelPath"], "model_path")?,
                args.optional_json_array_string(&["capability_tags", "capabilityTags"])?
                    .as_deref(),
                args.optional_string(&["notes"])?.as_deref(),
                args.optional_i64(&["context_window", "contextWindow"])?,
                false,
            )
            .await?;
            action_result("register_local_runtime_model", registration)
        }
        "install_managed_local_model" => {
            let provider_name =
                args.required_string(&["provider_name", "providerName"], "provider_name")?;
            let model_name = args.required_string(&["model_name", "modelName"], "model_name")?;
            let download_url =
                args.required_string(&["download_url", "downloadUrl"], "download_url")?;
            let file_name = args.required_string(&["file_name", "fileName"], "file_name")?;
            let safe_dir = slugify(&provider_name);
            let models_dir = state.app_data_dir.join("models").join(safe_dir);
            tokio::fs::create_dir_all(&models_dir).await?;
            let destination_path = models_dir.join(file_name.trim());

            let mut downloaded = false;
            if !destination_path.exists() {
                let response = reqwest::get(download_url.trim()).await.map_err(|error| {
                    AppError::Provider(format!("Failed to download model: {error}"))
                })?;
                if !response.status().is_success() {
                    return Err(AppError::Provider(format!(
                        "Failed to download model: HTTP {}",
                        response.status()
                    )));
                }

                let mut file = tokio::fs::File::create(&destination_path).await?;
                let mut stream = response.bytes_stream();
                use futures_util::StreamExt;
                use tokio::io::AsyncWriteExt;

                while let Some(chunk) = stream.next().await {
                    let bytes = chunk.map_err(|error| {
                        AppError::Provider(format!("Failed to read model download stream: {error}"))
                    })?;
                    file.write_all(&bytes).await?;
                }
                file.flush().await?;
                downloaded = true;
            }

            let registration = upsert_local_runtime_registration(
                state,
                &provider_name,
                &model_name,
                destination_path.to_str().ok_or_else(|| {
                    AppError::Validation("Installed model path is not valid UTF-8".to_string())
                })?,
                args.optional_json_array_string(&["capability_tags", "capabilityTags"])?
                    .as_deref(),
                args.optional_string(&["notes"])?.as_deref(),
                args.optional_i64(&["context_window", "contextWindow"])?,
                downloaded,
            )
            .await?;
            action_result("install_managed_local_model", registration)
        }
        "run_model_chat_completion" => {
            let provider_id =
                args.required_string(&["provider_id", "providerId"], "provider_id")?;
            let provider = model_repo::get_provider(&state.db, &provider_id).await?;
            let api_key = secrets::resolve_provider_secret(&provider)?;
            let gateway = OpenAiCompatibleProvider::new(provider.base_url.clone(), api_key);
            let response = gateway
                .run_completion(CompletionRequest {
                    model: args.required_string(&["model"], "model")?,
                    messages: args.required_deserialize(&["messages"], "messages")?,
                    temperature: args.optional_f64(&["temperature"])?,
                    max_tokens: args.optional_i64(&["max_tokens", "maxTokens"])?,
                })
                .await?;
            action_result("run_model_chat_completion", response)
        }
        other => Err(AppError::Validation(format!(
            "unsupported aruvi_models action: {other}"
        ))),
    }
}

async fn handle_settings(state: &AppState, payload: Value) -> Result<Value, AppError> {
    let tool_action = ToolAction::parse(payload)?;
    let args = tool_action.args();

    match tool_action.action.as_str() {
        "get_setting" => action_result(
            "get_setting",
            json!({
                "key": args.required_string(&["key"], "key")?,
                "value": settings_repo::get_setting(&state.db, &args.required_string(&["key"], "key")?).await?
            }),
        ),
        "set_setting" => {
            settings_repo::set_setting(
                &state.db,
                &args.required_string(&["key"], "key")?,
                &args.required_string(&["value"], "value")?,
            )
            .await?;
            Ok(action_ok("set_setting"))
        }
        "get_mobile_bridge_status" => action_result(
            "get_mobile_bridge_status",
            webhook_service::resolve_mobile_bridge_status(state)
                .await
                .map_err(AppError::Internal)?,
        ),
        "get_mcp_bridge_status" => action_result(
            "get_mcp_bridge_status",
            webhook_service::resolve_mcp_bridge_status(state)
                .await
                .map_err(AppError::Internal)?,
        ),
        "get_database_health" => {
            let migrations = sqlx::query_as::<_, MigrationStatus>(
                "SELECT version, description, success, datetime(installed_on, 'unixepoch') AS installed_on
                 FROM _sqlx_migrations
                 ORDER BY version ASC",
            )
            .fetch_all(&state.db)
            .await?;
            let latest_version = migrations.last().map(|migration| migration.version);
            action_result(
                "get_database_health",
                DatabaseHealth {
                    applied_migrations: migrations.len(),
                    latest_version,
                    migrations,
                },
            )
        }
        "get_active_database_path" => {
            let rows = sqlx::query("PRAGMA database_list")
                .fetch_all(&state.db)
                .await?;
            let main_path = rows
                .iter()
                .find(|row| row.get::<String, _>("name") == "main")
                .map(|row| row.get::<String, _>("file"))
                .ok_or_else(|| {
                    AppError::Internal("Unable to resolve active SQLite database path".to_string())
                })?;
            action_result("get_active_database_path", json!({ "path": main_path }))
        }
        "get_database_path_override" => {
            let override_path = state.app_data_dir.join("db_override_path.txt");
            let value = match std::fs::read_to_string(&override_path) {
                Ok(content) => {
                    let trimmed = content.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                Err(error) => return Err(error.into()),
            };
            action_result("get_database_path_override", json!({ "path": value }))
        }
        "set_database_path_override" => {
            let db_path = args.required_string(&["db_path", "dbPath"], "db_path")?;
            if !std::path::Path::new(&db_path).is_absolute() {
                return Err(AppError::Validation(
                    "Database path must be an absolute path".to_string(),
                ));
            }
            std::fs::write(
                state.app_data_dir.join("db_override_path.txt"),
                db_path.trim(),
            )?;
            Ok(action_ok("set_database_path_override"))
        }
        "clear_database_path_override" => {
            let override_path = state.app_data_dir.join("db_override_path.txt");
            match std::fs::remove_file(override_path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
            Ok(action_ok("clear_database_path_override"))
        }
        other => Err(AppError::Validation(format!(
            "unsupported aruvi_settings action: {other}"
        ))),
    }
}

async fn handle_channels(state: &AppState, payload: Value) -> Result<Value, AppError> {
    let tool_action = ToolAction::parse(payload)?;
    let args = tool_action.args();

    match tool_action.action.as_str() {
        "send_twilio_whatsapp_message" => {
            channel_service::send_whatsapp_message(
                state,
                args.required_string(&["to"], "to")?,
                args.required_string(&["content"], "content")?,
            )
            .await?;
            Ok(action_ok("send_twilio_whatsapp_message"))
        }
        "start_twilio_voice_call" => {
            channel_service::start_voice_call(
                state,
                args.required_string(&["to"], "to")?,
                args.optional_string(&["initial_prompt", "initialPrompt"])?,
            )
            .await?;
            Ok(action_ok("start_twilio_voice_call"))
        }
        "route_planner_contact" => action_result(
            "route_planner_contact",
            channel_service::route_planner_contact(
                state,
                PlannerContactRequest {
                    to: args.required_string(&["to"], "to")?,
                    content: args.required_string(&["content"], "content")?,
                    preferred_channel: args
                        .optional_string(&["preferred_channel", "preferredChannel"])?,
                    allow_after_hours: args
                        .optional_bool(&["allow_after_hours", "allowAfterHours"])?,
                },
            )
            .await?,
        ),
        other => Err(AppError::Validation(format!(
            "unsupported aruvi_channels action: {other}"
        ))),
    }
}

async fn handle_speech(state: &AppState, payload: Value) -> Result<Value, AppError> {
    let tool_action = ToolAction::parse(payload)?;
    let args = tool_action.args();

    match tool_action.action.as_str() {
        "transcribe_audio" => {
            let provider_setting =
                settings_repo::get_setting(&state.db, "speech.transcription_provider_id").await?;
            let model_setting =
                settings_repo::get_setting(&state.db, "speech.transcription_model_name").await?;
            let provider_id = args
                .optional_string(&["provider_id", "providerId"])?
                .filter(|value| !value.trim().is_empty())
                .or(provider_setting)
                .ok_or_else(|| {
                    AppError::Validation("A speech transcription provider is required".to_string())
                })?;
            let requested_model_name = args
                .optional_string(&["model_name", "modelName"])?
                .filter(|value| !value.trim().is_empty())
                .or(model_setting);
            let provider_models = model_repo::list_model_definitions(&state.db)
                .await?
                .into_iter()
                .filter(|model| model.provider_id == provider_id);
            let speech_models = provider_models
                .filter(model_supports_transcription)
                .collect::<Vec<_>>();
            let model_name = if let Some(model_name) = requested_model_name {
                let known_model = speech_models.iter().any(|model| model.name == model_name);
                if known_model || looks_like_transcription_model(&model_name) {
                    model_name
                } else {
                    return Err(AppError::Validation(format!(
                        "Configured speech model '{}' does not look like a transcription model for this provider. Choose a Whisper/STT model in Settings.",
                        model_name
                    )));
                }
            } else if let Some(model) = speech_models.first() {
                model.name.clone()
            } else {
                "whisper-1".to_string()
            };
            let provider = model_repo::get_provider(&state.db, &provider_id).await?;
            let transcript = transcribe_audio_with_provider(
                &provider,
                &model_name,
                crate::services::speech_service::SpeechToTextRequest {
                    audio_bytes_base64: args.required_string(
                        &["audio_bytes_base64", "audioBytesBase64"],
                        "audio_bytes_base64",
                    )?,
                    mime_type: args.required_string(&["mime_type", "mimeType"], "mime_type")?,
                    locale: args
                        .optional_string(&["locale"])?
                        .or(settings_repo::get_setting(&state.db, "speech.locale").await?),
                },
            )
            .await?;
            action_result("transcribe_audio", transcript)
        }
        "speak_text_natively" => {
            let voice = args
                .optional_string(&["voice"])?
                .filter(|value| !value.trim().is_empty())
                .or(settings_repo::get_setting(&state.db, "speech.native_voice").await?);
            let locale = args
                .optional_string(&["locale"])?
                .filter(|value| !value.trim().is_empty())
                .or(settings_repo::get_setting(&state.db, "speech.locale").await?);
            speak_text_natively(TextToSpeechRequest {
                text: args.required_string(&["text"], "text")?,
                voice,
                locale,
            })?;
            Ok(action_ok("speak_text_natively"))
        }
        other => Err(AppError::Validation(format!(
            "unsupported aruvi_speech action: {other}"
        ))),
    }
}

fn model_supports_transcription(model: &ModelDefinition) -> bool {
    model.enabled
        && (model
            .capability_tags
            .iter()
            .any(|tag| matches!(tag.as_str(), "speech_to_text" | "transcription" | "audio"))
            || looks_like_transcription_model(&model.name))
}

fn slugify(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut last_was_dash = false;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            output.push(ch.to_ascii_lowercase());
            last_was_dash = false;
        } else if !last_was_dash {
            output.push('-');
            last_was_dash = true;
        }
    }
    output.trim_matches('-').to_string()
}
