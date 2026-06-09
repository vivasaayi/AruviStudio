use crate::domain::model::ModelCall;
use crate::error::AppError;
use sqlx::{Row, SqlitePool};
use std::path::{Path, PathBuf};

pub struct CreateModelCallParams<'a> {
    pub id: &'a str,
    pub source_kind: &'a str,
    pub source_id: Option<&'a str>,
    pub source_label: &'a str,
    pub workflow_run_id: Option<&'a str>,
    pub agent_run_id: Option<&'a str>,
    pub work_item_id: Option<&'a str>,
    pub product_id: Option<&'a str>,
    pub session_id: Option<&'a str>,
    pub agent_id: Option<&'a str>,
    pub stage: Option<&'a str>,
    pub provider_id: &'a str,
    pub provider_name: &'a str,
    pub provider_type: &'a str,
    pub provider_base_url: &'a str,
    pub model_id: Option<&'a str>,
    pub model_name: &'a str,
    pub call_index: i64,
    pub request_message_count: i64,
    pub prompt_chars: i64,
    pub response_chars: i64,
    pub request_snapshot_path: Option<&'a str>,
    pub response_snapshot_path: Option<&'a str>,
    pub max_tokens: Option<i64>,
    pub temperature: Option<f64>,
    pub token_count_input: Option<i64>,
    pub token_count_output: Option<i64>,
    pub duration_ms: Option<i64>,
    pub status: &'a str,
    pub error_message: Option<&'a str>,
}

pub struct ModelCallSnapshotPaths {
    pub request_snapshot_path: Option<String>,
    pub response_snapshot_path: Option<String>,
}

pub async fn write_model_call_snapshots(
    artifact_base_path: &Path,
    call_id: &str,
    request_messages_json: Option<&str>,
    response_text: Option<&str>,
) -> Result<ModelCallSnapshotPaths, AppError> {
    if request_messages_json.is_none() && response_text.is_none() {
        return Ok(ModelCallSnapshotPaths {
            request_snapshot_path: None,
            response_snapshot_path: None,
        });
    }

    let snapshot_dir: PathBuf = artifact_base_path.join("model-calls").join(call_id);
    tokio::fs::create_dir_all(&snapshot_dir).await?;

    let request_snapshot_path = if let Some(request_messages_json) = request_messages_json {
        let request_path = snapshot_dir.join("request.json");
        tokio::fs::write(&request_path, request_messages_json).await?;
        Some(request_path.to_string_lossy().to_string())
    } else {
        None
    };

    let response_snapshot_path = if let Some(response_text) = response_text {
        let response_path = snapshot_dir.join("response.txt");
        tokio::fs::write(&response_path, response_text).await?;
        Some(response_path.to_string_lossy().to_string())
    } else {
        None
    };

    Ok(ModelCallSnapshotPaths {
        request_snapshot_path,
        response_snapshot_path,
    })
}

pub async fn next_model_call_index(
    pool: &SqlitePool,
    source_kind: &str,
    source_id: Option<&str>,
) -> Result<i64, AppError> {
    let row = sqlx::query(
        "SELECT COALESCE(MAX(call_index), 0) + 1 AS next_index
         FROM model_calls
         WHERE source_kind=? AND COALESCE(source_id, '')=COALESCE(?, '')",
    )
    .bind(source_kind)
    .bind(source_id)
    .fetch_one(pool)
    .await?;
    Ok(row.get("next_index"))
}

