use crate::error::AppError;
use crate::persistence::planner_repo;
use crate::services::planner_service::{PlannerDraftPlan, PlannerPlan};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerSessionInfo {
    pub session_id: String,
    pub provider_id: Option<String>,
    pub model_name: Option<String>,
    pub has_pending_plan: bool,
    pub has_draft_plan: bool,
    pub selected_draft_node_id: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct PlannerConversationEntry {
    pub(crate) role: String,
    pub(crate) content: String,
}

#[derive(Debug, Clone)]
pub(crate) struct PlannerSession {
    pub(crate) provider_id: Option<String>,
    pub(crate) model_name: Option<String>,
    pub(crate) pending_plan: Option<PlannerPlan>,
    pub(crate) draft_plan: Option<PlannerDraftPlan>,
    pub(crate) selected_draft_node_id: Option<String>,
    pub(crate) conversation: Vec<PlannerConversationEntry>,
}

pub struct PlannerService {
    sessions: HashMap<String, PlannerSession>,
}

impl PlannerService {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    pub(crate) fn create_session(
        &mut self,
        provider_id: Option<String>,
        model_name: Option<String>,
    ) -> PlannerSessionInfo {
        let session_id = uuid::Uuid::new_v4().to_string();
        self.sessions.insert(
            session_id.clone(),
            PlannerSession {
                provider_id: provider_id.clone(),
                model_name: model_name.clone(),
                pending_plan: None,
                draft_plan: None,
                selected_draft_node_id: None,
                conversation: vec![],
            },
        );
        PlannerSessionInfo {
            session_id,
            provider_id,
            model_name,
            has_pending_plan: false,
            has_draft_plan: false,
            selected_draft_node_id: None,
        }
    }

    pub(crate) fn update_session(
        &mut self,
        session_id: &str,
        provider_id: Option<String>,
        model_name: Option<String>,
    ) -> Result<PlannerSessionInfo, AppError> {
        let session = self.sessions.get_mut(session_id).ok_or_else(|| {
            AppError::NotFound(format!("Planner session {} not found", session_id))
        })?;
        session.provider_id = provider_id.clone();
        session.model_name = model_name.clone();
        Ok(PlannerSessionInfo {
            session_id: session_id.to_string(),
            provider_id,
            model_name,
            has_pending_plan: session.pending_plan.is_some(),
            has_draft_plan: session.draft_plan.is_some(),
            selected_draft_node_id: session.selected_draft_node_id.clone(),
        })
    }

    pub(crate) fn clear_pending(
        &mut self,
        session_id: &str,
    ) -> Result<PlannerSessionInfo, AppError> {
        let session = self.sessions.get_mut(session_id).ok_or_else(|| {
            AppError::NotFound(format!("Planner session {} not found", session_id))
        })?;
        session.pending_plan = None;
        session.draft_plan = None;
        session.selected_draft_node_id = None;
        Ok(PlannerSessionInfo {
            session_id: session_id.to_string(),
            provider_id: session.provider_id.clone(),
            model_name: session.model_name.clone(),
            has_pending_plan: false,
            has_draft_plan: false,
            selected_draft_node_id: None,
        })
    }

    pub(crate) fn get_session(&self, session_id: &str) -> Result<PlannerSession, AppError> {
        self.sessions
            .get(session_id)
            .cloned()
            .ok_or_else(|| AppError::NotFound(format!("Planner session {} not found", session_id)))
    }

    pub(crate) fn save_session(&mut self, session_id: &str, session: PlannerSession) {
        self.sessions.insert(session_id.to_string(), session);
    }
}

pub(crate) async fn get_or_load_session(
    planner_service: &Arc<Mutex<PlannerService>>,
    db: &SqlitePool,
    session_id: &str,
) -> Result<PlannerSession, AppError> {
    let mut service = planner_service.lock().await;
    match service.get_session(session_id) {
        Ok(session) => Ok(session),
        Err(_) => {
            let loaded = load_session_from_db(db, session_id).await?;
            service.save_session(session_id, loaded.clone());
            Ok(loaded)
        }
    }
}

pub(crate) async fn load_session_from_db(
    db: &SqlitePool,
    session_id: &str,
) -> Result<PlannerSession, AppError> {
    let record = planner_repo::get_session(db, session_id).await?;
    let conversation = planner_repo::list_conversation_entries(db, session_id)
        .await?
        .into_iter()
        .map(|entry| PlannerConversationEntry {
            role: entry.role,
            content: entry.content,
        })
        .collect::<Vec<_>>();
    let pending_plan = match record.pending_plan_json {
        Some(value) => Some(serde_json::from_str::<PlannerPlan>(&value)?),
        None => None,
    };
    let draft_plan = match record.draft_plan_json {
        Some(value) => Some(serde_json::from_str::<PlannerDraftPlan>(&value)?),
        None => None,
    };
    Ok(PlannerSession {
        provider_id: record.provider_id,
        model_name: record.model_name,
        pending_plan,
        draft_plan,
        selected_draft_node_id: record.selected_draft_node_id,
        conversation,
    })
}

pub(crate) async fn persist_pending_plan(
    db: &SqlitePool,
    session_id: &str,
    pending_plan: Option<&PlannerPlan>,
) -> Result<(), AppError> {
    let serialized = pending_plan.map(serde_json::to_string).transpose()?;
    planner_repo::update_pending_plan(db, session_id, serialized.as_deref()).await?;
    Ok(())
}

pub(crate) async fn persist_draft_state(
    db: &SqlitePool,
    session_id: &str,
    draft_plan: Option<&PlannerDraftPlan>,
    selected_draft_node_id: Option<&str>,
) -> Result<(), AppError> {
    let serialized = draft_plan.map(serde_json::to_string).transpose()?;
    planner_repo::update_draft_state(
        db,
        session_id,
        serialized.as_deref(),
        selected_draft_node_id,
    )
    .await?;
    Ok(())
}

pub(crate) async fn append_conversation(
    db: &SqlitePool,
    session_id: &str,
    role: &str,
    content: &str,
) -> Result<(), AppError> {
    planner_repo::append_conversation_entry(
        db,
        &uuid::Uuid::new_v4().to_string(),
        session_id,
        role,
        content,
    )
    .await?;
    Ok(())
}
