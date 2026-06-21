use crate::domain::model::{ModelCall, ModelDefinition, ModelProvider, ProviderType};
use crate::error::AppError;
use crate::persistence::{model_call_repo, model_repo};
use crate::providers::gateway::ModelGateway;
use crate::providers::openai_compatible::OpenAiCompatibleProvider;
use crate::providers::types::{ChatMessage, CompletionRequest, CompletionResponse};
use crate::secrets;
use crate::services::speech_service::resolve_local_runtime_model_path;
use crate::state::AppState;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tauri::{AppHandle, Emitter, State};
use tokio::io::AsyncWriteExt;
use tracing::warn;

#[derive(Debug, Clone, Serialize)]
struct ChatStreamChunkEvent {
    stream_id: String,
    delta: String,
}

#[derive(Debug, Clone, Serialize)]
struct ChatStreamDoneEvent {
    stream_id: String,
}

#[derive(Debug, Clone, Serialize)]
struct ChatStreamErrorEvent {
    stream_id: String,
    error: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateModelDefinitionCommand {
    pub(crate) id: String,
    #[serde(alias = "providerId")]
    pub(crate) provider_id: Option<String>,
    pub(crate) name: Option<String>,
    #[serde(alias = "contextWindow")]
    pub(crate) context_window: Option<i64>,
    #[serde(alias = "capabilityTags")]
    pub(crate) capability_tags: Option<String>,
    pub(crate) notes: Option<String>,
    pub(crate) enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct RegisterLocalRuntimeModelCommand {
    #[serde(alias = "providerName")]
    pub(crate) provider_name: String,
    #[serde(alias = "modelName")]
    pub(crate) model_name: String,
    #[serde(alias = "modelPath")]
    pub(crate) model_path: String,
    #[serde(alias = "capabilityTags")]
    pub(crate) capability_tags: Option<String>,
    pub(crate) notes: Option<String>,
    #[serde(alias = "contextWindow")]
    pub(crate) context_window: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct InstallManagedLocalModelCommand {
    #[serde(alias = "providerName")]
    pub(crate) provider_name: String,
    #[serde(alias = "modelName")]
    pub(crate) model_name: String,
    #[serde(alias = "downloadUrl")]
    pub(crate) download_url: String,
    #[serde(alias = "fileName")]
    pub(crate) file_name: String,
    #[serde(alias = "capabilityTags")]
    pub(crate) capability_tags: Option<String>,
    pub(crate) notes: Option<String>,
    #[serde(alias = "contextWindow")]
    pub(crate) context_window: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ModelChatCommand {
    #[serde(alias = "providerId")]
    pub(crate) provider_id: String,
    pub(crate) model: String,
    pub(crate) messages: Vec<ChatMessage>,
    pub(crate) temperature: Option<f64>,
    #[serde(alias = "maxTokens")]
    pub(crate) max_tokens: Option<i64>,
    #[serde(alias = "sourceKind")]
    pub(crate) source_kind: Option<String>,
    #[serde(alias = "sourceId")]
    pub(crate) source_id: Option<String>,
    #[serde(alias = "sourceLabel")]
    pub(crate) source_label: Option<String>,
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

async fn record_model_command_call(record: ModelCommandCallRecord<'_>) -> Result<(), AppError> {
    let call_index =
        model_call_repo::next_model_call_index(record.db, record.source_kind, record.source_id)
            .await?;
    let call_id = uuid::Uuid::new_v4().to_string();
    let request_messages_json = serde_json::to_string_pretty(record.messages)?;
    let snapshots = model_call_repo::write_model_call_snapshots(
        record.artifact_base_path,
        &call_id,
        Some(&request_messages_json),
        record.response_text,
    )
    .await?;
    model_call_repo::create_model_call(
        record.db,
        model_call_repo::CreateModelCallParams {
            id: &call_id,
            source_kind: record.source_kind,
            source_id: record.source_id,
            source_label: record.source_label,
            workflow_run_id: None,
            agent_run_id: None,
            work_item_id: None,
            product_id: None,
            session_id: record.source_id,
            agent_id: None,
            stage: None,
            provider_id: &record.provider.id,
            provider_name: &record.provider.name,
            provider_type: record.provider.provider_type.as_str(),
            provider_base_url: &record.provider.base_url,
            model_id: None,
            model_name: record.model_name,
            call_index,
            request_message_count: i64::try_from(record.messages.len()).unwrap_or(i64::MAX),
            prompt_chars: message_char_count(record.messages),
            response_chars: record.response_chars,
            request_snapshot_path: snapshots.request_snapshot_path.as_deref(),
            response_snapshot_path: snapshots.response_snapshot_path.as_deref(),
            max_tokens: record.max_tokens,
            temperature: record.temperature,
            token_count_input: record.token_count_input,
            token_count_output: record.token_count_output,
            duration_ms: Some(record.duration_ms),
            status: record.status,
            error_message: record.error_message,
        },
    )
    .await?;
    Ok(())
}

struct ModelCommandCallRecord<'a> {
    db: &'a SqlitePool,
    artifact_base_path: &'a Path,
    source_kind: &'a str,
    source_id: Option<&'a str>,
    source_label: &'a str,
    provider: &'a ModelProvider,
    model_name: &'a str,
    messages: &'a [ChatMessage],
    max_tokens: Option<i64>,
    temperature: Option<f64>,
    response_chars: i64,
    token_count_input: Option<i64>,
    token_count_output: Option<i64>,
    duration_ms: i64,
    status: &'a str,
    error_message: Option<&'a str>,
    response_text: Option<&'a str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalModelRegistrationResult {
    pub file_path: String,
    pub downloaded: bool,
    pub provider: ModelProvider,
    pub model_definition: ModelDefinition,
}

pub(crate) struct LocalRuntimeRegistrationInput<'a> {
    pub(crate) provider_name: &'a str,
    pub(crate) model_name: &'a str,
    pub(crate) model_path: &'a str,
    pub(crate) capability_tags: Option<&'a str>,
    pub(crate) notes: Option<&'a str>,
    pub(crate) context_window: Option<i64>,
    pub(crate) downloaded: bool,
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

pub(crate) async fn upsert_local_runtime_registration(
    state: &AppState,
    input: LocalRuntimeRegistrationInput<'_>,
) -> Result<LocalModelRegistrationResult, AppError> {
    let normalized_path = resolve_local_runtime_model_path(input.model_path)?;
    let normalized_path_string = normalized_path.display().to_string();

    let existing_provider = model_repo::list_providers(&state.db)
        .await?
        .into_iter()
        .find(|provider| {
            matches!(provider.provider_type, ProviderType::LocalRuntime)
                && provider.base_url == normalized_path_string
        });

    let provider = if let Some(provider) = existing_provider {
        provider
    } else {
        let provider_id = uuid::Uuid::new_v4().to_string();
        model_repo::create_provider(
            &state.db,
            &provider_id,
            input.provider_name,
            ProviderType::LocalRuntime.as_str(),
            &normalized_path_string,
            None,
        )
        .await?
    };

    let existing_model = model_repo::list_model_definitions(&state.db)
        .await?
        .into_iter()
        .find(|model| model.provider_id == provider.id && model.name == input.model_name);

    let model_definition = if let Some(model) = existing_model {
        model
    } else {
        let model_id = uuid::Uuid::new_v4().to_string();
        model_repo::create_model_definition(
            &state.db,
            &model_id,
            &provider.id,
            input.model_name,
            input.context_window,
            input.capability_tags,
            input.notes,
        )
        .await?
    };

    Ok(LocalModelRegistrationResult {
        file_path: normalized_path_string,
        downloaded: input.downloaded,
        provider,
        model_definition,
    })
}

fn endpoint_url(base_url: &str, path: &str) -> String {
    let trimmed = base_url.trim_end_matches('/');
    if trimmed.ends_with("/v1") {
        format!("{trimmed}{path}")
    } else {
        format!("{trimmed}/v1{path}")
    }
}

#[tauri::command]
pub async fn create_provider(
    state: State<'_, AppState>,
    name: String,
    provider_type: String,
    base_url: String,
    auth_secret_ref: Option<String>,
) -> Result<ModelProvider, AppError> {
    let id = uuid::Uuid::new_v4().to_string();
    let stored_secret_ref = secrets::store_provider_secret(&id, auth_secret_ref.as_deref())?;
    model_repo::create_provider(
        &state.db,
        &id,
        &name,
        &provider_type,
        &base_url,
        stored_secret_ref.as_deref(),
    )
    .await
}

#[tauri::command]
pub async fn list_providers(state: State<'_, AppState>) -> Result<Vec<ModelProvider>, AppError> {
    model_repo::list_providers(&state.db).await
}

#[tauri::command]
pub async fn update_provider(
    state: State<'_, AppState>,
    id: String,
    name: Option<String>,
    provider_type: Option<String>,
    base_url: Option<String>,
    auth_secret_ref: Option<String>,
    enabled: Option<bool>,
) -> Result<ModelProvider, AppError> {
    let stored_secret_ref = if let Some(secret_input) = auth_secret_ref.as_deref() {
        secrets::store_provider_secret(&id, Some(secret_input))?
    } else {
        None
    };
    model_repo::update_provider(
        &state.db,
        &id,
        name.as_deref(),
        provider_type.as_deref(),
        base_url.as_deref(),
        stored_secret_ref.as_deref(),
        enabled,
    )
    .await
}

#[tauri::command]
pub async fn delete_provider(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    model_repo::delete_provider(&state.db, &id).await
}

#[tauri::command]
pub async fn create_model_definition(
    state: State<'_, AppState>,
    provider_id: String,
    name: String,
    context_window: Option<i64>,
    capability_tags: Option<String>,
    notes: Option<String>,
) -> Result<ModelDefinition, AppError> {
    let id = uuid::Uuid::new_v4().to_string();
    model_repo::create_model_definition(
        &state.db,
        &id,
        &provider_id,
        &name,
        context_window,
        capability_tags.as_deref(),
        notes.as_deref(),
    )
    .await
}

#[tauri::command]
pub async fn list_model_definitions(
    state: State<'_, AppState>,
) -> Result<Vec<ModelDefinition>, AppError> {
    model_repo::list_model_definitions(&state.db).await
}

#[tauri::command]
pub async fn update_model_definition(
    state: State<'_, AppState>,
    request: UpdateModelDefinitionCommand,
) -> Result<ModelDefinition, AppError> {
    model_repo::update_model_definition(
        &state.db,
        model_repo::UpdateModelDefinitionPatch {
            id: &request.id,
            provider_id: request.provider_id.as_deref(),
            name: request.name.as_deref(),
            context_window: request.context_window,
            capability_tags: request.capability_tags.as_deref(),
            notes: request.notes.as_deref(),
            enabled: request.enabled,
        },
    )
    .await
}

#[tauri::command]
pub async fn delete_model_definition(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), AppError> {
    model_repo::delete_model_definition(&state.db, &id).await
}

#[tauri::command]
pub async fn test_provider_connectivity(
    state: State<'_, AppState>,
    id: String,
) -> Result<String, AppError> {
    let provider = model_repo::get_provider(&state.db, &id).await?;
    if matches!(provider.provider_type, ProviderType::LocalRuntime) {
        let model_path = resolve_local_runtime_model_path(&provider.base_url)?;
        return Ok(format!(
            "Local speech runtime is configured at {}. Whisper models transcribe audio; they do not perform speech synthesis.",
            model_path.display()
        ));
    }
    let api_key = secrets::resolve_provider_secret(&provider)?;
    let gw = OpenAiCompatibleProvider::new(provider.base_url, api_key);
    match gw.health_check().await {
        Ok(true) => Ok("Connection successful".to_string()),
        Ok(false) => Ok("Connection failed - server responded but not healthy".to_string()),
        Err(e) => Err(e),
    }
}

#[tauri::command]
pub async fn browse_for_local_model_file() -> Result<Option<String>, AppError> {
    let output = std::process::Command::new("osascript")
        .arg("-e")
        .arg(r#"POSIX path of (choose file with prompt "Select local model file")"#)
        .output()
        .map_err(|error| AppError::Validation(format!("Failed to open model picker: {error}")))?;

    if !output.status.success() {
        return Ok(None);
    }

    let selected = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if selected.is_empty() {
        Ok(None)
    } else {
        Ok(Some(selected))
    }
}

#[tauri::command]
pub async fn register_local_runtime_model_command(
    state: State<'_, AppState>,
    request: RegisterLocalRuntimeModelCommand,
) -> Result<LocalModelRegistrationResult, AppError> {
    upsert_local_runtime_registration(
        state.inner(),
        LocalRuntimeRegistrationInput {
            provider_name: &request.provider_name,
            model_name: &request.model_name,
            model_path: &request.model_path,
            capability_tags: request.capability_tags.as_deref(),
            notes: request.notes.as_deref(),
            context_window: request.context_window,
            downloaded: false,
        },
    )
    .await
}

#[tauri::command]
pub async fn install_managed_local_model_command(
    state: State<'_, AppState>,
    request: InstallManagedLocalModelCommand,
) -> Result<LocalModelRegistrationResult, AppError> {
    let safe_dir = slugify(&request.provider_name);
    let models_dir = state.app_data_dir.join("models").join(safe_dir);
    tokio::fs::create_dir_all(&models_dir).await?;
    let destination_path = models_dir.join(request.file_name.trim());

    let mut downloaded = false;
    if !destination_path.exists() {
        let response = reqwest::get(request.download_url.trim())
            .await
            .map_err(|error| AppError::Provider(format!("Failed to download model: {error}")))?;
        if !response.status().is_success() {
            return Err(AppError::Provider(format!(
                "Failed to download model: HTTP {}",
                response.status()
            )));
        }

        let mut file = tokio::fs::File::create(&destination_path).await?;
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let bytes = chunk.map_err(|error| {
                AppError::Provider(format!("Failed to read model download stream: {error}"))
            })?;
            file.write_all(&bytes).await?;
        }
        file.flush().await?;
        downloaded = true;
    }

    upsert_local_runtime_registration(
        state.inner(),
        LocalRuntimeRegistrationInput {
            provider_name: &request.provider_name,
            model_name: &request.model_name,
            model_path: destination_path.to_str().ok_or_else(|| {
                AppError::Validation("Installed model path is not valid UTF-8".to_string())
            })?,
            capability_tags: request.capability_tags.as_deref(),
            notes: request.notes.as_deref(),
            context_window: request.context_window,
            downloaded,
        },
    )
    .await
}

#[tauri::command]
pub async fn run_model_chat_completion(
    state: State<'_, AppState>,
    request: ModelChatCommand,
) -> Result<CompletionResponse, AppError> {
    let provider = model_repo::get_provider(&state.db, &request.provider_id).await?;
    let api_key = secrets::resolve_provider_secret(&provider)?;
    let gateway = OpenAiCompatibleProvider::new(provider.base_url.clone(), api_key);
    let source_kind = request
        .source_kind
        .unwrap_or_else(|| "desktop_model_completion".to_string());
    let source_label = request
        .source_label
        .unwrap_or_else(|| "Desktop model completion".to_string());
    let started = Instant::now();
    let response = match gateway
        .run_completion(CompletionRequest {
            model: request.model.clone(),
            messages: request.messages.clone(),
            temperature: request.temperature,
            max_tokens: request.max_tokens,
        })
        .await
    {
        Ok(response) => response,
        Err(error) => {
            let error_message = error.to_string();
            if let Err(record_error) = record_model_command_call(ModelCommandCallRecord {
                db: &state.db,
                artifact_base_path: &state.artifact_base_path,
                source_kind: &source_kind,
                source_id: request.source_id.as_deref(),
                source_label: &source_label,
                provider: &provider,
                model_name: &request.model,
                messages: &request.messages,
                max_tokens: request.max_tokens,
                temperature: request.temperature,
                response_chars: 0,
                token_count_input: None,
                token_count_output: None,
                duration_ms: elapsed_ms(started),
                status: "failed",
                error_message: Some(&error_message),
                response_text: None,
            })
            .await
            {
                warn!(error = %record_error, "Failed to record model completion telemetry");
            }
            return Err(error);
        }
    };
    if let Err(record_error) = record_model_command_call(ModelCommandCallRecord {
        db: &state.db,
        artifact_base_path: &state.artifact_base_path,
        source_kind: &source_kind,
        source_id: request.source_id.as_deref(),
        source_label: &source_label,
        provider: &provider,
        model_name: &request.model,
        messages: &request.messages,
        max_tokens: request.max_tokens,
        temperature: request.temperature,
        response_chars: char_count_i64(&response.content),
        token_count_input: response.token_count_input,
        token_count_output: response.token_count_output,
        duration_ms: elapsed_ms(started),
        status: "completed",
        error_message: None,
        response_text: Some(&response.content),
    })
    .await
    {
        warn!(error = %record_error, "Failed to record model completion telemetry");
    }
    Ok(response)
}

#[tauri::command]
pub async fn list_model_calls(
    state: State<'_, AppState>,
    limit: Option<i64>,
) -> Result<Vec<ModelCall>, AppError> {
    model_call_repo::list_model_calls(&state.db, limit.unwrap_or(200)).await
}

#[tauri::command]
pub async fn get_model_call(state: State<'_, AppState>, id: String) -> Result<ModelCall, AppError> {
    model_call_repo::get_model_call(&state.db, &id).await
}

#[tauri::command]
pub async fn read_model_call_snapshot(
    state: State<'_, AppState>,
    id: String,
    kind: String,
) -> Result<String, AppError> {
    let call = model_call_repo::get_model_call(&state.db, &id).await?;
    let snapshot_path = match kind.as_str() {
        "request" => call.request_snapshot_path,
        "response" => call.response_snapshot_path,
        _ => {
            return Err(AppError::Validation(
                "snapshot kind must be request or response".to_string(),
            ))
        }
    }
    .ok_or_else(|| AppError::NotFound(format!("Model call {id} has no {kind} snapshot")))?;

    let requested_path = PathBuf::from(snapshot_path);
    let canonical_path = tokio::fs::canonicalize(&requested_path).await?;
    let canonical_artifact_base = tokio::fs::canonicalize(&state.artifact_base_path).await?;
    if !canonical_path.starts_with(&canonical_artifact_base) {
        return Err(AppError::Validation(
            "Snapshot path is outside the artifact directory".to_string(),
        ));
    }

    tokio::fs::read_to_string(canonical_path)
        .await
        .map_err(AppError::from)
}

#[tauri::command]
pub async fn start_model_chat_stream(
    app: AppHandle,
    state: State<'_, AppState>,
    request: ModelChatCommand,
) -> Result<String, AppError> {
    let provider = model_repo::get_provider(&state.db, &request.provider_id).await?;
    let api_key = secrets::resolve_provider_secret(&provider)?;
    let stream_id = uuid::Uuid::new_v4().to_string();

    let db = state.db.clone();
    let artifact_base_path = state.artifact_base_path.clone();
    let source_kind = request
        .source_kind
        .unwrap_or_else(|| "desktop_stream".to_string());
    let telemetry_source_id = request.source_id.unwrap_or_else(|| stream_id.clone());
    let source_label = request
        .source_label
        .unwrap_or_else(|| "Desktop stream".to_string());
    let model = request.model;
    let messages = request.messages;
    let temperature = request.temperature;
    let max_tokens = request.max_tokens;
    let base_url = provider.base_url.clone();
    let stream_id_for_task = stream_id.clone();
    let telemetry_source_id_for_task = telemetry_source_id.clone();
    tokio::spawn(async move {
        let started = Instant::now();
        let mut response_chars = 0_i64;
        let mut response_text = String::new();
        let client = reqwest::Client::new();
        let url = endpoint_url(&base_url, "/chat/completions");
        let body = serde_json::json!({
            "model": model,
            "messages": messages.iter().map(|m| serde_json::json!({
                "role": m.role,
                "content": m.content,
            })).collect::<Vec<_>>(),
            "temperature": temperature.unwrap_or(0.7),
            "max_tokens": max_tokens.unwrap_or(4096),
            "stream": true,
        });

        let mut req = client.post(&url).json(&body);
        if let Some(key) = api_key {
            req = req.bearer_auth(key);
        }

        let response = match req.send().await {
            Ok(response) => response,
            Err(error) => {
                let error_text = format!("Request failed: {}", error);
                if let Err(record_error) = record_model_command_call(ModelCommandCallRecord {
                    db: &db,
                    artifact_base_path: &artifact_base_path,
                    source_kind: &source_kind,
                    source_id: Some(&telemetry_source_id_for_task),
                    source_label: &source_label,
                    provider: &provider,
                    model_name: &model,
                    messages: &messages,
                    max_tokens,
                    temperature,
                    response_chars,
                    token_count_input: None,
                    token_count_output: None,
                    duration_ms: elapsed_ms(started),
                    status: "failed",
                    error_message: Some(&error_text),
                    response_text: None,
                })
                .await
                {
                    warn!(error = %record_error, "Failed to record stream telemetry");
                }
                let _ = app.emit(
                    "chat_stream_error",
                    ChatStreamErrorEvent {
                        stream_id: stream_id_for_task.clone(),
                        error: error_text,
                    },
                );
                return;
            }
        };

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            let error_text = format!("API error {}: {}", status, text);
            if let Err(record_error) = record_model_command_call(ModelCommandCallRecord {
                db: &db,
                artifact_base_path: &artifact_base_path,
                source_kind: &source_kind,
                source_id: Some(&telemetry_source_id_for_task),
                source_label: &source_label,
                provider: &provider,
                model_name: &model,
                messages: &messages,
                max_tokens,
                temperature,
                response_chars,
                token_count_input: None,
                token_count_output: None,
                duration_ms: elapsed_ms(started),
                status: "failed",
                error_message: Some(&error_text),
                response_text: None,
            })
            .await
            {
                warn!(error = %record_error, "Failed to record stream telemetry");
            }
            let _ = app.emit(
                "chat_stream_error",
                ChatStreamErrorEvent {
                    stream_id: stream_id_for_task.clone(),
                    error: error_text,
                },
            );
            return;
        }

        let mut buffer = String::new();
        let mut stream = response.bytes_stream();

        while let Some(chunk_result) = stream.next().await {
            let chunk = match chunk_result {
                Ok(chunk) => chunk,
                Err(error) => {
                    let error_text = format!("Stream read failed: {}", error);
                    if let Err(record_error) = record_model_command_call(ModelCommandCallRecord {
                        db: &db,
                        artifact_base_path: &artifact_base_path,
                        source_kind: &source_kind,
                        source_id: Some(&telemetry_source_id_for_task),
                        source_label: &source_label,
                        provider: &provider,
                        model_name: &model,
                        messages: &messages,
                        max_tokens,
                        temperature,
                        response_chars,
                        token_count_input: None,
                        token_count_output: None,
                        duration_ms: elapsed_ms(started),
                        status: "failed",
                        error_message: Some(&error_text),
                        response_text: Some(&response_text),
                    })
                    .await
                    {
                        warn!(error = %record_error, "Failed to record stream telemetry");
                    }
                    let _ = app.emit(
                        "chat_stream_error",
                        ChatStreamErrorEvent {
                            stream_id: stream_id_for_task.clone(),
                            error: error_text,
                        },
                    );
                    return;
                }
            };

            buffer.push_str(&String::from_utf8_lossy(&chunk));
            while let Some(index) = buffer.find('\n') {
                let line = buffer[..index].to_string();
                buffer = buffer[index + 1..].to_string();
                let trimmed = line.trim();
                if !trimmed.starts_with("data:") {
                    continue;
                }
                let payload = trimmed.trim_start_matches("data:").trim();
                if payload == "[DONE]" {
                    if let Err(record_error) = record_model_command_call(ModelCommandCallRecord {
                        db: &db,
                        artifact_base_path: &artifact_base_path,
                        source_kind: &source_kind,
                        source_id: Some(&telemetry_source_id_for_task),
                        source_label: &source_label,
                        provider: &provider,
                        model_name: &model,
                        messages: &messages,
                        max_tokens,
                        temperature,
                        response_chars,
                        token_count_input: None,
                        token_count_output: None,
                        duration_ms: elapsed_ms(started),
                        status: "completed",
                        error_message: None,
                        response_text: Some(&response_text),
                    })
                    .await
                    {
                        warn!(error = %record_error, "Failed to record stream telemetry");
                    }
                    let _ = app.emit(
                        "chat_stream_done",
                        ChatStreamDoneEvent {
                            stream_id: stream_id_for_task.clone(),
                        },
                    );
                    return;
                }

                match serde_json::from_str::<serde_json::Value>(payload) {
                    Ok(value) => {
                        if let Some(delta) = value["choices"][0]["delta"]["content"].as_str() {
                            if !delta.is_empty() {
                                response_chars += char_count_i64(delta);
                                response_text.push_str(delta);
                                let _ = app.emit(
                                    "chat_stream_chunk",
                                    ChatStreamChunkEvent {
                                        stream_id: stream_id_for_task.clone(),
                                        delta: delta.to_string(),
                                    },
                                );
                            }
                        }
                    }
                    Err(error) => {
                        warn!(stream_id = %stream_id_for_task, error = %error, "Failed to parse stream payload");
                    }
                }
            }
        }

        let _ = app.emit(
            "chat_stream_done",
            ChatStreamDoneEvent {
                stream_id: stream_id_for_task.clone(),
            },
        );
        if let Err(record_error) = record_model_command_call(ModelCommandCallRecord {
            db: &db,
            artifact_base_path: &artifact_base_path,
            source_kind: &source_kind,
            source_id: Some(&telemetry_source_id_for_task),
            source_label: &source_label,
            provider: &provider,
            model_name: &model,
            messages: &messages,
            max_tokens,
            temperature,
            response_chars,
            token_count_input: None,
            token_count_output: None,
            duration_ms: elapsed_ms(started),
            status: "completed",
            error_message: None,
            response_text: Some(&response_text),
        })
        .await
        {
            warn!(error = %record_error, "Failed to record stream telemetry");
        }
    });

    Ok(stream_id)
}
