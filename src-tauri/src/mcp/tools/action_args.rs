use crate::error::AppError;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

pub(super) struct ToolAction {
    pub(super) action: String,
    arguments: Value,
}

impl ToolAction {
    pub(super) fn parse(payload: Value) -> Result<Self, AppError> {
        let object = payload.as_object().ok_or_else(|| {
            AppError::Validation("tool payload must be a JSON object".to_string())
        })?;
        let root = ActionArgs { object };
        let action = root.required_string(&["action"], "action")?;
        let arguments = match object.get("arguments") {
            Some(Value::Object(map)) => Value::Object(map.clone()),
            Some(Value::Null) | None => Value::Object(Map::new()),
            Some(_) => {
                return Err(AppError::Validation(
                    "tool payload arguments must be a JSON object".to_string(),
                ))
            }
        };

        Ok(Self { action, arguments })
    }

    pub(super) fn args(&self) -> ActionArgs<'_> {
        ActionArgs {
            object: self
                .arguments
                .as_object()
                .expect("arguments must be object"),
        }
    }
}

pub(super) struct ActionArgs<'a> {
    object: &'a Map<String, Value>,
}

impl<'a> ActionArgs<'a> {
    fn get(&self, keys: &[&str]) -> Option<&Value> {
        for key in keys {
            if let Some(value) = self.object.get(*key) {
                return Some(value);
            }
        }
        None
    }

    pub(super) fn required_string(&self, keys: &[&str], label: &str) -> Result<String, AppError> {
        self.optional_string(keys)?
            .ok_or_else(|| AppError::Validation(format!("missing {label}")))
    }

