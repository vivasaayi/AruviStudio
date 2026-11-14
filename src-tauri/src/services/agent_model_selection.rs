use crate::domain::model::ModelDefinition;
use crate::error::AppError;
use crate::persistence::{agent_repo, model_repo};
use sqlx::SqlitePool;
use tracing::{debug, error, info, warn};

pub(crate) async fn find_model_for_agent(
    db: &SqlitePool,
    agent_id: &str,
) -> Result<ModelDefinition, AppError> {
    debug!(agent_id = %agent_id, "Finding model for agent");
    let agent_definition = agent_repo::get_agent_definition(db, agent_id).await?;

    let bindings = agent_repo::get_agent_model_bindings(db, agent_id).await?;
    if let Some(binding) = bindings.first() {
        let model = model_repo::get_model_definition(db, &binding.model_id).await?;
        if !model.enabled {
            return Err(AppError::Validation("The agent's bound model is disabled. Enable it or select another model binding.".into()));
        }
        let provider = model_repo::get_provider(db, &model.provider_id).await?;
        if !provider.enabled {
            return Err(AppError::Validation("The agent's bound model provider is disabled. Enable it or select another model binding.".into()));
        }
        debug!(agent_id = %agent_id, model_id = %model.id, model_name = %model.name, "Found model for agent");
        return Ok(model);
    }

    warn!(agent_id = %agent_id, "No direct model binding found for agent, falling back to an enabled shared model");

    let mut shared_models = model_repo::list_model_definitions(db)
        .await?
        .into_iter()
        .filter(|model| model.enabled)
        .collect::<Vec<_>>();

    let preferred_tags = preferred_model_tags_for_role(&agent_definition.role);
    shared_models.sort_by_key(|model| {
        let tag_score = preferred_tags
            .iter()
            .filter(|tag| {
                model
                    .capability_tags
                    .iter()
                    .any(|model_tag| model_tag.trim().eq_ignore_ascii_case(tag))
            })
            .count();
        let lowered = model.name.to_ascii_lowercase();
        let name_bias = if lowered.contains("deepseek-coder") {
            0
        } else if lowered.contains("deepseek") {
            1
        } else {
            2
        };
        (usize::MAX - tag_score, name_bias)
    });

    for model in shared_models {
        let provider = model_repo::get_provider(db, &model.provider_id).await?;
        if provider.enabled {
            info!(
                agent_id = %agent_id,
                model_id = %model.id,
                model_name = %model.name,
                provider_id = %provider.id,
                provider_name = %provider.name,
                preferred_tags = ?preferred_tags,
                matched_tags = ?matched_model_tags(&model, &preferred_tags),
                "Using shared fallback model for agent"
            );
            return Ok(model);
        }
    }

    error!(agent_id = %agent_id, "No enabled model bindings or shared models available for agent");
    Err(AppError::NotFound(format!(
        "No enabled model bindings or shared models available for agent {}",
        agent_id
    )))
}

fn preferred_model_tags_for_role(role: &str) -> Vec<&'static str> {
    match role {
        "coding" => vec!["coding", "implementation", "repo_write", "testing"],
        "planning" => vec!["planning", "analysis", "review"],
        "requirement_analysis" => vec!["analysis", "planning", "review"],
        "unit_test_generation" => vec!["unit_test", "testing", "coding"],
        "integration_test_generation" => vec!["integration_test", "testing", "coding"],
        "ui_test_planning" => vec!["ui_test", "testing", "planning"],
        "qa_validation" => vec!["qa", "validation", "testing"],
        "security_review" => vec!["security", "review", "analysis"],
        "performance_review" => vec!["performance", "review", "analysis"],
        "coordinator_review" | "manager" => vec!["coordination", "planning", "review"],
        _ => vec!["general"],
    }
}

fn matched_model_tags(model: &ModelDefinition, preferred_tags: &[&str]) -> Vec<String> {
    preferred_tags
        .iter()
        .filter(|tag| {
            model
                .capability_tags
                .iter()
                .any(|model_tag| model_tag.trim().eq_ignore_ascii_case(tag))
        })
        .map(|tag| (*tag).to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::{agent_repo, db, model_repo};

    #[tokio::test]
    async fn explicit_binding_requires_enabled_model_and_provider_without_silent_fallback() {
        let directory = tempfile::tempdir().unwrap();
        let pool = db::create_pool(&format!("sqlite:{}", directory.path().join("agent.db").display())).await.unwrap();
        model_repo::create_provider(&pool, "bound-provider", "Bound", "openai_compatible", "http://example.invalid", None).await.unwrap();
        model_repo::create_model_definition(&pool, "bound-model", "bound-provider", "bound", None, None, None).await.unwrap();
        model_repo::create_provider(&pool, "fallback-provider", "Fallback", "openai_compatible", "http://example.invalid", None).await.unwrap();
        model_repo::create_model_definition(&pool, "fallback-model", "fallback-provider", "fallback", None, None, None).await.unwrap();
        agent_repo::replace_primary_agent_model_binding(&pool, "binding", "coding-agent", "bound-model").await.unwrap();
        assert_eq!(find_model_for_agent(&pool, "coding-agent").await.unwrap().id, "bound-model");
        sqlx::query("UPDATE model_definitions SET enabled=0 WHERE id='bound-model'").execute(&pool).await.unwrap();
        assert!(matches!(find_model_for_agent(&pool, "coding-agent").await, Err(AppError::Validation(message)) if message.contains("bound model is disabled")));
        sqlx::query("UPDATE model_definitions SET enabled=1 WHERE id='bound-model'").execute(&pool).await.unwrap();
        sqlx::query("UPDATE model_providers SET enabled=0 WHERE id='bound-provider'").execute(&pool).await.unwrap();
        assert!(matches!(find_model_for_agent(&pool, "coding-agent").await, Err(AppError::Validation(message)) if message.contains("provider is disabled")));
        sqlx::query("UPDATE model_providers SET enabled=1 WHERE id='bound-provider'").execute(&pool).await.unwrap();
        assert_eq!(find_model_for_agent(&pool, "coding-agent").await.unwrap().id, "bound-model");
        pool.close().await;
    }
}
