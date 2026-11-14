use crate::persistence::model_repo;
use crate::state::AppState;

pub(crate) async fn resolve_mobile_chat_provider_id(
    state: &AppState,
    requested_provider_id: Option<String>,
    requested_model_name: Option<&str>,
) -> Result<String, String> {
    if let Some(provider_id) = requested_provider_id.filter(|value| !value.trim().is_empty()) {
        return Ok(provider_id);
    }
    let providers = model_repo::list_providers(&state.db)
        .await
        .map_err(|error| error.to_string())?
        .into_iter()
        .filter(|provider| provider.enabled && provider.provider_type.as_str() != "local_runtime")
        .collect::<Vec<_>>();
    let models = model_repo::list_model_definitions(&state.db).await.map_err(|error| error.to_string())?;
    if let Some(name) = requested_model_name.filter(|name| !name.trim().is_empty()) {
        let matching: Vec<_> = providers.iter().filter(|provider| models.iter().any(|model|
            model.enabled && model.provider_id == provider.id && model.name == name)).collect();
        match matching.as_slice() {
            [provider] => return Ok(provider.id.clone()),
            [] if models.iter().any(|model| model.name == name) => return Err("The requested model has no enabled text provider and model binding.".into()),
            [] => return match providers.as_slice() {
                [provider] => Ok(provider.id.clone()),
                [] => Err("No enabled model provider is configured.".into()),
                _ => Err("The requested model is not registered; select a provider explicitly.".into()),
            },
            _ => return Err("The requested model is registered with multiple providers; select a provider explicitly.".into()),
        }
    } else {
        if let Some(provider) = providers.iter().find(|provider| models.iter().any(|model|
            model.enabled && model.provider_id == provider.id && !crate::services::speech_service::is_voice_only_selector(&model.name))) {
            return Ok(provider.id.clone());
        }
    }
    providers.into_iter().next().map(|provider| provider.id)
        .ok_or_else(|| "No enabled model provider is configured.".to_string())
}

pub(crate) async fn resolve_mobile_chat_model_name(
    state: &AppState,
    provider_id: &str,
    requested_model_name: Option<String>,
) -> Result<String, String> {
    if let Some(model_name) = requested_model_name.filter(|value| !value.trim().is_empty()) {
        if crate::services::speech_service::is_voice_only_selector(&model_name) {
            return Err("Voice-only selectors cannot be used for text chat.".into());
        }
        return Ok(model_name);
    }
    model_repo::list_model_definitions(&state.db)
        .await
        .map_err(|error| error.to_string())?
        .into_iter()
        .find(|model| model.enabled && model.provider_id == provider_id
            && !crate::services::speech_service::is_voice_only_selector(&model.name))
        .map(|model| model.name)
        .ok_or_else(|| "No enabled model is configured for the selected provider.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tauri::Manager;

    #[tokio::test]
    async fn automatic_provider_skips_voice_only_models_without_changing_explicit_selection() {
        let app = crate::commands::test_helpers::make_test_app("mobile_default_provider").await;
        let state = app.state::<AppState>();
        sqlx::query("UPDATE model_providers SET enabled=0").execute(&state.db).await.unwrap();
        model_repo::create_provider(&state.db, "voice", "A Voice", "openai_compatible", "http://example.invalid", None).await.unwrap();
        model_repo::create_provider(&state.db, "text", "B Text", "openai_compatible", "http://example.invalid", None).await.unwrap();
        model_repo::create_model_definition(&state.db, "voice-model", "voice", "deepseek-voice-transcription", None, None, None).await.unwrap();
        model_repo::create_model_definition(&state.db, "text-model", "text", "chat-model", None, None, None).await.unwrap();
        assert_eq!(resolve_mobile_chat_provider_id(&state, None, None).await.unwrap(), "text");
        assert_eq!(resolve_mobile_chat_model_name(&state, "text", None).await.unwrap(), "chat-model");
        assert_eq!(resolve_mobile_chat_provider_id(&state, Some("voice".into()), None).await.unwrap(), "voice");
        assert!(resolve_mobile_chat_model_name(&state, "voice", None).await.is_err());
        assert!(resolve_mobile_chat_provider_id(&state, None, Some("unregistered-model")).await.unwrap_err().contains("select a provider explicitly"));
        sqlx::query("UPDATE model_providers SET enabled=0 WHERE id='text'").execute(&state.db).await.unwrap();
        assert_eq!(resolve_mobile_chat_provider_id(&state, None, Some("unregistered-model")).await.unwrap(), "voice");
        sqlx::query("UPDATE model_providers SET enabled=1 WHERE id='text'").execute(&state.db).await.unwrap();
        assert_eq!(resolve_mobile_chat_provider_id(&state, None, Some("chat-model")).await.unwrap(), "text");
        model_repo::create_model_definition(&state.db, "duplicate-model", "voice", "chat-model", None, None, None).await.unwrap();
        assert!(resolve_mobile_chat_provider_id(&state, None, Some("chat-model")).await.unwrap_err().contains("multiple providers"));
        sqlx::query("UPDATE model_definitions SET enabled=0 WHERE name='chat-model'").execute(&state.db).await.unwrap();
        assert!(resolve_mobile_chat_provider_id(&state, None, Some("chat-model")).await.unwrap_err().contains("no enabled"));
    }
}
