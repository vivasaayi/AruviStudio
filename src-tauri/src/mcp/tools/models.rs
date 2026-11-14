use crate::commands::model_commands::{
    upsert_local_runtime_registration, LocalRuntimeRegistrationInput,
    local_runtime::{install_managed_local_model, InstallManagedLocalModelCommand},
};
use crate::domain::model::ProviderType;
use crate::error::AppError;
use crate::persistence::{model_call_repo, model_repo};
use crate::services::model_service::create_gateway;
use crate::providers::types::{ChatMessage, CompletionRequest};
use crate::secrets;
use crate::state::AppState;
use serde_json::{json, Value};
use std::time::Instant;

use super::action_args::ToolAction;
use super::{action_ok, action_result};

pub(super) async fn handle(state: &AppState, payload: Value) -> Result<Value, AppError> {
    let tool_action = ToolAction::parse(payload)?;
    let args = tool_action.args();

    match tool_action.action.as_str() {
        "run_file_copilot" => {
            let body = crate::services::webhook_mobile_chat::MobileFileCopilotRequest {
                repository_id: args.required_string(&["repository_id", "repositoryId"], "repository_id")?,
                relative_path: args.required_string(&["relative_path", "relativePath"], "relative_path")?,
                expected_sha256: args.required_string(&["expected_sha256", "expectedSha256"], "expected_sha256")?,
                working_content: args.optional_string(&["working_content", "workingContent"])?,
                include_file_context: args.optional_bool(&["include_file_context", "includeFileContext"])?,
                context_budget: args.optional_deserialize(&["context_budget", "contextBudget"], "context_budget")?,
                system_prompt: args.optional_string(&["system_prompt", "systemPrompt"])?,
                temperature: args.optional_f64(&["temperature"])?,
                max_tokens: args.optional_i64(&["max_tokens", "maxTokens"])?,
                provider_id: args.required_string(&["provider_id", "providerId"], "provider_id")?,
                model_name: args.required_string(&["model_name", "modelName"], "model_name")?,
                question: args.required_string(&["question"], "question")?,
                patch_mode: args.bool_or_default(&["patch_mode", "patchMode"], false)?,
                history: args.optional_deserialize(&["history"], "history")?.unwrap_or_default(),
            };
            let response = crate::services::webhook_mobile_chat::run_file_copilot(state, body, "mcp_ide", "MCP IDE")
                .await.map_err(|(_, message)| AppError::Validation(message))?;
            action_result("run_file_copilot", response)
        }
        "read_model_call_snapshot" => {
            let id = args.required_string(&["id"], "id")?;
            let kind = args.required_string(&["kind"], "kind")?;
            action_result("read_model_call_snapshot", crate::commands::model_commands::read_snapshot_for_state(state, &id, &kind, Some(1024 * 1024)).await?)
        }
        "create_provider" => {
            let provider_type = args.string_or_default(&["provider_type", "providerType"], "openai_compatible")?;
            let protocol = args.string_or_default(&["protocol"], "openai_compatible")?;
            model_repo::validate_provider_protocol(&provider_type, &protocol)?;
            let profile = args.string_or_default(&["completion_profile", "completionProfile"], "standard")?;
            model_repo::validate_completion_profile(&provider_type, &protocol, &profile)?;
            let name = args.required_string(&["name"], "name")?;
            let base_url = args.required_string(&["base_url", "baseUrl"], "base_url")?;
            model_repo::validate_provider_base_url(&provider_type, &base_url)?;
            let id = uuid::Uuid::new_v4().to_string();
            let stored_secret_ref = secrets::store_provider_secret(
                &id,
                args.optional_string(&["auth_secret_ref", "authSecretRef"])?
                    .as_deref(),
            )?;
            let provider = model_repo::create_provider_configured(
                &state.db,
                &id,
                &name,
                &provider_type,
                &base_url,
                stored_secret_ref.as_deref(),
                &protocol,
                &profile,
            )
            .await?;
            action_result("create_provider", provider)
        }
        "list_providers" => action_result(
            "list_providers",
            model_repo::list_providers(&state.db).await?,
        ),
        "get_provider_credential_state" => {
            let id = args.required_string(&["id"], "id")?;
            let (revision, disabled): (String, bool) = sqlx::query_as("SELECT credential_revision, auth_secret_ref IS ? FROM model_providers WHERE id=?")
                .bind(secrets::NO_PROVIDER_CREDENTIAL).bind(&id).fetch_optional(&state.db).await?
                .ok_or_else(|| AppError::NotFound(format!("Provider {id} not found")))?;
            action_result("get_provider_credential_state", json!({"id":id,"credential_revision":revision,"credentials_disabled":disabled}))
        }
        "detach_provider_credential" => {
            let id = args.required_string(&["id"], "id")?;
            let revision = args.required_string(&["credential_revision", "credentialRevision"], "credential_revision")?;
            let result = sqlx::query("UPDATE model_providers SET auth_secret_ref=?,updated_at=datetime('now') WHERE id=? AND credential_revision=? AND auth_secret_ref IS NOT ?")
                .bind(secrets::NO_PROVIDER_CREDENTIAL).bind(&id).bind(revision).bind(secrets::NO_PROVIDER_CREDENTIAL).execute(&state.db).await?;
            if result.rows_affected() != 1 { return Err(AppError::Validation("Provider or credential changed or was removed. Refresh before detaching the key.".into())); }
            Ok(action_ok("detach_provider_credential"))
        }
        "update_provider" => {
            let id = args.required_string(&["id"], "id")?;
            let name = args.optional_string(&["name"])?;
            let provider_type = args.optional_string(&["provider_type", "providerType"])?;
            let protocol = args.optional_string(&["protocol"])?;
            let profile = args.optional_string(&["completion_profile", "completionProfile"])?;
            let base_url = args.optional_string(&["base_url", "baseUrl"])?;
            let enabled = args.optional_bool(&["enabled"])?;
            let existing = model_repo::get_provider(&state.db, &id).await?;
            let next_type = provider_type.as_deref().unwrap_or(existing.provider_type.as_str());
            let next_protocol = protocol.as_deref().unwrap_or(&existing.protocol);
            model_repo::validate_provider_protocol(next_type, next_protocol)?;
            model_repo::validate_completion_profile(next_type, next_protocol, profile.as_deref().unwrap_or(&existing.completion_profile))?;
            if base_url.is_some() || provider_type.is_some() {
                model_repo::validate_provider_base_url(next_type, base_url.as_deref().unwrap_or(&existing.base_url))?;
            }
            let stored_secret_ref = if let Some(secret_input) =
                args.optional_string(&["auth_secret_ref", "authSecretRef"])?
            {
                secrets::store_updated_provider_secret(&id, &secret_input)?
            } else {
                None
            };
            let provider = model_repo::update_provider(
                &state.db,
                &id,
                name.as_deref(),
                provider_type.as_deref(),
                protocol.as_deref(),
                profile.as_deref(),
                base_url.as_deref(),
                stored_secret_ref.as_deref(),
                enabled,
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
            let provider_id = args.optional_string(&["provider_id", "providerId"])?;
            let name = args.optional_string(&["name"])?;
            let capability_tags =
                args.optional_json_array_string(&["capability_tags", "capabilityTags"])?;
            let notes = args.optional_string(&["notes"])?;
            let model = model_repo::update_model_definition(
                &state.db,
                model_repo::UpdateModelDefinitionPatch {
                    id: &id,
                    provider_id: provider_id.as_deref(),
                    name: name.as_deref(),
                    context_window: args.optional_i64(&["context_window", "contextWindow"])?,
                    capability_tags: capability_tags.as_deref(),
                    notes: notes.as_deref(),
                    enabled: args.optional_bool(&["enabled"])?,
                },
            )
            .await?;
            action_result("update_model_definition", model)
        }
        "delete_model_definition" => {
            let id = args.required_string(&["id"], "id")?;
            model_repo::delete_model_definition(&state.db, &id).await?;
            Ok(action_ok("delete_model_definition"))
        }
        "get_provider_transport_capabilities" => {
            let id = args.required_string(&["id", "provider_id", "providerId"], "provider_id")?;
            let provider = model_repo::get_provider(&state.db, &id).await?;
            if !provider.enabled { return Err(AppError::Validation("Selected provider is disabled".into())); }
            let gateway = crate::services::model_service::create_shared_gateway(&provider)?;
            let capabilities = gateway.transport_capabilities();
            action_result("get_provider_transport_capabilities", json!({
                "text_completion": capabilities.text_completion,
                "text_streaming": capabilities.text_streaming,
                "model_discovery": capabilities.model_discovery,
                "transcription": capabilities.transcription,
                "speech": capabilities.speech,
                "json_object_completion": gateway.supports_json_object_completion(),
            }))
        }
        "discover_provider_models" => {
            let id = args.required_string(&["id", "provider_id", "providerId"], "provider_id")?;
            let provider = model_repo::get_provider(&state.db, &id).await?;
            if !provider.enabled { return Err(AppError::Validation("Selected provider is disabled".into())); }
            let gateway = create_gateway(&provider)?;
            let cursor = args.optional_string(&["cursor"])?;
            action_result("discover_provider_models", gateway.list_models_page(cursor.as_deref()).await?)
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
                let gateway = create_gateway(&provider)?;
                match gateway.health_check().await {
                    Ok(true) => "Connection successful".to_string(),
                    Ok(false) => "Connection failed - server responded but not healthy".to_string(),
                    Err(error) => return Err(error),
                }
            };
            action_result("test_provider_connectivity", json!({ "message": message }))
        }
        "register_local_runtime_model" => {
            let provider_name =
                args.required_string(&["provider_name", "providerName"], "provider_name")?;
            let model_name = args.required_string(&["model_name", "modelName"], "model_name")?;
            let model_path = args.required_string(&["model_path", "modelPath"], "model_path")?;
            let capability_tags =
                args.optional_json_array_string(&["capability_tags", "capabilityTags"])?;
            let notes = args.optional_string(&["notes"])?;
            let registration = upsert_local_runtime_registration(
                state,
                LocalRuntimeRegistrationInput {
                    provider_name: &provider_name,
                    model_name: &model_name,
                    model_path: &model_path,
                    capability_tags: capability_tags.as_deref(),
                    notes: notes.as_deref(),
                    context_window: args.optional_i64(&["context_window", "contextWindow"])?,
                    downloaded: false,
                },
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
            let capability_tags =
                args.optional_json_array_string(&["capability_tags", "capabilityTags"])?;
            let notes = args.optional_string(&["notes"])?;
            let registration = install_managed_local_model(state, InstallManagedLocalModelCommand {
                provider_name, model_name, download_url, file_name, capability_tags, notes,
                expected_sha256: args.optional_string(&["expected_sha256", "expectedSha256"])?,
                context_window: args.optional_i64(&["context_window", "contextWindow"])?,
            }).await?;
            action_result("install_managed_local_model", registration)
        }
        "run_model_chat_completion" => {
            let provider_id =
                args.required_string(&["provider_id", "providerId"], "provider_id")?;
            let provider = model_repo::get_provider(&state.db, &provider_id).await?;
            let gateway = create_gateway(&provider)?;
            let model = args.required_string(&["model"], "model")?;
            let messages =
                args.required_deserialize::<Vec<ChatMessage>>(&["messages"], "messages")?;
            let temperature = args.optional_f64(&["temperature"])?;
            let max_tokens = args.optional_i64(&["max_tokens", "maxTokens"])?;
            let started = Instant::now();
            let response = match gateway
                .run_completion(CompletionRequest {
                    model: model.clone(),
                    messages: messages.clone(),
                    temperature,
                    max_tokens,
                })
                .await
            {
                Ok(response) => response,
                Err(error) => {
                    let error_message = error.to_string();
                    let call_index = model_call_repo::next_model_call_index(
                        &state.db,
                        "mcp_model_completion",
                        None,
                    )
                    .await?;
                    let call_id = uuid::Uuid::new_v4().to_string();
                    let request_messages_json = serde_json::to_string_pretty(&messages)?;
                    let snapshots = model_call_repo::write_model_call_snapshots(
                        &state.artifact_base_path,
                        &call_id,
                        Some(&request_messages_json),
                        None,
                    )
                    .await?;
                    model_call_repo::create_model_call(
                        &state.db,
                        model_call_repo::CreateModelCallParams {
                            id: &call_id,
                            source_kind: "mcp_model_completion",
                            source_id: None,
                            source_label: "MCP Model Completion",
                            workflow_run_id: None,
                            agent_run_id: None,
                            work_item_id: None,
                            product_id: None,
                            session_id: None,
                            agent_id: None,
                            stage: None,
                            provider_id: &provider.id,
                            provider_name: &provider.name,
                            provider_type: provider.provider_type.as_str(),
                            provider_base_url: &provider.base_url,
                            model_id: None,
                            model_name: &model,
                            call_index,
                            request_message_count: i64::try_from(messages.len())
                                .unwrap_or(i64::MAX),
                            prompt_chars: message_char_count(&messages),
                            response_chars: 0,
                            request_snapshot_path: snapshots.request_snapshot_path.as_deref(),
                            response_snapshot_path: snapshots.response_snapshot_path.as_deref(),
                            max_tokens,
                            temperature,
                            token_count_input: None,
                            token_count_output: None,
                            duration_ms: Some(elapsed_ms(started)),
                            status: "failed",
                            error_message: Some(&error_message),
                        },
                    )
                    .await?;
                    return Err(error);
                }
            };
            let call_index =
                model_call_repo::next_model_call_index(&state.db, "mcp_model_completion", None)
                    .await?;
            let call_id = uuid::Uuid::new_v4().to_string();
            let request_messages_json = serde_json::to_string_pretty(&messages)?;
            let snapshots = model_call_repo::write_model_call_snapshots(
                &state.artifact_base_path,
                &call_id,
                Some(&request_messages_json),
                Some(&response.content),
            )
            .await?;
            model_call_repo::create_model_call(
                &state.db,
                model_call_repo::CreateModelCallParams {
                    id: &call_id,
                    source_kind: "mcp_model_completion",
                    source_id: None,
                    source_label: "MCP Model Completion",
                    workflow_run_id: None,
                    agent_run_id: None,
                    work_item_id: None,
                    product_id: None,
                    session_id: None,
                    agent_id: None,
                    stage: None,
                    provider_id: &provider.id,
                    provider_name: &provider.name,
                    provider_type: provider.provider_type.as_str(),
                    provider_base_url: &provider.base_url,
                    model_id: None,
                    model_name: &model,
                    call_index,
                    request_message_count: i64::try_from(messages.len()).unwrap_or(i64::MAX),
                    prompt_chars: message_char_count(&messages),
                    response_chars: char_count_i64(&response.content),
                    request_snapshot_path: snapshots.request_snapshot_path.as_deref(),
                    response_snapshot_path: snapshots.response_snapshot_path.as_deref(),
                    max_tokens,
                    temperature,
                    token_count_input: response.token_count_input,
                    token_count_output: response.token_count_output,
                    duration_ms: Some(elapsed_ms(started)),
                    status: "completed",
                    error_message: None,
                },
            )
            .await?;
            action_result("run_model_chat_completion", response)
        }
        other => Err(AppError::Validation(format!(
            "unsupported aruvi_models action: {other}"
        ))),
    }
}

fn char_count_i64(content: &str) -> i64 {
    i64::try_from(content.chars().count()).unwrap_or(i64::MAX)
}

fn message_char_count(messages: &[ChatMessage]) -> i64 {
    messages
        .iter()
        .map(|message| char_count_i64(&message.content))
        .sum()
}

fn elapsed_ms(started: Instant) -> i64 {
    i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX)
}
