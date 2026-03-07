use crate::domain::model::ModelProvider;
use crate::error::AppError;
use keyring::Entry;
use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

const KEYCHAIN_SERVICE_NAME: &str = "com.aruvi.studio";

fn llm_config_primary_path() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join(".aruvistudio").join("llm-config.json"))
}

fn llm_config_secondary_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("com", "aruvi", "studio")
        .map(|dirs| dirs.data_dir().join("llm-config.json"))
}

fn read_config_from_path(path: &Path) -> Option<Value> {
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str::<Value>(&content).ok()
}

fn read_config() -> Option<Value> {
    llm_config_primary_path()
        .and_then(|path| read_config_from_path(&path))
        .or_else(|| llm_config_secondary_path().and_then(|path| read_config_from_path(&path)))
}

fn write_config(config: &Value) -> Result<PathBuf, AppError> {
    let path = if let Some(primary) = llm_config_primary_path() {
        primary
    } else if let Some(secondary) = llm_config_secondary_path() {
        secondary
    } else {
        return Err(AppError::Internal(
            "Unable to resolve a writable path for llm-config.json".to_string(),
        ));
    };

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, serde_json::to_string_pretty(config)?)?;
    Ok(path)
}

fn read_key_from_value(value: &Value) -> Option<String> {
    match value {
        Value::String(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        Value::Object(object) => {
            for key in ["api_key", "key", "token", "secret"] {
                if let Some(entry) = object.get(key) {
                    if let Some(secret) = read_key_from_value(entry) {
                        return Some(secret);
                    }
                }
            }
            None
        }
        _ => None,
    }
}

fn looks_like_raw_api_key(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.len() < 20 || trimmed.contains(' ') {
        return false;
    }
    trimmed.contains('-') || trimmed.starts_with("sk_") || trimmed.starts_with("sk-")
}

fn normalize_secret_ref(value: &str) -> &str {
    value.strip_prefix("ref:").unwrap_or(value)
}

fn read_from_keychain(secret_ref: &str) -> Option<String> {
    let entry = Entry::new(KEYCHAIN_SERVICE_NAME, secret_ref).ok()?;
    match entry.get_password() {
        Ok(secret) if !secret.trim().is_empty() => Some(secret),
        Ok(_) => None,
        Err(error) => {
            warn!(secret_ref = %secret_ref, error = %error, "Failed to read secret from keychain");
            None
        }
    }
}

fn write_to_keychain(secret_ref: &str, secret_value: &str) -> Result<(), AppError> {
    let entry = Entry::new(KEYCHAIN_SERVICE_NAME, secret_ref).map_err(|error| {
        AppError::Internal(format!("Unable to initialize keychain entry: {}", error))
    })?;
    entry.set_password(secret_value).map_err(|error| {
        AppError::Internal(format!("Unable to save secret in keychain: {}", error))
    })
}

fn write_to_fallback_config(
    secret_ref: &str,
    provider_id: &str,
    secret_value: &str,
) -> Result<(), AppError> {
    let mut config = read_config().unwrap_or_else(|| json!({}));
    let root = config.as_object_mut().ok_or_else(|| {
        AppError::Internal("Fallback llm-config root must be an object".to_string())
    })?;

    let refs = root
        .entry("refs")
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .ok_or_else(|| {
            AppError::Internal("Fallback llm-config.refs must be an object".to_string())
        })?;
    refs.insert(
        secret_ref.to_string(),
        Value::String(secret_value.to_string()),
    );

    let providers = root
        .entry("providers")
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .ok_or_else(|| {
            AppError::Internal("Fallback llm-config.providers must be an object".to_string())
        })?;
    providers.insert(provider_id.to_string(), json!({ "api_key": secret_value }));

    let target_path = write_config(&config)?;
    info!(provider_id = %provider_id, path = %target_path.display(), "Stored provider secret in fallback llm-config.json");
    Ok(())
}

pub fn store_provider_secret(
    provider_id: &str,
    raw_input: Option<&str>,
) -> Result<Option<String>, AppError> {
    let Some(raw) = raw_input.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };

    let secret_ref = normalize_secret_ref(&format!("provider:{provider_id}:api_key")).to_string();

    // If the user intentionally passed a reference, keep it as-is and do not overwrite.
    if raw.starts_with("ref:") {
        return Ok(Some(normalize_secret_ref(raw).to_string()));
    }

    match write_to_keychain(&secret_ref, raw) {
        Ok(()) => {
            debug!(provider_id = %provider_id, secret_ref = %secret_ref, "Stored provider secret in keychain");
            Ok(Some(secret_ref))
        }
        Err(error) => {
            warn!(provider_id = %provider_id, secret_ref = %secret_ref, error = %error, "Keychain storage failed; writing secret to fallback config");
            write_to_fallback_config(&secret_ref, provider_id, raw)?;
            Ok(Some(secret_ref))
        }
    }
}

fn read_from_fallback_config(provider: &ModelProvider, secret_ref: Option<&str>) -> Option<String> {
    let config = read_config()?;
    let root = config.as_object()?;

    if let Some(reference) = secret_ref {
        if let Some(secret) = root
            .get("refs")
            .and_then(|value| value.as_object())
            .and_then(|refs| refs.get(reference))
            .and_then(read_key_from_value)
        {
            return Some(secret);
        }
    }

    if let Some(secret) = root
        .get("providers")
        .and_then(|value| value.as_object())
        .and_then(|providers| {
            providers
                .get(&provider.id)
                .or_else(|| providers.get(&provider.name))
                .or_else(|| providers.get(&provider.base_url))
        })
        .and_then(read_key_from_value)
    {
        return Some(secret);
    }

    if let Some(secret) = root
        .get("api_keys")
        .and_then(|value| value.as_object())
        .and_then(|api_keys| {
            api_keys
                .get(&provider.id)
                .or_else(|| api_keys.get(&provider.name))
                .or_else(|| api_keys.get(&provider.base_url))
                .or_else(|| {
                    if provider.base_url.to_lowercase().contains("deepseek") {
                        api_keys.get("deepseek")
                    } else {
                        None
                    }
                })
                .or_else(|| api_keys.get("default"))
        })
        .and_then(read_key_from_value)
    {
        return Some(secret);
    }

    if provider.base_url.to_lowercase().contains("deepseek") {
        if let Some(secret) = root.get("deepseek").and_then(read_key_from_value) {
            return Some(secret);
        }
    }

    root.get("api_key").and_then(read_key_from_value)
}

pub fn resolve_provider_secret(provider: &ModelProvider) -> Result<Option<String>, AppError> {
    let normalized_ref = provider
        .auth_secret_ref
        .as_deref()
        .map(normalize_secret_ref)
        .map(str::to_string);

    if let Some(reference) = normalized_ref.as_deref() {
        if looks_like_raw_api_key(reference) {
            // Legacy plaintext compatibility path.
            return Ok(Some(reference.to_string()));
        }

        if let Some(secret) = read_from_keychain(reference) {
            return Ok(Some(secret));
        }
    }

    Ok(read_from_fallback_config(
        provider,
        normalized_ref.as_deref(),
    ))
}
