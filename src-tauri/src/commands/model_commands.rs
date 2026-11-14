use crate::domain::model::{ModelCall, ModelDefinition, ModelProvider, ProviderType};
use crate::error::AppError;
use crate::persistence::{model_call_repo, model_repo};
use crate::services::model_service::create_gateway;
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
    #[serde(alias = "streamId")]
    pub(crate) stream_id: Option<String>,
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
    protocol: Option<String>,
    completion_profile: Option<String>,
) -> Result<ModelProvider, AppError> {
    let protocol = protocol.as_deref().unwrap_or("openai_compatible");
    model_repo::validate_provider_protocol(&provider_type, protocol)?;
    let profile = completion_profile.as_deref().unwrap_or("standard");
    model_repo::validate_completion_profile(&provider_type, protocol, profile)?;
    model_repo::validate_provider_base_url(&provider_type, &base_url)?;
    let id = uuid::Uuid::new_v4().to_string();
    let stored_secret_ref = secrets::store_provider_secret(&id, auth_secret_ref.as_deref())?;
    model_repo::create_provider_configured(
        &state.db,
        &id,
        &name,
        &provider_type,
        &base_url,
        stored_secret_ref.as_deref(),
        protocol,
        profile,
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
    protocol: Option<String>,
    completion_profile: Option<String>,
    base_url: Option<String>,
    auth_secret_ref: Option<String>,
    enabled: Option<bool>,
) -> Result<ModelProvider, AppError> {
    let existing = model_repo::get_provider(&state.db, &id).await?;
    let next_type = provider_type.as_deref().unwrap_or(existing.provider_type.as_str());
    let next_protocol = protocol.as_deref().unwrap_or(&existing.protocol);
    model_repo::validate_provider_protocol(next_type, next_protocol)?;
    model_repo::validate_completion_profile(next_type, next_protocol, completion_profile.as_deref().unwrap_or(&existing.completion_profile))?;
    if base_url.is_some() || provider_type.is_some() {
        model_repo::validate_provider_base_url(next_type, base_url.as_deref().unwrap_or(&existing.base_url))?;
    }
    let stored_secret_ref = if let Some(secret_input) = auth_secret_ref.as_deref() {
        secrets::store_updated_provider_secret(&id, secret_input)?
    } else {
        None
    };
    model_repo::update_provider(
        &state.db,
        &id,
        name.as_deref(),
        provider_type.as_deref(),
        protocol.as_deref(),
        completion_profile.as_deref(),
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
pub async fn discover_provider_models(
    state: State<'_, AppState>, id: String, cursor: Option<String>,
) -> Result<aruvi_model_client::ModelPage, AppError> {
    let provider = model_repo::get_provider(&state.db, &id).await?;
    if !provider.enabled { return Err(AppError::Validation("Selected provider is disabled".into())); }
    create_gateway(&provider)?.list_models_page(cursor.as_deref()).await
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
    let gw = create_gateway(&provider)?;
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
    let gateway = create_gateway(&provider)?;
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
    read_snapshot_for_state(&state, &id, &kind, None).await
}

pub(crate) async fn read_snapshot_for_state(state: &AppState, id: &str, kind: &str, limit: Option<u64>) -> Result<String, AppError> {
    let call = model_call_repo::get_model_call(&state.db, id).await?;
    let snapshot_path = match kind {
        "request" => call.request_snapshot_path,
        "response" => call.response_snapshot_path,
        _ => {
            return Err(AppError::Validation(
                "snapshot kind must be request or response".to_string(),
            ))
        }
    }
    .ok_or_else(|| AppError::NotFound(format!("Model call {id} has no {kind} snapshot")))?;

    read_snapshot_file(&state.artifact_base_path, &snapshot_path, limit).await
}

async fn read_snapshot_file(artifact_base: &std::path::Path, snapshot_path: &str, limit: Option<u64>) -> Result<String, AppError> {
    use tokio::io::AsyncReadExt;
    let requested_path = PathBuf::from(snapshot_path);
    let canonical_path = tokio::fs::canonicalize(&requested_path).await?;
    let canonical_artifact_base = tokio::fs::canonicalize(artifact_base).await?;
    if !canonical_path.starts_with(&canonical_artifact_base) {
        return Err(AppError::Validation(
            "Snapshot path is outside the artifact directory".to_string(),
        ));
    }

    if let Some(limit) = limit {
        let file = tokio::fs::File::open(canonical_path).await?;
        if !file.metadata().await?.is_file() { return Err(AppError::Validation("Snapshot is not a regular file".into())); }
        let mut bytes = Vec::new();
        file.take(limit + 1).read_to_end(&mut bytes).await?;
        if bytes.len() as u64 > limit { return Err(AppError::Validation("Snapshot exceeds the remote read limit".into())); }
        String::from_utf8(bytes).map_err(|_| AppError::Validation("Snapshot is not valid UTF-8".into()))
    } else {
        tokio::fs::read_to_string(canonical_path).await.map_err(AppError::from)
    }
}

#[cfg(test)]
mod provider_ipc_tests {
    use tauri::test::{mock_builder, mock_context, noop_assets, INVOKE_KEY};

    #[test]
    fn desktop_ipc_preserves_provider_completion_profiles() {
        let directory = tempfile::tempdir().unwrap();
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let state = runtime.block_on(async {
            let db = crate::persistence::db::create_pool(&format!("sqlite:{}", directory.path().join("providers.db").display())).await.unwrap();
            crate::state::AppState::new(db, directory.path().to_path_buf()).await.unwrap()
        });
        let app = mock_builder().manage(state)
            .invoke_handler(tauri::generate_handler![super::create_provider, super::update_provider, super::list_providers])
            .build(mock_context(noop_assets())).unwrap();
        let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default()).build().unwrap();
        let invoke = |cmd: &str, body: serde_json::Value| {
            tauri::test::get_ipc_response(&webview, tauri::webview::InvokeRequest {
                cmd: cmd.into(), callback: tauri::ipc::CallbackFn(0), error: tauri::ipc::CallbackFn(1),
                url: "http://tauri.localhost".parse().unwrap(), body: tauri::ipc::InvokeBody::Json(body),
                headers: Default::default(), invoke_key: INVOKE_KEY.to_string(),
            })
        };
        let created = invoke("create_provider", serde_json::json!({"name":"Reasoning IPC", "providerType":"openai_compatible", "protocol":"openai_compatible", "completionProfile":"openai_reasoning", "baseUrl":"https://example.invalid/v1"}))
            .unwrap().deserialize::<serde_json::Value>().unwrap();
        assert_eq!(created["completion_profile"], "openai_reasoning");
        let id = created["id"].as_str().unwrap();
        let renamed = invoke("update_provider", serde_json::json!({"id":id,"name":"Renamed IPC"}))
            .unwrap().deserialize::<serde_json::Value>().unwrap();
        assert_eq!(renamed["completion_profile"], "openai_reasoning");
        assert!(invoke("update_provider", serde_json::json!({"id":id,"protocol":"anthropic"})).is_err());
        let updated = invoke("update_provider", serde_json::json!({"id":id,"protocol":"anthropic","completionProfile":"standard"}))
            .unwrap().deserialize::<serde_json::Value>().unwrap();
        assert_eq!(updated["protocol"], "anthropic");
        assert_eq!(updated["completion_profile"], "standard");
        let legacy = invoke("create_provider", serde_json::json!({"name":"Legacy IPC", "providerType":"openai_compatible", "baseUrl":"https://example.invalid/v1"}))
            .unwrap().deserialize::<serde_json::Value>().unwrap();
        assert_eq!(legacy["completion_profile"], "standard");
        let listed = invoke("list_providers", serde_json::json!({})).unwrap().deserialize::<serde_json::Value>().unwrap();
        assert!(listed.as_array().unwrap().iter().any(|p| p["id"] == id && p["completion_profile"] == "standard"));
    }
}

#[cfg(test)]
mod remote_snapshot_tests {
    use super::*;

    #[tokio::test]
    async fn snapshot_reads_are_confined_and_bounded() {
        let dir = std::env::temp_dir().join(format!("aruvi-snapshot-{}", uuid::Uuid::new_v4()));
        let artifacts = dir.join("artifacts");
        tokio::fs::create_dir_all(&artifacts).await.unwrap();
        let valid = artifacts.join("request.json");
        let outside = dir.join("outside.txt");
        tokio::fs::write(&valid, "snapshot").await.unwrap();
        tokio::fs::write(&outside, "private").await.unwrap();
        assert_eq!(read_snapshot_file(&artifacts, valid.to_str().unwrap(), Some(8)).await.unwrap(), "snapshot");
        assert!(read_snapshot_file(&artifacts, valid.to_str().unwrap(), Some(7)).await.is_err());
        assert!(read_snapshot_file(&artifacts, outside.to_str().unwrap(), Some(100)).await.is_err());
        #[cfg(unix)] {
            let link = artifacts.join("escape");
            std::os::unix::fs::symlink(&outside, &link).unwrap();
            assert!(read_snapshot_file(&artifacts, link.to_str().unwrap(), Some(100)).await.is_err());
        }
        tokio::fs::remove_dir_all(dir).await.unwrap();
    }
}
