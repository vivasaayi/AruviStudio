use directories::ProjectDirs;
use std::path::PathBuf;
use std::sync::OnceLock;

const DEFAULT_QUALIFIER: &str = "com";
const DEFAULT_ORGANIZATION: &str = "aruvi";
const DEFAULT_APPLICATION: &str = "studio";
const DEFAULT_KEYCHAIN_SERVICE: &str = "com.aruvi.studio";
const DEFAULT_BUNDLE_IDENTIFIER: &str = "com.aruvi.studio";
const LOCAL_RELEASE_BUNDLE_IDENTIFIER: &str = "com.aruvi.studio.localrelease";

#[derive(Debug, Clone)]
pub struct RuntimeProfile {
    pub profile: Option<String>,
    pub data_dir: PathBuf,
    pub keychain_service: String,
}

static RUNTIME_PROFILE: OnceLock<RuntimeProfile> = OnceLock::new();

pub fn initialize_runtime_profile(
    app_identifier: Option<&str>,
) -> Result<&'static RuntimeProfile, std::io::Error> {
    if let Some(profile) = RUNTIME_PROFILE.get() {
        return Ok(profile);
    }

    let profile = resolve_runtime_profile(app_identifier)?;
    let _ = RUNTIME_PROFILE.set(profile);
    RUNTIME_PROFILE.get().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            "runtime profile initialization failed",
        )
    })
}

pub fn current_profile() -> Option<String> {
    RUNTIME_PROFILE
        .get()
        .and_then(|runtime| runtime.profile.clone())
        .or_else(|| resolve_profile(None))
}

pub fn current_app_data_dir() -> Result<PathBuf, std::io::Error> {
    if let Some(runtime) = RUNTIME_PROFILE.get() {
        return Ok(runtime.data_dir.clone());
    }
    resolve_runtime_profile(None).map(|runtime| runtime.data_dir)
}

pub fn current_keychain_service_name() -> String {
    RUNTIME_PROFILE
        .get()
        .map(|runtime| runtime.keychain_service.clone())
        .unwrap_or_else(|| resolve_keychain_service_name(resolve_profile(None).as_deref()))
}

pub fn default_webhook_port(profile: Option<&str>) -> u16 {
    match profile {
        Some("local-release") => 8788,
        Some(_) => 8788,
        None => 8787,
    }
}

fn resolve_runtime_profile(app_identifier: Option<&str>) -> Result<RuntimeProfile, std::io::Error> {
    let profile = resolve_profile(app_identifier);
    let data_dir = resolve_app_data_dir(profile.as_deref())?;
    let keychain_service = resolve_keychain_service_name(profile.as_deref());

    Ok(RuntimeProfile {
        profile,
        data_dir,
        keychain_service,
    })
}

fn resolve_profile(app_identifier: Option<&str>) -> Option<String> {
    std::env::var("ARUVI_PROFILE")
        .ok()
        .and_then(|value| normalize_profile(&value))
        .or_else(|| profile_from_bundle_identifier(app_identifier))
}

fn profile_from_bundle_identifier(app_identifier: Option<&str>) -> Option<String> {
    let identifier = app_identifier?.trim();
    match identifier {
        "" | DEFAULT_BUNDLE_IDENTIFIER => None,
        LOCAL_RELEASE_BUNDLE_IDENTIFIER => Some("local-release".to_string()),
        _ => identifier
            .strip_prefix("com.aruvi.studio.")
            .and_then(normalize_profile),
    }
}

fn resolve_app_data_dir(profile: Option<&str>) -> Result<PathBuf, std::io::Error> {
    if let Some(path) = std::env::var_os("ARUVI_APP_DATA_DIR")
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
    {
        return Ok(path);
    }

    if profile == Some("local-release") {
        if let Some(path) = default_local_release_data_dir() {
            return Ok(path);
        }
    }

    let application = project_application_name(profile);
    let proj_dirs = ProjectDirs::from(DEFAULT_QUALIFIER, DEFAULT_ORGANIZATION, &application)
        .ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "project dirs unavailable")
        })?;
    Ok(proj_dirs.data_dir().to_path_buf())
}

fn default_local_release_data_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from).map(|home| {
        home.join("work")
            .join("releases")
            .join("aruvi-studio-local-data")
    })
}

fn resolve_keychain_service_name(profile: Option<&str>) -> String {
    if let Some(service) = std::env::var("ARUVI_KEYCHAIN_SERVICE")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        return service;
    }

    profile
        .map(|profile| format!("{DEFAULT_KEYCHAIN_SERVICE}.{profile}"))
        .unwrap_or_else(|| DEFAULT_KEYCHAIN_SERVICE.to_string())
}

fn project_application_name(profile: Option<&str>) -> String {
    profile
        .map(|profile| format!("{DEFAULT_APPLICATION}.{profile}"))
        .unwrap_or_else(|| DEFAULT_APPLICATION.to_string())
}

fn normalize_profile(raw: &str) -> Option<String> {
    let mut output = String::new();
    let mut previous_was_separator = false;

    for character in raw.trim().chars() {
        if character.is_ascii_alphanumeric() {
            output.push(character.to_ascii_lowercase());
            previous_was_separator = false;
        } else if character == '-'
            || character == '_'
            || character == '.'
            || character.is_whitespace()
        {
            if !output.is_empty() && !previous_was_separator {
                output.push('-');
                previous_was_separator = true;
            }
        }
    }

    while output.ends_with('-') {
        output.pop();
    }

    if output.is_empty() {
        None
    } else {
        Some(output)
    }
}

#[cfg(test)]
mod tests {
    use super::{default_webhook_port, normalize_profile, profile_from_bundle_identifier};

    #[test]
    fn normalizes_profile_for_paths_and_services() {
        assert_eq!(
            normalize_profile(" Local.Release "),
            Some("local-release".to_string())
        );
        assert_eq!(
            normalize_profile("qa_candidate 01"),
            Some("qa-candidate-01".to_string())
        );
        assert_eq!(normalize_profile("..."), None);
    }

    #[test]
    fn resolves_known_bundle_profiles() {
        assert_eq!(
            profile_from_bundle_identifier(Some("com.aruvi.studio")),
            None
        );
        assert_eq!(
            profile_from_bundle_identifier(Some("com.aruvi.studio.localrelease")),
            Some("local-release".to_string())
        );
        assert_eq!(
            profile_from_bundle_identifier(Some("com.aruvi.studio.qa")),
            Some("qa".to_string())
        );
    }

    #[test]
    fn uses_separate_profile_webhook_port() {
        assert_eq!(default_webhook_port(None), 8787);
        assert_eq!(default_webhook_port(Some("local-release")), 8788);
    }
}
