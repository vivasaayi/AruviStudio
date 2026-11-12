use crate::app_paths;
use crate::domain::model::ModelProvider;
use crate::error::AppError;
use keyring::Entry;
use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};
use std::io::Write;
use std::sync::{Mutex, MutexGuard};
use tracing::{debug, info, warn};

static CONFIG_WRITE_LOCK: Mutex<()> = Mutex::new(());

fn llm_config_primary_path() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join(".aruvistudio").join("llm-config.json"))
}

fn llm_config_secondary_path() -> Option<PathBuf> {
    app_paths::current_app_data_dir()
        .ok()
        .map(|data_dir| data_dir.join("llm-config.json"))
}

fn llm_config_override_path() -> Option<PathBuf> {
    std::env::var_os("ARUVI_LLM_CONFIG_PATH")
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
}

fn read_config_from_path(path: &Path) -> Option<Value> {
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str::<Value>(&content).ok()
}

fn read_config() -> Option<Value> {
    if let Some(path) = llm_config_override_path() {
        return read_config_from_path(&path);
    }

    if app_paths::current_profile().is_some() {
        return llm_config_secondary_path().and_then(|path| read_config_from_path(&path));
    }

    llm_config_primary_path()
        .and_then(|path| read_config_from_path(&path))
        .or_else(|| llm_config_secondary_path().and_then(|path| read_config_from_path(&path)))
}

fn writable_config_path() -> Result<PathBuf, AppError> {
    if let Some(path) = llm_config_override_path() { return Ok(path); }
    if app_paths::current_profile().is_some() {
        return llm_config_secondary_path().ok_or_else(|| AppError::Internal("Unable to resolve a writable profile path for llm-config.json".into()));
    }
    llm_config_primary_path().or_else(llm_config_secondary_path).ok_or_else(|| AppError::Internal("Unable to resolve a writable path for llm-config.json".into()))
}

fn config_for_write(path: &Path) -> Result<Value, AppError> {
    match std::fs::read_to_string(path) {
        Ok(content) => Ok(serde_json::from_str(&content)?),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            if llm_config_override_path().is_none() && app_paths::current_profile().is_none() {
                if let Some(secondary) = llm_config_secondary_path().filter(|candidate| candidate != path) {
                    match std::fs::read_to_string(secondary) {
                        Ok(content) => return Ok(serde_json::from_str(&content)?),
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {},
                        Err(error) => return Err(error.into()),
                    }
                }
            }
            Ok(json!({}))
        },
        Err(error) => Err(error.into()),
    }
}

struct ConfigWriteGuard {
    _process_lock: MutexGuard<'static, ()>,
    #[cfg(unix)]
    _file_lock: std::fs::File,
    path: PathBuf,
}

fn lock_config_for_write() -> Result<ConfigWriteGuard, AppError> {
    let process_lock = CONFIG_WRITE_LOCK.lock().map_err(|_| AppError::Internal("Credential configuration lock failed".into()))?;
    let path = writable_config_path()?;
    let parent = path.parent().ok_or_else(|| AppError::Internal("Credential file needs a parent directory".into()))?;
    std::fs::create_dir_all(parent)?;
    #[cfg(unix)]
    let file_lock = {
        use std::os::fd::AsRawFd;
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        let filename = path.file_name().ok_or_else(|| AppError::Internal("Credential file needs a filename".into()))?;
        let mut lock_name = std::ffi::OsString::from("."); lock_name.push(filename); lock_name.push(".lock");
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create(true).mode(0o600).custom_flags(libc::O_NOFOLLOW);
        let file = options.open(parent.join(lock_name))?;
        file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
        loop {
            if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) } == 0 { break; }
            let error = std::io::Error::last_os_error();
            if error.kind() != std::io::ErrorKind::Interrupted { return Err(error.into()); }
        }
        file
    };
    Ok(ConfigWriteGuard { _process_lock: process_lock, #[cfg(unix)] _file_lock: file_lock, path })
}

fn write_config(path: &Path, config: &Value) -> Result<(), AppError> {
    write_private_config(path, serde_json::to_string_pretty(config)?.as_bytes())?;
    Ok(())
}

fn write_private_config(path: &Path, contents: &[u8]) -> Result<(), std::io::Error> {
    let parent = path.parent().ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "Credential file needs a parent directory"))?;
    let temporary = parent.join(format!(".aruvi-credentials-{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| {
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        { use std::os::unix::fs::OpenOptionsExt; options.mode(0o600); }
        let mut file = options.open(&temporary)?;
        #[cfg(unix)]
        { use std::os::unix::fs::PermissionsExt; file.set_permissions(std::fs::Permissions::from_mode(0o600))?; }
        file.write_all(contents)?;
        file.sync_all()?;
        std::fs::rename(&temporary, path)?;
        Ok(())
    })();
    if result.is_err() { let _ = std::fs::remove_file(&temporary); }
    result
}

