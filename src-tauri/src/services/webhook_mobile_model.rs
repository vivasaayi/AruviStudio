use crate::persistence::model_repo;
use crate::state::AppState;

pub(crate) async fn resolve_mobile_chat_provider_id(
    state: &AppState,
    requested_provider_id: Option<String>,
) -> Result<String, String> {
    if let Some(provider_id) = requested_provider_id.filter(|value| !value.trim().is_empty()) {
        return Ok(provider_id);
    }
    model_repo::list_providers(&state.db)
        .await
        .map_err(|error| error.to_string())?
        .into_iter()
        .find(|provider| provider.enabled)
        .map(|provider| provider.id)
        .ok_or_else(|| "No enabled model provider is configured.".to_string())
}

pub(crate) async fn resolve_mobile_chat_model_name(
    state: &AppState,
    provider_id: &str,
    requested_model_name: Option<String>,
) -> Result<String, String> {
    if let Some(model_name) = requested_model_name.filter(|value| !value.trim().is_empty()) {
        return Ok(model_name);
    }
    model_repo::list_model_definitions(&state.db)
        .await
        .map_err(|error| error.to_string())?
        .into_iter()
        .find(|model| model.enabled && model.provider_id == provider_id)
        .map(|model| model.name)
        .ok_or_else(|| "No enabled model is configured for the selected provider.".to_string())
}
