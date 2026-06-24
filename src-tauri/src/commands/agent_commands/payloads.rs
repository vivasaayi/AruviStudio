use crate::error::AppError;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct CreateAgentDefinitionCommand {
    pub(crate) name: String,
    pub(crate) role: String,
    pub(crate) description: String,
    #[serde(alias = "promptTemplateRef")]
    pub(crate) prompt_template_ref: String,
    #[serde(alias = "allowedTools")]
    pub(crate) allowed_tools: String,
    #[serde(alias = "skillTags")]
    pub(crate) skill_tags: String,
    pub(crate) boundaries: String,
    pub(crate) enabled: bool,
    #[serde(alias = "employmentStatus")]
    pub(crate) employment_status: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAgentDefinitionCommand {
    pub(crate) id: String,
    pub(crate) name: Option<String>,
    pub(crate) role: Option<String>,
    pub(crate) description: Option<String>,
    #[serde(alias = "promptTemplateRef")]
    pub(crate) prompt_template_ref: Option<String>,
    #[serde(alias = "allowedTools")]
    pub(crate) allowed_tools: Option<String>,
    #[serde(alias = "skillTags")]
    pub(crate) skill_tags: Option<String>,
    pub(crate) boundaries: Option<String>,
    pub(crate) enabled: Option<bool>,
    #[serde(alias = "employmentStatus")]
    pub(crate) employment_status: Option<String>,
}

pub(crate) fn validate_json_array(label: &str, value: &str) -> Result<(), AppError> {
    let parsed = serde_json::from_str::<serde_json::Value>(value)?;
    if !parsed.is_array() {
        return Err(AppError::Validation(format!(
            "{label} must be a JSON array"
        )));
    }
    Ok(())
}

pub(crate) fn validate_json_object(label: &str, value: &str) -> Result<(), AppError> {
    let parsed = serde_json::from_str::<serde_json::Value>(value)?;
    if !parsed.is_object() {
        return Err(AppError::Validation(format!(
            "{label} must be a JSON object"
        )));
    }
    Ok(())
}

pub(crate) fn resolve_required(
    value: Option<String>,
    field_name: &str,
) -> Result<String, AppError> {
    value.ok_or_else(|| AppError::Validation(format!("missing {}", field_name)))
}