#[cfg(all(test, unix))]
mod private_config_tests {
    use super::{config_for_write, is_legacy_plaintext_reference, read_from_fallback_value, store_provider_secret, write_private_config, write_to_fallback_config};
    use crate::domain::model::{ModelProvider, ProviderType};
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn fallback_config_is_private_even_when_file_already_exists() {
        let dir = std::env::temp_dir().join(format!("aruvi-secret-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&dir).unwrap();
        let path = dir.join("config.json");
        std::fs::write(&path, b"old content").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        write_private_config(&path, b"new").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"new");
        assert_eq!(std::fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn invalid_existing_config_is_not_overwritten() {
        let dir = std::env::temp_dir().join(format!("aruvi-secret-invalid-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&dir).unwrap();
        let path = dir.join("config.json");
        std::fs::write(&path, b"{invalid").unwrap();
        assert!(config_for_write(&path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"{invalid");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn atomic_config_replacement_never_exposes_partial_data() {
        let dir = std::env::temp_dir().join(format!("aruvi-secret-atomic-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&dir).unwrap();
        let path = dir.join("config.json");
        let first = vec![b'a'; 128 * 1024]; let second = vec![b'b'; 128 * 1024];
        write_private_config(&path, &first).unwrap();
        let target = path.clone(); let next = second.clone();
        let writer = std::thread::spawn(move || { for _ in 0..20 { write_private_config(&target, &next).unwrap(); } });
        for _ in 0..20 { let observed = std::fs::read(&path).unwrap(); assert!(observed == first || observed == second); }
        writer.join().unwrap();
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn missing_managed_reference_does_not_use_another_key() {
        let provider = ModelProvider { id: "id".into(), name: "name".into(), provider_type: ProviderType::OpenaiCompatible,
            base_url: "https://example.invalid".into(), auth_secret_ref: None, enabled: true, created_at: String::new(), updated_at: String::new() };
        let config = serde_json::json!({"refs":{"provider:id:rotation:found":"exact"},"providers":{"id":{"api_key":"old"}},"api_key":"global"});
        assert_eq!(read_from_fallback_value(&provider, Some("provider:id:rotation:found"), &config).as_deref(), Some("exact"));
        assert_eq!(read_from_fallback_value(&provider, Some("provider:id:rotation:missing"), &config), None);
        assert_eq!(read_from_fallback_value(&provider, None, &config).as_deref(), Some("old"));
        let mut explicit = provider.clone();
        explicit.auth_secret_ref = Some("ref:missing-external-reference".into());
        assert_eq!(read_from_fallback_value(&explicit, Some("missing-external-reference"), &config), None);
    }

    #[test]
    fn explicitly_named_references_are_not_treated_as_plaintext() {
        let input = "ref:external-long-reference-with-hyphens";
        assert_eq!(store_provider_secret("id", Some(input)).unwrap().as_deref(), Some(input));
        assert!(!is_legacy_plaintext_reference(input));
        assert!(!is_legacy_plaintext_reference("provider:id:rotation:unique-id"));
        assert!(is_legacy_plaintext_reference("sk-legacy-plaintext-key-123456"));
    }

    #[test]
    fn credential_config_writer_child() {
        let Ok(worker) = std::env::var("ARUVI_TEST_CREDENTIAL_WRITER") else { return; };
        for index in 0..20 {
            let id = format!("{worker}-{index}");
            write_to_fallback_config(&format!("provider:{id}:api_key"), &id, "synthetic-test-value").unwrap();
        }
    }

    #[test]
    fn concurrent_processes_preserve_both_writers() {
        use std::process::{Command, Stdio};
        let dir = std::env::temp_dir().join(format!("aruvi-secret-process-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&dir).unwrap();
        let path = dir.join("config.json");
        let spawn = |name: &str| Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "secrets::private_config_tests::credential_config_writer_child"])
            .env("ARUVI_TEST_CREDENTIAL_WRITER", name).env("ARUVI_LLM_CONFIG_PATH", &path)
            .stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap();
        let mut first = spawn("first"); let mut second = spawn("second");
        assert!(first.wait().unwrap().success()); assert!(second.wait().unwrap().success());
        let data: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(data["refs"].as_object().unwrap().len(), 40);
        assert_eq!(data["providers"].as_object().unwrap().len(), 40);
        assert_eq!(std::fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
        assert_eq!(std::fs::metadata(dir.join(".config.json.lock")).unwrap().permissions().mode() & 0o777, 0o600);
        std::fs::remove_dir_all(dir).unwrap();
    }

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

fn is_legacy_plaintext_reference(raw: &str) -> bool {
    !raw.starts_with("ref:") && !raw.starts_with("provider:") && looks_like_raw_api_key(raw)
}

fn read_from_keychain(secret_ref: &str) -> Option<String> {
    let service_name = app_paths::current_keychain_service_name();
    let entry = Entry::new(&service_name, secret_ref).ok()?;
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
    let service_name = app_paths::current_keychain_service_name();
    let entry = Entry::new(&service_name, secret_ref).map_err(|error| {
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
    let guard = lock_config_for_write()?;
    let mut config = config_for_write(&guard.path)?;
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

    write_config(&guard.path, &config)?;
    info!(provider_id = %provider_id, path = %guard.path.display(), "Stored provider secret in fallback llm-config.json");
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
        return Ok(Some(raw.to_string()));
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

// A new reference avoids changing the active credential until the database update commits.
pub fn store_rotated_provider_secret(provider_id: &str, secret: &str) -> Result<String, AppError> {
    let secret = secret.trim();
    if secret.is_empty() || secret.len() > 16 * 1024 { return Err(AppError::Validation("API key must contain 1 to 16384 bytes".into())); }
    let reference = format!("provider:{provider_id}:rotation:{}", uuid::Uuid::new_v4());
    if write_to_keychain(&reference, secret).is_err() {
        let guard = lock_config_for_write()?;
        let mut config = config_for_write(&guard.path)?;
        let root = config.as_object_mut().ok_or_else(|| AppError::Internal("Fallback llm-config root must be an object".into()))?;
        let refs = root.entry("refs").or_insert_with(|| Value::Object(Map::new())).as_object_mut()
            .ok_or_else(|| AppError::Internal("Fallback llm-config.refs must be an object".into()))?;
        refs.insert(reference.clone(), Value::String(secret.to_string()));
        write_config(&guard.path, &config)?;
    }
    Ok(reference)
}

// Remove unreferenced entries matching the rotation reference format from the fallback file.
// A manually named entry can match this format, so this is only called on explicit request.
fn prune_rotation_refs(config: &mut Value, provider_id: &str, protected: &std::collections::HashSet<String>) -> Result<usize, AppError> {
    let canonical = |value: &str| uuid::Uuid::parse_str(value).is_ok_and(|id| id.to_string() == value);
    if !canonical(provider_id) { return Err(AppError::Validation("Provider ID is not a canonical UUID".into())); }
    let root = config.as_object_mut().ok_or_else(|| AppError::Internal("Fallback llm-config root must be an object".into()))?;
    let Some(refs_value) = root.get_mut("refs") else { return Ok(0); };
    let refs = refs_value.as_object_mut().ok_or_else(|| AppError::Internal("Fallback llm-config.refs must be an object".into()))?;
    let prefix = format!("provider:{provider_id}:rotation:");
    let before = refs.len();
    refs.retain(|key, _| !key.strip_prefix(&prefix).is_some_and(canonical) || protected.contains(key));
    Ok(before - refs.len())
}

pub fn cleanup_unreferenced_rotated_fallback_keys(provider_id: &str, protected: &std::collections::HashSet<String>) -> Result<usize, AppError> {
    let guard = lock_config_for_write()?;
    let mut config = config_for_write(&guard.path)?;
    let removed = prune_rotation_refs(&mut config, provider_id, protected)?;
    if removed > 0 { write_config(&guard.path, &config)?; }
    Ok(removed)
}

fn read_from_fallback_config(provider: &ModelProvider, secret_ref: Option<&str>) -> Option<String> {
    read_from_fallback_value(provider, secret_ref, &read_config()?)
}

fn read_from_fallback_value(provider: &ModelProvider, secret_ref: Option<&str>, config: &Value) -> Option<String> {
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

    // A managed reference identifies a specific credential. Never fall back to a
    // different provider or global key if that reference cannot be resolved.
    if secret_ref.is_some_and(|reference| reference.starts_with("provider:"))
        || provider.auth_secret_ref.as_deref().is_some_and(|reference| reference.starts_with("ref:")) { return None; }

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
        if provider.auth_secret_ref.as_deref().is_some_and(is_legacy_plaintext_reference) {
            // Legacy plaintext compatibility path; managed references are never plaintext.
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

#[cfg(test)]
mod cleanup_tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn cleanup_preserves_protected_and_noncanonical_references() {
        let id = "11111111-1111-4111-8111-111111111111";
        let old = format!("provider:{id}:rotation:22222222-2222-4222-8222-222222222222");
        let active = format!("provider:{id}:rotation:33333333-3333-4333-8333-333333333333");
        let custom = format!("provider:{id}:rotation:44444444444444448444444444444444");
        let legacy = format!("provider:{id}:api_key");
        let mut config = serde_json::json!({"refs": {(&old): "old", (&active): "active", (&custom): "custom", (&legacy): "legacy"}, "providers": {(id): "provider"}});
        assert_eq!(prune_rotation_refs(&mut config, id, &HashSet::from([active.clone()])).unwrap(), 1);
        assert!(config["refs"].get(&old).is_none());
        for key in [&active, &custom, &legacy] { assert!(config["refs"].get(key).is_some()); }
        assert!(config["providers"].get(id).is_some());
        assert!(prune_rotation_refs(&mut config, "bad:id", &HashSet::new()).is_err());
    }
}
