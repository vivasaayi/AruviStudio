use crate::app_paths;
use crate::persistence::db;
use crate::state::AppState;
use std::path::PathBuf;

pub async fn initialize_app_state(
    app_identifier: Option<&str>,
) -> Result<AppState, Box<dyn std::error::Error>> {
    let runtime_profile = app_paths::initialize_runtime_profile(app_identifier)?;
    let data_dir = runtime_profile.data_dir.clone();
    std::fs::create_dir_all(&data_dir)?;

    let db_path = resolve_database_path(&data_dir)?;
    tracing::info!(database_path = %db_path.display(), "using AruviStudio database");
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let db_url = format!("sqlite:{}", db_path.display());
    let pool = db::create_pool(&db_url).await?;
    AppState::new_with_profile(pool, data_dir, runtime_profile.profile.clone()).await
}

fn resolve_database_path(data_dir: &std::path::Path) -> Result<PathBuf, std::io::Error> {
    let db_override_path = data_dir.join("db_override_path.txt");
    Ok(std::env::var("ARUVI_DB_PATH")
        .ok()
        .map(PathBuf::from)
        .or_else(|| {
            std::fs::read_to_string(&db_override_path)
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .map(PathBuf::from)
        })
        .unwrap_or_else(|| data_dir.join("aruvi_studio.db")))
}