pub async fn create_model_call(
    pool: &SqlitePool,
    params: CreateModelCallParams<'_>,
) -> Result<ModelCall, AppError> {
    sqlx::query_as::<_, ModelCall>(
        "INSERT INTO model_calls (
            id,
            source_kind,
            source_id,
            source_label,
            workflow_run_id,
            agent_run_id,
            work_item_id,
            product_id,
            session_id,
            agent_id,
            stage,
            provider_id,
            provider_name,
            provider_type,
            provider_base_url,
            model_id,
            model_name,
            call_index,
            request_message_count,
            prompt_chars,
            response_chars,
            request_snapshot_path,
            response_snapshot_path,
            max_tokens,
            temperature,
            token_count_input,
            token_count_output,
            duration_ms,
            status,
            error_message
         )
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         RETURNING
            id,
            source_kind,
            source_id,
            source_label,
            workflow_run_id,
            agent_run_id,
            work_item_id,
            product_id,
            session_id,
            agent_id,
            stage,
            provider_id,
            provider_name,
            provider_type,
            provider_base_url,
            model_id,
            model_name,
            call_index,
            request_message_count,
            prompt_chars,
            response_chars,
            request_snapshot_path,
            response_snapshot_path,
            max_tokens,
            temperature,
            token_count_input,
            token_count_output,
            duration_ms,
            status,
            error_message,
            created_at",
    )
    .bind(params.id)
    .bind(params.source_kind)
    .bind(params.source_id)
    .bind(params.source_label)
    .bind(params.workflow_run_id)
    .bind(params.agent_run_id)
    .bind(params.work_item_id)
    .bind(params.product_id)
    .bind(params.session_id)
    .bind(params.agent_id)
    .bind(params.stage)
    .bind(params.provider_id)
    .bind(params.provider_name)
    .bind(params.provider_type)
    .bind(params.provider_base_url)
    .bind(params.model_id)
    .bind(params.model_name)
    .bind(params.call_index)
    .bind(params.request_message_count)
    .bind(params.prompt_chars)
    .bind(params.response_chars)
    .bind(params.request_snapshot_path)
    .bind(params.response_snapshot_path)
    .bind(params.max_tokens)
    .bind(params.temperature)
    .bind(params.token_count_input)
    .bind(params.token_count_output)
    .bind(params.duration_ms)
    .bind(params.status)
    .bind(params.error_message)
    .fetch_one(pool)
    .await
    .map_err(|e| e.into())
}

pub async fn list_model_calls(pool: &SqlitePool, limit: i64) -> Result<Vec<ModelCall>, AppError> {
    let bounded_limit = limit.clamp(1, 500);
    sqlx::query_as::<_, ModelCall>(
        "SELECT
            id,
            source_kind,
            source_id,
            source_label,
            workflow_run_id,
            agent_run_id,
            work_item_id,
            product_id,
            session_id,
            agent_id,
            stage,
            provider_id,
            provider_name,
            provider_type,
            provider_base_url,
            model_id,
            model_name,
            call_index,
            request_message_count,
            prompt_chars,
            response_chars,
            request_snapshot_path,
            response_snapshot_path,
            max_tokens,
            temperature,
            token_count_input,
            token_count_output,
            duration_ms,
            status,
            error_message,
            created_at
         FROM model_calls
         ORDER BY created_at DESC, id DESC
         LIMIT ?",
    )
    .bind(bounded_limit)
    .fetch_all(pool)
    .await
    .map_err(|e| e.into())
}

pub async fn get_model_call(pool: &SqlitePool, id: &str) -> Result<ModelCall, AppError> {
    sqlx::query_as::<_, ModelCall>(
        "SELECT
            id,
            source_kind,
            source_id,
            source_label,
            workflow_run_id,
            agent_run_id,
            work_item_id,
            product_id,
            session_id,
            agent_id,
            stage,
            provider_id,
            provider_name,
            provider_type,
            provider_base_url,
            model_id,
            model_name,
            call_index,
            request_message_count,
            prompt_chars,
            response_chars,
            request_snapshot_path,
            response_snapshot_path,
            max_tokens,
            temperature,
            token_count_input,
            token_count_output,
            duration_ms,
            status,
            error_message,
            created_at
         FROM model_calls
         WHERE id=?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Model call {id} not found")))
}

pub async fn list_model_calls_for_workflow(
    pool: &SqlitePool,
    workflow_run_id: &str,
) -> Result<Vec<ModelCall>, AppError> {
    sqlx::query_as::<_, ModelCall>(
        "SELECT
            id,
            source_kind,
            source_id,
            source_label,
            workflow_run_id,
            agent_run_id,
            work_item_id,
            product_id,
            session_id,
            agent_id,
            stage,
            provider_id,
            provider_name,
            provider_type,
            provider_base_url,
            model_id,
            model_name,
            call_index,
            request_message_count,
            prompt_chars,
            response_chars,
            request_snapshot_path,
            response_snapshot_path,
            max_tokens,
            temperature,
            token_count_input,
            token_count_output,
            duration_ms,
            status,
            error_message,
            created_at
         FROM model_calls
         WHERE workflow_run_id=?
         ORDER BY created_at ASC, agent_run_id ASC, call_index ASC",
    )
    .bind(workflow_run_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.into())
}
