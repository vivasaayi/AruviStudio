use crate::error::AppError;
use crate::persistence::model_call_repo;
use crate::providers::types::ChatMessage;
use crate::state::AppState;
use std::time::Instant;

pub(crate) fn char_count_i64(content: &str) -> i64 {
    i64::try_from(content.chars().count()).unwrap_or(i64::MAX)
}

fn message_char_count(messages: &[ChatMessage]) -> i64 {
    messages
        .iter()
        .map(|message| char_count_i64(&message.content))
        .sum()
}

pub(crate) fn elapsed_ms(started: Instant) -> i64 {
    i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX)
}

pub(crate) struct WebhookModelCallContext<'a> {
    pub(crate) source_kind: &'a str,
    pub(crate) source_id: Option<&'a str>,
    pub(crate) source_label: &'a str,
    pub(crate) session_id: Option<&'a str>,
    pub(crate) product_id: Option<&'a str>,
}

pub(crate) struct WebhookModelCallRecord<'a> {
    pub(crate) context: WebhookModelCallContext<'a>,
    pub(crate) provider: &'a crate::domain::model::ModelProvider,
    pub(crate) model_name: &'a str,
    pub(crate) messages: &'a [ChatMessage],
    pub(crate) max_tokens: Option<i64>,
    pub(crate) temperature: Option<f64>,
    pub(crate) response_chars: i64,
    pub(crate) token_count_input: Option<i64>,
    pub(crate) token_count_output: Option<i64>,
    pub(crate) duration_ms: i64,
    pub(crate) status: &'a str,
    pub(crate) error_message: Option<&'a str>,
    pub(crate) response_text: Option<&'a str>,
}

pub(crate) async fn record_webhook_model_call(
    state: &AppState,
    record: WebhookModelCallRecord<'_>,
) -> Result<(), AppError> {
    let call_index = model_call_repo::next_model_call_index(
        &state.db,
        record.context.source_kind,
        record.context.source_id,
    )
    .await?;
    let call_id = uuid::Uuid::new_v4().to_string();
    let request_messages_json = serde_json::to_string_pretty(record.messages)?;
    let snapshots = model_call_repo::write_model_call_snapshots(
        &state.artifact_base_path,
        &call_id,
        Some(&request_messages_json),
        record.response_text,
    )
    .await?;
    model_call_repo::create_model_call(
        &state.db,
        model_call_repo::CreateModelCallParams {
            id: &call_id,
            source_kind: record.context.source_kind,
            source_id: record.context.source_id,
            source_label: record.context.source_label,
            workflow_run_id: None,
            agent_run_id: None,
            work_item_id: None,
            product_id: record.context.product_id,
            session_id: record.context.session_id,
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
