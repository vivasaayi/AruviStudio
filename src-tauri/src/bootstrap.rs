use crate::persistence::db;
use crate::state::AppState;
use directories::ProjectDirs;
use std::path::PathBuf;

pub async fn initialize_app_state() -> Result<AppState, Box<dyn std::error::Error>> {
    let proj_dirs = ProjectDirs::from("com", "aruvi", "studio").ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "project dirs unavailable")
    })?;
    let data_dir = proj_dirs.data_dir();
    std::fs::create_dir_all(data_dir)?;

    let db_path = resolve_database_path(data_dir)?;
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let db_url = format!("sqlite:{}", db_path.display());
    let pool = db::create_pool(&db_url).await?;
    AppState::new(pool, data_dir.to_path_buf()).await
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
