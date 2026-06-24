use crate::domain::model::{ModelCall, ModelDefinition, ModelProvider, ProviderType};
use crate::error::AppError;
use crate::persistence::{model_call_repo, model_repo};
use crate::providers::gateway::ModelGateway;
use crate::providers::openai_compatible::OpenAiCompatibleProvider;
use crate::providers::types::{ChatMessage, CompletionRequest, CompletionResponse};
use crate::secrets;
use crate::services::speech_service::resolve_local_runtime_model_path;
use crate::state::AppState;
use serde::Deserialize;
use std::path::PathBuf;
use std::time::Instant;
use tauri::State;
use tracing::warn;

pub(crate) mod local_runtime;
pub(crate) use local_runtime::{upsert_local_runtime_registration, LocalRuntimeRegistrationInput};
pub(crate) mod stream;
mod telemetry;
use telemetry::{char_count_i64, elapsed_ms, record_model_command_call, ModelCommandCallRecord};

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
