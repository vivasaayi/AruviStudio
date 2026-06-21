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
