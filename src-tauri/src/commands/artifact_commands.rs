use crate::domain::artifact::Artifact;
use crate::error::AppError;
use crate::persistence::artifact_repo;
use crate::state::AppState;
use tauri::State;
use tracing::debug;

#[tauri::command]
pub async fn list_work_item_artifacts(
    state: State<'_, AppState>,
    work_item_id: Option<String>,
) -> Result<Vec<Artifact>, AppError> {
    let work_item_id =
        work_item_id.ok_or_else(|| AppError::Validation("missing work item id".to_string()))?;
    debug!(work_item_id = %work_item_id, "list_work_item_artifacts requested");
    artifact_repo::list_work_item_artifacts(&state.db, &work_item_id).await
}

#[tauri::command]
pub async fn read_artifact_content(
    state: State<'_, AppState>,
    artifact_id: String,
) -> Result<String, AppError> {
    let artifact = artifact_repo::get_artifact(&state.db, &artifact_id).await?;
    crate::services::artifact_service::read_text(
        &state.artifact_base_path,
        std::path::Path::new(&artifact.storage_path),
        crate::services::artifact_service::TEXT_LIMIT,
    ).await
}

#[tauri::command]
pub async fn read_artifact_page(
    state: State<'_, AppState>, artifact_id: String, offset: u64, version: Option<String>,
) -> Result<crate::services::artifact_service::ArtifactPage, AppError> {
    let artifact = artifact_repo::get_artifact(&state.db, &artifact_id).await?;
    crate::services::artifact_service::read_page(
        &state.artifact_base_path, std::path::Path::new(&artifact.storage_path), offset, version.as_deref(),
    ).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tauri::test::{mock_builder, mock_context, noop_assets, INVOKE_KEY};

    #[test]
    fn desktop_ipc_reads_artifact_pages() {
        let root = tempfile::tempdir().unwrap();
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let state = runtime.block_on(async {
            let db = crate::persistence::db::create_pool(&format!("sqlite:{}", root.path().join("db").display())).await.unwrap();
            let state = AppState::new(db, root.path().to_path_buf()).await.unwrap();
            let item: String = sqlx::query_scalar("SELECT id FROM work_items LIMIT 1").fetch_one(&state.db).await.unwrap();
            let file = state.artifact_base_path.join("large.txt");
            tokio::fs::write(&file, "x".repeat(70_000)).await.unwrap();
            artifact_repo::create_artifact(&state.db, artifact_repo::CreateArtifactInput {
                id:"desktop-page", work_item_id:&item, workflow_run_id:None, agent_run_id:None,
                artifact_type:"coding_output", summary:"page fixture", storage_path:file.to_str().unwrap(),
            }).await.unwrap();
            state
        });
        let app = mock_builder().manage(state).invoke_handler(tauri::generate_handler![super::read_artifact_page])
            .build(mock_context(noop_assets())).unwrap();
        let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default()).build().unwrap();
        let call = |body| tauri::test::get_ipc_response(&webview, tauri::webview::InvokeRequest {
            cmd:"read_artifact_page".into(), callback:tauri::ipc::CallbackFn(0), error:tauri::ipc::CallbackFn(1),
            url:"http://tauri.localhost".parse().unwrap(), body:tauri::ipc::InvokeBody::Json(body),
            headers:Default::default(), invoke_key:INVOKE_KEY.to_string(),
        });
        let first = call(serde_json::json!({"artifactId":"desktop-page", "offset":0, "version":null})).unwrap().deserialize::<serde_json::Value>().unwrap();
        assert_eq!(first["content"].as_str().unwrap().len(), 65_536);
        let second = call(serde_json::json!({"artifactId":"desktop-page", "offset":first["next_offset"], "version":first["version"]})).unwrap().deserialize::<serde_json::Value>().unwrap();
        assert_eq!(second["content"].as_str().unwrap().len(), 4_464);
        assert!(second["next_offset"].is_null());
    }
}
