use crate::commands::model_commands::upsert_local_runtime_registration;
use crate::domain::model::ProviderType;
use crate::error::AppError;
use crate::persistence::{model_call_repo, model_repo};
use crate::providers::gateway::ModelGateway;
use crate::providers::openai_compatible::OpenAiCompatibleProvider;
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