    pub(super) fn optional_string(&self, keys: &[&str]) -> Result<Option<String>, AppError> {
        match self.get(keys) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::String(value)) => Ok(Some(value.to_string())),
            Some(Value::Number(value)) => Ok(Some(value.to_string())),
            Some(Value::Bool(value)) => Ok(Some(value.to_string())),
            Some(_) => Err(AppError::Validation(format!(
                "{} must be a string",
                keys[0]
            ))),
        }
    }

    pub(super) fn string_or_default(
        &self,
        keys: &[&str],
        default: &str,
    ) -> Result<String, AppError> {
        Ok(self
            .optional_string(keys)?
            .unwrap_or_else(|| default.to_string()))
    }

    pub(super) fn optional_bool(&self, keys: &[&str]) -> Result<Option<bool>, AppError> {
        match self.get(keys) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::Bool(value)) => Ok(Some(*value)),
            Some(Value::String(value)) => match value.trim().to_ascii_lowercase().as_str() {
                "true" => Ok(Some(true)),
                "false" => Ok(Some(false)),
                _ => Err(AppError::Validation(format!(
                    "{} must be a boolean",
                    keys[0]
                ))),
            },
            Some(_) => Err(AppError::Validation(format!(
                "{} must be a boolean",
                keys[0]
            ))),
        }
    }

    pub(super) fn bool_or_default(&self, keys: &[&str], default: bool) -> Result<bool, AppError> {
        Ok(self.optional_bool(keys)?.unwrap_or(default))
    }

    pub(super) fn optional_i64(&self, keys: &[&str]) -> Result<Option<i64>, AppError> {
        match self.get(keys) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::Number(value)) => value
                .as_i64()
                .ok_or_else(|| AppError::Validation(format!("{} must be an integer", keys[0])))
                .map(Some),
            Some(Value::String(value)) => value
                .trim()
                .parse::<i64>()
                .map(Some)
                .map_err(|_| AppError::Validation(format!("{} must be an integer", keys[0]))),
            Some(_) => Err(AppError::Validation(format!(
                "{} must be an integer",
                keys[0]
            ))),
        }
    }

    pub(super) fn optional_i32(&self, keys: &[&str]) -> Result<Option<i32>, AppError> {
        self.optional_i64(keys)?
            .map(|value| {
                i32::try_from(value).map_err(|_| {
                    AppError::Validation(format!("{} is out of range for i32", keys[0]))
                })
            })
            .transpose()
    }

    pub(super) fn optional_f64(&self, keys: &[&str]) -> Result<Option<f64>, AppError> {
        match self.get(keys) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::Number(value)) => value
                .as_f64()
                .ok_or_else(|| AppError::Validation(format!("{} must be a number", keys[0])))
                .map(Some),
            Some(Value::String(value)) => value
                .trim()
                .parse::<f64>()
                .map(Some)
                .map_err(|_| AppError::Validation(format!("{} must be a number", keys[0]))),
            Some(_) => Err(AppError::Validation(format!(
                "{} must be a number",
                keys[0]
            ))),
        }
    }

    pub(super) fn required_string_list(
        &self,
        keys: &[&str],
        label: &str,
    ) -> Result<Vec<String>, AppError> {
        self.optional_string_list(keys)?
            .ok_or_else(|| AppError::Validation(format!("missing {label}")))
    }

    pub(super) fn optional_string_list(
        &self,
        keys: &[&str],
    ) -> Result<Option<Vec<String>>, AppError> {
        match self.get(keys) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::Array(values)) => values
                .iter()
                .map(|value| {
                    value
                        .as_str()
                        .map(|value| value.to_string())
                        .ok_or_else(|| {
                            AppError::Validation(format!("{} must contain only strings", keys[0]))
                        })
                })
                .collect::<Result<Vec<_>, _>>()
                .map(Some),
            Some(Value::String(value)) => {
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    return Ok(Some(Vec::new()));
                }
                if trimmed.starts_with('[') {
                    let parsed = serde_json::from_str::<Vec<String>>(trimmed)?;
                    Ok(Some(parsed))
                } else {
                    Ok(Some(
                        trimmed
                            .split(',')
                            .map(str::trim)
                            .filter(|item| !item.is_empty())
                            .map(ToString::to_string)
                            .collect(),
                    ))
                }
            }
            Some(_) => Err(AppError::Validation(format!(
                "{} must be an array of strings",
                keys[0]
            ))),
        }
    }

    pub(super) fn optional_json_array_string(
        &self,
        keys: &[&str],
    ) -> Result<Option<String>, AppError> {
        self.optional_string_list(keys)?
            .map(|value| serde_json::to_string(&value).map_err(AppError::from))
            .transpose()
    }

    pub(super) fn optional_json_object_string(
        &self,
        keys: &[&str],
    ) -> Result<Option<String>, AppError> {
        match self.get(keys) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::Object(value)) => serde_json::to_string(value)
                .map(Some)
                .map_err(AppError::from),
            Some(Value::String(value)) => {
                let parsed = serde_json::from_str::<Value>(value)?;
                if !parsed.is_object() {
                    return Err(AppError::Validation(format!(
                        "{} must be a JSON object",
                        keys[0]
                    )));
                }
                serde_json::to_string(&parsed)
                    .map(Some)
                    .map_err(AppError::from)
            }
            Some(_) => Err(AppError::Validation(format!(
                "{} must be a JSON object",
                keys[0]
            ))),
        }
    }

    pub(super) fn optional_deserialize<T: DeserializeOwned>(
        &self,
        keys: &[&str],
        label: &str,
    ) -> Result<Option<T>, AppError> {
        match self.get(keys) {
            None | Some(Value::Null) => Ok(None),
            Some(value) => serde_json::from_value::<T>(value.clone())
                .map(Some)
                .map_err(|error| AppError::Validation(format!("invalid {label}: {error}"))),
        }
    }

    pub(super) fn required_deserialize<T: DeserializeOwned>(
        &self,
        keys: &[&str],
        label: &str,
    ) -> Result<T, AppError> {
        self.optional_deserialize(keys, label)?
            .ok_or_else(|| AppError::Validation(format!("missing {label}")))
    }
}
