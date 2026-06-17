use crate::domain::events::DomainEvent;
use crate::persistence::{bulk_import_repo, external_cli_repo};
use crate::services::{
    agent_service, model_service, planner_service, product_service, workflow_service,
};
use sqlx::SqlitePool;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub app_data_dir: PathBuf,
    pub app_profile: Option<String>,
    pub event_tx: broadcast::Sender<DomainEvent>,
    pub artifact_base_path: PathBuf,
    pub workspace_base_path: PathBuf,
    pub workflow_service: Arc<Mutex<workflow_service::WorkflowService>>,
    pub agent_service: Arc<Mutex<agent_service::AgentService>>,
    pub model_service: Arc<model_service::ModelService>,
    pub planner_service: Arc<Mutex<planner_service::PlannerService>>,
}

impl AppState {
    pub async fn new(
        db: SqlitePool,
        app_data_dir: PathBuf,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Self::new_with_profile(db, app_data_dir, None).await
    }

    pub async fn new_with_profile(
        db: SqlitePool,
        app_data_dir: PathBuf,
        app_profile: Option<String>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let (event_tx, _) = broadcast::channel(100);

        let artifact_base_path = app_data_dir.join("artifacts");
        let workspace_base_path = app_data_dir.join("workspaces");

        // Create directories if they don't exist
        tokio::fs::create_dir_all(&artifact_base_path).await?;
        tokio::fs::create_dir_all(&workspace_base_path).await?;
        external_cli_repo::mark_interrupted_external_cli_runs(&db).await?;
        bulk_import_repo::mark_interrupted_jobs(&db).await?;

        // Initialize services
        let db_arc = Arc::new(db);
        let model_service = Arc::new(model_service::ModelService::new(db_arc.clone()));
        let agent_service = Arc::new(Mutex::new(agent_service::AgentService::new(
            db_arc.clone(),
            model_service.clone(),
            artifact_base_path.clone(),
            workspace_base_path.clone(),
        )));
        let workflow_service = Arc::new(Mutex::new(workflow_service::WorkflowService::new(
            db_arc.clone(),
            agent_service.clone(),
        )));
        let planner_service = Arc::new(Mutex::new(planner_service::PlannerService::new()));

        product_service::initialize_example_catalog(db_arc.as_ref()).await?;

        Ok(Self {
            db: (*db_arc).clone(),
            app_data_dir,
            app_profile,
            event_tx,
            artifact_base_path,
            workspace_base_path,
            workflow_service,
            agent_service,
            model_service,
            planner_service,
        })
    }
}
