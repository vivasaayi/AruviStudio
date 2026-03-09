use crate::domain::agent::{AgentDefinition, AgentRun, AgentRunStatus, AgentTeam};
use crate::domain::model::ModelDefinition;
use crate::domain::work_item::WorkItem;
use crate::domain::workflow::WorkflowRun;
use crate::error::AppError;
use crate::execution::workspace::WorkItemWorkspace;
use crate::persistence::{agent_repo, artifact_repo, model_repo, work_item_repo, workflow_repo};
use crate::providers::types::{ChatMessage, CompletionRequest};
use crate::services::model_service;
use crate::services::repo_service;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::SqlitePool;
#[cfg(test)]
use std::collections::VecDeque;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
#[cfg(test)]
use std::sync::{Mutex as StdMutex, OnceLock};
use tracing::{debug, error, info, warn};
use uuid;
use walkdir::WalkDir;

pub struct AgentService {
    db: Arc<SqlitePool>,
    model_service: Arc<model_service::ModelService>,
    artifact_base_path: PathBuf,
    workspace_base_path: PathBuf,
}

struct TeamExecutionContext {
    team: Option<AgentTeam>,
    coordinator: Option<AgentDefinition>,
    stage_agent: AgentDefinition,
    expected_role: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct AgentExecutionBoundaries {
    instructions: Option<String>,
    max_tokens: Option<i64>,
    max_context_chars: Option<usize>,
    context_window_ratio: Option<f32>,
    max_files_per_run: Option<usize>,
    max_read_file_chars: Option<usize>,
    max_write_file_chars: Option<usize>,
    max_file_chars: Option<usize>,
    max_repo_files_scanned: Option<usize>,
    allowed_paths: Option<Vec<String>>,
    blocked_paths: Option<Vec<String>>,
    keep_workspace: Option<bool>,
    max_tool_steps: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ToolLoopResponse {
    ToolCall {
        tool: String,
        arguments: Value,
        reason: Option<String>,
    },
    Final {
        summary: Option<String>,
        result: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize)]
struct ToolLoopTraceEntry {
    step: usize,
    kind: String,
    payload: String,
}

#[cfg(test)]
static TEST_MODEL_OUTPUTS: OnceLock<StdMutex<HashMap<String, VecDeque<String>>>> = OnceLock::new();
#[cfg(test)]
const TEST_ANY_WORKFLOW_KEY: &str = "__any_workflow__";

impl AgentService {
    #[cfg(test)]
    pub(crate) fn set_test_model_outputs_for_workflow(workflow_run_id: &str, outputs: Vec<String>) {
        let map = TEST_MODEL_OUTPUTS.get_or_init(|| StdMutex::new(HashMap::new()));
        let mut guard = map.lock().expect("failed to lock test model output queue");
        if outputs.is_empty() {
            guard.remove(workflow_run_id);
        } else {
            guard.insert(workflow_run_id.to_string(), VecDeque::from(outputs));
        }
    }

    #[cfg(test)]
    pub(crate) fn set_test_model_outputs_for_any_workflow(outputs: Vec<String>) {
        Self::set_test_model_outputs_for_workflow(TEST_ANY_WORKFLOW_KEY, outputs);
    }

    pub fn new(
        db: Arc<SqlitePool>,
        model_service: Arc<model_service::ModelService>,
        artifact_base_path: PathBuf,
        workspace_base_path: PathBuf,
    ) -> Self {
        Self {
            db,
            model_service,
            artifact_base_path,
            workspace_base_path,
        }
    }

    fn is_coordinator_role(role: &str) -> bool {
        matches!(
            role.to_ascii_lowercase().as_str(),
            "manager" | "team_lead" | "coordinator"
        )
    }

    fn parse_boundaries(raw: &serde_json::Value) -> AgentExecutionBoundaries {
        serde_json::from_value::<AgentExecutionBoundaries>(raw.clone()).unwrap_or_default()
    }

    fn resolve_context_char_budget(
        model_def: &ModelDefinition,
        boundaries: &AgentExecutionBoundaries,
    ) -> usize {
        if let Some(explicit) = boundaries.max_context_chars {
            return explicit.clamp(4_000, 120_000);
        }
        let ratio = boundaries
            .context_window_ratio
            .unwrap_or(0.25)
            .clamp(0.1, 0.9) as f64;
        if let Some(context_window) = model_def.context_window {
            let estimated_chars = ((context_window as f64) * ratio * 4.0) as usize;
            return estimated_chars.clamp(4_000, 120_000);
        }
        12_000
    }

    fn resolve_response_token_budget(
        model_def: &ModelDefinition,
        boundaries: &AgentExecutionBoundaries,
    ) -> i64 {
        let requested = boundaries.max_tokens.unwrap_or(4_096).clamp(256, 16_384);
        if let Some(context_window) = model_def.context_window {
            let ceiling = ((context_window as f64) * 0.4) as i64;
            return requested.min(ceiling.max(512));
        }
        requested
    }

    fn max_files_per_run(boundaries: &AgentExecutionBoundaries) -> usize {
        boundaries.max_files_per_run.unwrap_or(3).clamp(1, 200)
    }

    fn max_read_file_chars(boundaries: &AgentExecutionBoundaries) -> usize {
        boundaries
            .max_read_file_chars
            .unwrap_or(16_000)
            .clamp(400, 500_000)
    }

    fn max_write_file_chars(boundaries: &AgentExecutionBoundaries) -> usize {
        boundaries
            .max_write_file_chars
            .unwrap_or(200_000)
            .clamp(400, 2_000_000)
    }

    fn max_repo_snippet_chars(boundaries: &AgentExecutionBoundaries) -> usize {
        boundaries
            .max_file_chars
            .unwrap_or(6_000)
            .clamp(200, 80_000)
    }

    fn max_tool_steps(boundaries: &AgentExecutionBoundaries) -> usize {
        boundaries.max_tool_steps.unwrap_or(24).clamp(2, 64)
    }

    fn should_keep_workspace(boundaries: &AgentExecutionBoundaries) -> bool {
        boundaries.keep_workspace.unwrap_or(true)
    }

    fn char_count(content: &str) -> usize {
        content.chars().count()
    }

    fn substring_by_char_range(content: &str, offset_chars: usize, length_chars: usize) -> String {
        content
            .chars()
            .skip(offset_chars)
            .take(length_chars)
            .collect::<String>()
    }

    fn split_lines_preserve_trailing(content: &str) -> (Vec<String>, bool) {
        let has_trailing_newline = content.ends_with('\n');
        let mut lines = content
            .split('\n')
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>();
        if has_trailing_newline && lines.last().is_some_and(|value| value.is_empty()) {
            lines.pop();
        }
        (lines, has_trailing_newline)
    }

    fn ensure_write_limit(
        content: &str,
        boundaries: &AgentExecutionBoundaries,
    ) -> Result<(), AppError> {
        let chars = Self::char_count(content);
        let limit = Self::max_write_file_chars(boundaries);
        if chars > limit {
            return Err(AppError::Validation(format!(
                "File content exceeds max_write_file_chars ({} > {})",
                chars, limit
            )));
        }
        Ok(())
    }

    fn normalize_relative_path(path: &str) -> Option<String> {
        let normalized = path
            .replace('\\', "/")
            .trim()
            .trim_start_matches("./")
            .to_string();
        if normalized.is_empty() {
            return None;
        }
        let candidate = Path::new(&normalized);
        if candidate.is_absolute() {
            return None;
        }
        let mut cleaned: Vec<String> = Vec::new();
        for component in candidate.components() {
            match component {
                std::path::Component::Normal(part) => {
                    cleaned.push(part.to_string_lossy().to_string())
                }
                std::path::Component::CurDir => {}
                _ => return None,
            }
        }
        if cleaned.is_empty() {
            return None;
        }
        Some(cleaned.join("/"))
    }

    fn is_repo_relative_path_allowed(path: &str, boundaries: &AgentExecutionBoundaries) -> bool {
        let Some(normalized) = Self::normalize_relative_path(path) else {
            return false;
        };
        if let Some(blocked_paths) = &boundaries.blocked_paths {
            if blocked_paths
                .iter()
                .any(|blocked| normalized.starts_with(blocked.trim_start_matches("./")))
            {
                return false;
            }
        }
        if let Some(allowed_paths) = &boundaries.allowed_paths {
            if allowed_paths.is_empty() {
                return true;
            }
            return allowed_paths
                .iter()
                .any(|allowed| normalized.starts_with(allowed.trim_start_matches("./")));
        }
        true
    }

    /// Run an agent for a specific workflow stage
    pub async fn run_agent_for_stage(
        &self,
        workflow_run_id: &str,
        stage_name: &str,
    ) -> Result<AgentRun, AppError> {
        info!(
            "Running agent for stage {} in workflow {}",
            stage_name, workflow_run_id
        );

        // Get workflow and work item context
        let workflow_run = workflow_repo::get_workflow_run(&self.db, workflow_run_id).await?;
        let work_item = work_item_repo::get_work_item(&self.db, &workflow_run.work_item_id).await?;

        // Resolve team ownership first, then assign the stage to the best agent inside that team.
        let execution_context = self
            .resolve_execution_context(&work_item, &workflow_run, stage_name)
            .await?;
        workflow_repo::set_workflow_assignment(
            &self.db,
            workflow_run_id,
            execution_context.team.as_ref().map(|team| team.id.as_str()),
            execution_context
                .coordinator
                .as_ref()
                .map(|agent| agent.id.as_str()),
        )
        .await?;
        let model_def = self
            .find_model_for_agent(&execution_context.stage_agent.id)
            .await?;
        let boundaries = Self::parse_boundaries(&execution_context.stage_agent.boundaries);
        let context_budget_chars = Self::resolve_context_char_budget(&model_def, &boundaries);
        let response_token_budget = Self::resolve_response_token_budget(&model_def, &boundaries);

        // Create agent run record
        let agent_run_id = uuid::Uuid::new_v4().to_string();
        let agent_run = agent_repo::create_agent_run(
            &self.db,
            &agent_run_id,
            workflow_run_id,
            &workflow_run.work_item_id,
            &execution_context.stage_agent.id,
            &model_def.id,
            stage_name,
        )
        .await?;

        self.store_manager_handoff(&work_item, &workflow_run, stage_name, &execution_context)
            .await?;

        // Build context and prompt
        let context = self
            .build_agent_context(
                &work_item,
                &workflow_run,
                stage_name,
                &execution_context,
                &boundaries,
                context_budget_chars,
            )
            .await?;
        let prompt = self.build_agent_prompt(
            &execution_context.stage_agent,
            &context,
            stage_name,
            &execution_context,
            &boundaries,
        )?;

        // Execute the agent
        let execution_result = if stage_name == "coding" {
            self.execute_coding_with_tools(
                &agent_run,
                &model_def,
                &prompt,
                &execution_context.stage_agent,
                &boundaries,
                response_token_budget,
            )
            .await
        } else {
            match self
                .execute_agent_run(&agent_run, &model_def, &prompt, response_token_budget)
                .await
            {
                Ok(output) => {
                    self.process_agent_output(&agent_run, &output, stage_name)
                        .await
                }
                Err(error) => Err(error),
            }
        };

        match execution_result {
            Ok(processed_output) => {
                // Store the output as an artifact
                self.store_agent_output(&agent_run, &prompt, &processed_output)
                    .await?;

                // Update agent run as completed
                agent_repo::update_agent_run_status(
                    &self.db,
                    &agent_run_id,
                    AgentRunStatus::Completed,
                )
                .await?;

                let mut completed_run = agent_run.clone();
                completed_run.status = AgentRunStatus::Completed;
                Ok(completed_run)
            }
            Err(e) => {
                error!("Agent execution failed: {}", e);
                let error_message = e.to_string();
                let failure_output = format!("Agent stage failed:\n{error_message}");
                if let Err(store_error) = self
                    .store_agent_output(&agent_run, &prompt, &failure_output)
                    .await
                {
                    warn!(
                        agent_run_id = %agent_run_id,
                        stage_name = %stage_name,
                        error = %store_error,
                        "Failed to persist prompt/output artifacts for failed agent run"
                    );
                }
                agent_repo::update_agent_run_failure(&self.db, &agent_run_id, &error_message)
                    .await?;
                Err(e)
            }
        }
    }

    /// Find the appropriate agent for a stage
    async fn find_agent_for_stage(
        &self,
        stage_name: &str,
        team: Option<&AgentTeam>,
    ) -> Result<(AgentDefinition, String), AppError> {
        debug!(stage_name = %stage_name, team_id = ?team.as_ref().map(|entry| entry.id.as_str()), "Finding agent for stage");
        let expected_roles = self.stage_role_candidates(stage_name).await;

        if let Some(team) = team {
            let team_agents = agent_repo::list_agents_for_team(&self.db, &team.id).await?;
            if let Some(agent) = self.select_agent_for_roles(team_agents, &expected_roles) {
                let matched_role = agent.role.clone();
                debug!(stage_name = %stage_name, team_id = %team.id, agent_id = %agent.id, matched_role = %matched_role, "Found team agent for stage");
                return Ok((agent, matched_role));
            }
        }

        let all_agents = agent_repo::list_agent_definitions(&self.db).await?;
        if let Some(agent) = self.select_agent_for_roles(all_agents, &expected_roles) {
            let matched_role = agent.role.clone();
            debug!(stage_name = %stage_name, agent_id = %agent.id, matched_role = %matched_role, "Found fallback agent for stage");
            return Ok((agent, matched_role));
        }

        error!(stage_name = %stage_name, expected_roles = ?expected_roles, "No enabled agent found for stage");
        Err(AppError::NotFound(format!(
            "No enabled agent found for stage {} (expected roles: {})",
            stage_name,
            expected_roles.join(", ")
        )))
    }

    /// Find the best model for an agent
    async fn find_model_for_agent(&self, agent_id: &str) -> Result<ModelDefinition, AppError> {
        debug!(agent_id = %agent_id, "Finding model for agent");
        // Get agent-model bindings
        let bindings = agent_repo::get_agent_model_bindings(&self.db, agent_id).await?;
        if let Some(binding) = bindings.first() {
            let model = model_repo::get_model_definition(&self.db, &binding.model_id).await?;
            debug!(agent_id = %agent_id, model_id = %model.id, model_name = %model.name, "Found model for agent");
            return Ok(model);
        }

        warn!(agent_id = %agent_id, "No direct model binding found for agent, falling back to an enabled shared model");

        let mut shared_models = model_repo::list_model_definitions(&self.db)
            .await?
            .into_iter()
            .filter(|model| model.enabled)
            .collect::<Vec<_>>();

        shared_models.sort_by_key(|model| {
            let lowered = model.name.to_ascii_lowercase();
            if lowered.contains("deepseek-coder") {
                0
            } else if lowered.contains("deepseek") {
                1
            } else {
                2
            }
        });

        for model in shared_models {
            let provider = model_repo::get_provider(&self.db, &model.provider_id).await?;
            if provider.enabled {
                info!(
                    agent_id = %agent_id,
                    model_id = %model.id,
                    model_name = %model.name,
                    provider_id = %provider.id,
                    provider_name = %provider.name,
                    "Using shared fallback model for agent"
                );
                return Ok(model);
            }
        }

        error!(agent_id = %agent_id, "No enabled model bindings or shared models available for agent");
        Err(AppError::NotFound(format!(
            "No enabled model bindings or shared models available for agent {}",
            agent_id
        )))
    }

    /// Build context for the agent
    async fn build_agent_context(
        &self,
        work_item: &WorkItem,
        workflow_run: &WorkflowRun,
        stage_name: &str,
        execution_context: &TeamExecutionContext,
        boundaries: &AgentExecutionBoundaries,
        context_budget_chars: usize,
    ) -> Result<HashMap<String, String>, AppError> {
        debug!(work_item_id = %work_item.id, workflow_run_id = %workflow_run.id, stage_name = %stage_name, "Building agent context");

        let mut context = HashMap::new();

        // Basic work item information
        context.insert("work_item_id".to_string(), work_item.id.clone());
        context.insert("work_item_title".to_string(), work_item.title.clone());
        context.insert(
            "work_item_description".to_string(),
            work_item.description.clone(),
        );
        context.insert(
            "work_item_problem_statement".to_string(),
            work_item.problem_statement.clone(),
        );
        context.insert(
            "work_item_acceptance_criteria".to_string(),
            work_item.acceptance_criteria.clone(),
        );
        context.insert(
            "work_item_constraints".to_string(),
            work_item.constraints.clone(),
        );
        context.insert(
            "expected_stage_role".to_string(),
            execution_context.expected_role.clone(),
        );

        if let Some(team) = &execution_context.team {
            context.insert("assigned_team".to_string(), team.name.clone());
            context.insert("team_department".to_string(), team.department.clone());
            context.insert("team_description".to_string(), team.description.clone());
        }
        if let Some(coordinator) = &execution_context.coordinator {
            context.insert("team_coordinator".to_string(), coordinator.name.clone());
            context.insert(
                "team_coordinator_role".to_string(),
                coordinator.role.clone(),
            );
        }

        // Get related artifacts from previous stages
        let artifacts = artifact_repo::list_work_item_artifacts(&self.db, &work_item.id).await?;
        debug!(work_item_id = %work_item.id, artifact_count = artifacts.len(), "Retrieved artifacts for work item");

        for artifact in &artifacts {
            if artifact.artifact_type.ends_with("_output") {
                context.insert(artifact.artifact_type.clone(), artifact.summary.clone());
            }
        }

        // Stage-specific context
        match stage_name {
            "requirement_analysis" => {
                // Include product/module/capability hierarchy if available
                if let Some(product_id) = &work_item.product_id {
                    context.insert("product_id".to_string(), product_id.clone());
                }
                if let Some(module_id) = &work_item.module_id {
                    context.insert("module_id".to_string(), module_id.clone());
                }
                if let Some(capability_id) = &work_item.capability_id {
                    context.insert("capability_id".to_string(), capability_id.clone());
                }
            }
            "planning" => {
                // Include requirement analysis results
                if let Some(req_analysis) = artifacts.iter().find(|a| {
                    a.artifact_type == "requirement_analysis_output"
                        || a.artifact_type == "requirement_analysis"
                }) {
                    context.insert(
                        "requirement_analysis".to_string(),
                        req_analysis.summary.clone(),
                    );
                }
            }
            "coding" => {
                // Include planning results and repository info
                if let Some(plan) = artifacts.iter().find(|a| {
                    a.artifact_type == "planning_output"
                        || a.artifact_type == "plan"
                        || a.artifact_type == "planning"
                }) {
                    context.insert("implementation_plan".to_string(), plan.summary.clone());
                }
                if let Some(repo_id) = &work_item.active_repo_id {
                    context.insert("repository_id".to_string(), repo_id.clone());
                    if let Ok(repo) =
                        crate::persistence::repository_repo::get_repository(&self.db, repo_id).await
                    {
                        let repo_context = self.build_repository_context(
                            &repo.local_path,
                            boundaries,
                            context_budget_chars,
                        )?;
                        if !repo_context.is_empty() {
                            context.insert("repository_context".to_string(), repo_context);
                        }
                    }
                }
            }
            _ => {}
        }

        let skills = agent_repo::list_skills(&self.db).await?;
        let skill_name_by_id = skills
            .into_iter()
            .map(|skill| (skill.id, skill.name))
            .collect::<HashMap<_, _>>();

        let agent_skill_links = agent_repo::list_agent_skill_links(&self.db).await?;
        let agent_skills = agent_skill_links
            .into_iter()
            .filter(|link| link.agent_id == execution_context.stage_agent.id)
            .filter_map(|link| {
                skill_name_by_id
                    .get(&link.skill_id)
                    .map(|name| format!("{name} ({})", link.proficiency))
            })
            .collect::<Vec<_>>();
        if !agent_skills.is_empty() {
            context.insert("agent_skills".to_string(), agent_skills.join(", "));
        }

        if let Some(team) = &execution_context.team {
            let team_skill_links = agent_repo::list_team_skill_links(&self.db).await?;
            let team_skills = team_skill_links
                .into_iter()
                .filter(|link| link.team_id == team.id)
                .filter_map(|link| skill_name_by_id.get(&link.skill_id).cloned())
                .collect::<Vec<_>>();
            if !team_skills.is_empty() {
                context.insert("team_skills".to_string(), team_skills.join(", "));
            }
        }

        info!(work_item_id = %work_item.id, workflow_run_id = %workflow_run.id, stage_name = %stage_name, context_keys = context.len(), "Successfully built agent context");
        Ok(context)
    }

    fn build_repository_context(
        &self,
        repo_path: &str,
        boundaries: &AgentExecutionBoundaries,
        context_budget_chars: usize,
    ) -> Result<String, AppError> {
        let repo_root = Path::new(repo_path);
        if !repo_root.exists() || !repo_root.is_dir() {
            return Ok(String::new());
        }

        let max_repo_files_scanned = boundaries
            .max_repo_files_scanned
            .unwrap_or(600)
            .clamp(50, 5_000);
        let mut stack = vec![repo_root.to_path_buf()];
        let mut files: Vec<String> = Vec::new();
        let mut scanned = 0usize;
        let skip_dirs: HashSet<&str> = [
            ".git",
            "node_modules",
            "target",
            "dist",
            "build",
            ".next",
            ".turbo",
            ".idea",
            ".vscode",
        ]
        .into_iter()
        .collect();

        while let Some(dir) = stack.pop() {
            let entries = match std::fs::read_dir(&dir) {
                Ok(read_dir) => read_dir,
                Err(_) => continue,
            };
            for entry in entries.flatten() {
                let path = entry.path();
                let file_name = entry.file_name().to_string_lossy().to_string();
                if path.is_dir() {
                    if skip_dirs.contains(file_name.as_str()) {
                        continue;
                    }
                    stack.push(path);
                    continue;
                }
                if !path.is_file() {
                    continue;
                }
                if scanned >= max_repo_files_scanned {
                    break;
                }
                scanned += 1;
                let rel = match path.strip_prefix(repo_root) {
                    Ok(value) => value.to_string_lossy().replace('\\', "/"),
                    Err(_) => continue,
                };
                if !Self::is_repo_relative_path_allowed(&rel, boundaries) {
                    continue;
                }
                if !Self::is_text_source_file(&rel) {
                    continue;
                }
                files.push(rel);
            }
            if scanned >= max_repo_files_scanned {
                break;
            }
        }

        files.sort();
        if files.is_empty() {
            return Ok(String::new());
        }

        let mut output = String::new();
        output.push_str("Repository file manifest:\n");
        for file in files.iter().take(80) {
            output.push_str("- ");
            output.push_str(file);
            output.push('\n');
        }

        let max_files = Self::max_files_per_run(boundaries);
        let max_file_chars = Self::max_repo_snippet_chars(boundaries);
        let mut used_chars = output.len();
        let mut snippets_added = 0usize;
        let snippet_budget = context_budget_chars
            .saturating_sub(used_chars)
            .min((context_budget_chars / 4).max(2_500))
            .min(6_000);

        let prioritized_files = Self::prioritize_repository_files(&files);
        for rel in &prioritized_files {
            if snippets_added >= max_files || used_chars >= snippet_budget {
                break;
            }
            let file_path = repo_root.join(rel);
            let metadata = match std::fs::metadata(&file_path) {
                Ok(meta) => meta,
                Err(_) => continue,
            };
            if metadata.len() > 300_000 {
                continue;
            }
            let content = match std::fs::read_to_string(&file_path) {
                Ok(value) => value,
                Err(_) => continue,
            };
            if content.trim().is_empty() {
                continue;
            }
            let remaining = snippet_budget.saturating_sub(used_chars);
            if remaining < 100 {
                break;
            }
            let take_chars = remaining.min(max_file_chars);
            let snippet = content.chars().take(take_chars).collect::<String>();
            output.push_str("\nFile Snippet:\nPath: ");
            output.push_str(rel);
            output.push('\n');
            output.push_str(&snippet);
            if snippet.len() < content.len() {
                output.push_str("\n...[truncated]...\n");
            } else {
                output.push('\n');
            }
            used_chars = output.len();
            snippets_added += 1;
        }

        Ok(output)
    }

    fn prioritize_repository_files(files: &[String]) -> Vec<String> {
        let mut scored = files
            .iter()
            .cloned()
            .map(|path| {
                let lower = path.to_ascii_lowercase();
                let score = if lower == "package.json" {
                    0
                } else if lower == "readme.md" {
                    1
                } else if lower == "src/app.tsx"
                    || lower == "src/app.jsx"
                    || lower == "src/app.ts"
                    || lower == "src/app.js"
                {
                    2
                } else if lower == "src/main.tsx"
                    || lower == "src/main.jsx"
                    || lower == "src/main.ts"
                    || lower == "src/main.js"
                    || lower == "src/index.tsx"
                    || lower == "src/index.jsx"
                    || lower == "src/index.ts"
                    || lower == "src/index.js"
                {
                    3
                } else if lower.starts_with("src/") {
                    4
                } else if lower.ends_with(".json") || lower.ends_with(".toml") {
                    5
                } else {
                    6
                };
                (score, path)
            })
            .collect::<Vec<_>>();
        scored.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        scored.into_iter().map(|(_, path)| path).collect()
    }

    fn is_text_source_file(path: &str) -> bool {
        let lowered = path.to_ascii_lowercase();
        let allowed = [
            ".rs", ".ts", ".tsx", ".js", ".jsx", ".json", ".md", ".toml", ".yaml", ".yml", ".sql",
            ".sh", ".py", ".go", ".java", ".kt", ".swift", ".c", ".h", ".cpp", ".hpp", ".css",
            ".scss", ".html", ".xml",
        ];
        allowed.iter().any(|ext| lowered.ends_with(ext))
    }

    /// Build the prompt for the agent
    fn build_agent_prompt(
        &self,
        agent_def: &AgentDefinition,
        context: &HashMap<String, String>,
        stage_name: &str,
        execution_context: &TeamExecutionContext,
        boundaries: &AgentExecutionBoundaries,
    ) -> Result<String, AppError> {
        debug!(agent_id = %agent_def.id, agent_role = %agent_def.role, stage_name = %stage_name, context_keys = context.len(), "Building agent prompt");

        let mut prompt = format!("You are a {} agent. ", agent_def.role);

        if let Some(team) = &execution_context.team {
            prompt.push_str(&format!("You are working inside the {} team. ", team.name));
        }
        if let Some(coordinator) = &execution_context.coordinator {
            prompt.push_str(&format!(
                "The team coordinator for this handoff is {} ({}). Follow their execution lane and keep outputs clean for the next handoff. ",
                coordinator.name, coordinator.role
            ));
        }

        // Add stage-specific instructions
        match stage_name {
            "requirement_analysis" => {
                prompt.push_str(
                    "Analyze the following work item and provide a detailed requirement analysis. ",
                );
                prompt.push_str("Identify any missing information, clarify ambiguities, and suggest improvements. ");
                prompt.push_str(
                    "Consider the broader product context and technical constraints.\n\n",
                );
            }
            "planning" => {
                prompt.push_str("Create a detailed implementation plan for the work item. ");
                prompt.push_str("Break down the work into specific steps, identify files that need to be created or modified, ");
                prompt.push_str("and outline the testing approach.\n\n");
            }
            "coding" => {
                prompt.push_str("Implement the code changes according to the approved plan. ");
                prompt.push_str("Use tool-calling JSON to inspect files, search code, and apply precise edits. ");
                prompt.push_str("Start with minimal context, then fetch additional files on demand through tools. ");
                prompt.push_str("Prefer targeted edits with repo.replace_range for function/class-level changes. ");
                prompt.push_str("Use repo.write_file for full-file rewrites only after reading current content. ");
                prompt.push_str(
                    "Use repo.apply_patch only when context lines are known to match exactly.\n\n",
                );
                prompt.push_str("Response contract for each turn (required):\n");
                prompt.push_str("Tool call:\n");
                prompt.push_str("{\"type\":\"tool_call\",\"tool\":\"repo.read_file|repo.search|repo.list_tree|repo.write_file|repo.replace_range|repo.apply_patch\",\"arguments\":{...},\"reason\":\"...\"}\n");
                prompt.push_str("Final answer:\n");
                prompt.push_str("{\"type\":\"final\",\"summary\":\"...\",\"result\":\"...\"}\n\n");
                prompt.push_str("If you cannot use tools, fallback to legacy file blocks:\n");
                prompt.push_str("File: relative/path/from/repo/root\n");
                prompt.push_str("```language\n");
                prompt.push_str("// full file content\n");
                prompt.push_str("```\n\n");
            }
            "unit_test_generation" => {
                prompt.push_str("Generate comprehensive unit tests for the implemented code. ");
                prompt.push_str(
                    "Include test cases for happy paths, edge cases, and error conditions.\n\n",
                );
            }
            "integration_test_generation" => {
                prompt.push_str(
                    "Generate integration tests that verify the interaction between components. ",
                );
                prompt.push_str("Focus on data flow and component integration.\n\n");
            }
            "ui_test_planning" => {
                prompt.push_str("Plan UI tests for the implemented features. ");
                prompt
                    .push_str("Describe the user interactions and expected behaviors to test.\n\n");
            }
            "qa_validation" => {
                prompt.push_str("Review the implementation, tests, and outputs. ");
                prompt.push_str(
                    "Validate that acceptance criteria are met and identify any issues.\n\n",
                );
            }
            "security_review" => {
                prompt.push_str("Review the code for security vulnerabilities. ");
                prompt.push_str("Check for common security issues, input validation, and secure coding practices.\n\n");
            }
            "performance_review" => {
                prompt.push_str("Review the implementation for performance considerations. ");
                prompt.push_str("Identify potential bottlenecks and suggest optimizations.\n\n");
            }
            _ => {
                prompt
                    .push_str("Execute your assigned work item based on the provided context.\n\n");
            }
        }

        // Add context information
        if stage_name == "coding" {
            prompt.push_str("Context:\n");
            for key in [
                "work_item_title",
                "work_item_type",
                "work_item_description",
                "problem_statement",
                "acceptance_criteria",
                "constraints",
                "requirement_analysis",
                "implementation_plan",
            ] {
                if let Some(value) = context.get(key) {
                    let limit = match key {
                        "implementation_plan" => 2_500,
                        "requirement_analysis" => 1_500,
                        "acceptance_criteria" => 1_000,
                        "constraints" => 800,
                        _ => 600,
                    };
                    prompt.push_str(&format!(
                        "{}: {}\n",
                        key,
                        value.chars().take(limit).collect::<String>()
                    ));
                }
            }
            if let Some(repo_context) = context.get("repository_context") {
                prompt.push_str("repository_context: ");
                prompt.push_str(&repo_context.chars().take(5_000).collect::<String>());
                prompt.push('\n');
            }
        } else {
            prompt.push_str("Context:\n");
            for (key, value) in context {
                prompt.push_str(&format!("{}: {}\n", key, value));
            }
        }

        if let Some(bounds) = &boundaries.instructions {
            prompt.push_str(&format!("\nAdditional Instructions: {}\n", bounds));
        }
        if let Some(allowed_paths) = &boundaries.allowed_paths {
            if !allowed_paths.is_empty() {
                prompt.push_str("\nAllowed paths:\n");
                for path in allowed_paths {
                    prompt.push_str("- ");
                    prompt.push_str(path);
                    prompt.push('\n');
                }
            }
        }
        if let Some(blocked_paths) = &boundaries.blocked_paths {
            if !blocked_paths.is_empty() {
                prompt.push_str("\nBlocked paths:\n");
                for path in blocked_paths {
                    prompt.push_str("- ");
                    prompt.push_str(path);
                    prompt.push('\n');
                }
            }
        }

        prompt.push_str("\nProvide your response in a clear, structured format.");

        info!(agent_id = %agent_def.id, stage_name = %stage_name, prompt_length = prompt.len(), "Successfully built agent prompt");
        Ok(prompt)
    }

    async fn resolve_execution_context(
        &self,
        work_item: &WorkItem,
        workflow_run: &WorkflowRun,
        stage_name: &str,
    ) -> Result<TeamExecutionContext, AppError> {
        let team = if let Some(team_id) = workflow_run.assigned_team_id.as_deref() {
            agent_repo::get_agent_team(&self.db, team_id).await.ok()
        } else {
            None
        }
        .or(agent_repo::resolve_team_for_work_item(&self.db, work_item).await?);

        let coordinator =
            if let Some(coordinator_id) = workflow_run.coordinator_agent_id.as_deref() {
                agent_repo::get_agent_definition(&self.db, coordinator_id)
                    .await
                    .ok()
            } else {
                None
            }
            .or(match &team {
                Some(team_entry) => {
                    agent_repo::find_team_coordinator(&self.db, &team_entry.id).await?
                }
                None => None,
            });

        if stage_name == "coordinator_review" {
            let stage_agent = coordinator.clone().ok_or_else(|| {
                AppError::NotFound("No active coordinator found for coordinator review".to_string())
            })?;
            if !Self::is_coordinator_role(&stage_agent.role) {
                return Err(AppError::Validation(format!(
                    "Invalid coordinator role for coordinator review: {}",
                    stage_agent.role
                )));
            }
            let expected_role = stage_agent.role.clone();
            return Ok(TeamExecutionContext {
                team,
                coordinator,
                stage_agent,
                expected_role,
            });
        }

        let (stage_agent, expected_role) =
            self.find_agent_for_stage(stage_name, team.as_ref()).await?;

        Ok(TeamExecutionContext {
            team,
            coordinator,
            stage_agent,
            expected_role,
        })
    }

    async fn stage_role_candidates(&self, stage_name: &str) -> Vec<String> {
        if let Ok(Some(policy)) = agent_repo::get_workflow_stage_policy(&self.db, stage_name).await
        {
            let mut roles = policy.primary_roles;
            roles.extend(policy.fallback_roles);
            if !roles.is_empty() {
                return roles;
            }
        }

        match stage_name {
            "coordinator_review" => vec!["manager", "team_lead"],
            "requirement_analysis" => {
                vec!["manager", "architect", "analyst", "requirement_analysis"]
            }
            "planning" => vec!["architect", "manager", "planning"],
            "coding" => vec!["developer", "coding"],
            "unit_test_generation" => vec!["unit_tester", "unit_test_generation"],
            "integration_test_generation" => {
                vec!["integration_tester", "integration_test_generation"]
            }
            "ui_test_planning" => vec!["ui_tester", "ui_test_planning"],
            "qa_validation" => vec!["code_reviewer", "qa", "qa_validation"],
            "security_review" => vec!["security_analyzer", "security_review"],
            "performance_review" => vec!["performance_optimizer", "performance_review"],
            "push_preparation" => vec!["manager", "devops", "planning"],
            "git_push" => vec!["devops", "sre"],
            other => vec![other],
        }
        .into_iter()
        .map(str::to_string)
        .collect()
    }

    fn select_agent_for_roles(
        &self,
        agents: Vec<AgentDefinition>,
        expected_roles: &[String],
    ) -> Option<AgentDefinition> {
        agents
            .into_iter()
            .filter(|agent| agent.enabled && agent.employment_status == "active")
            .min_by_key(|agent| {
                expected_roles
                    .iter()
                    .position(|expected_role| expected_role.eq_ignore_ascii_case(&agent.role))
                    .unwrap_or(usize::MAX)
            })
            .filter(|agent| {
                expected_roles
                    .iter()
                    .any(|expected_role| expected_role.eq_ignore_ascii_case(&agent.role))
            })
    }

    async fn store_manager_handoff(
        &self,
        work_item: &WorkItem,
        workflow_run: &WorkflowRun,
        stage_name: &str,
        execution_context: &TeamExecutionContext,
    ) -> Result<(), AppError> {
        let Some(coordinator) = &execution_context.coordinator else {
            return Ok(());
        };

        let handoff_dir = self
            .artifact_base_path
            .join(&workflow_run.id)
            .join("handoffs");
        tokio::fs::create_dir_all(&handoff_dir).await?;
        let handoff_path = handoff_dir.join(format!("{stage_name}.txt"));
        let team_name = execution_context
            .team
            .as_ref()
            .map(|team| team.name.clone())
            .unwrap_or_else(|| "unassigned".to_string());
        let handoff_body = format!(
            "Coordinator: {} ({})\nTeam: {}\nStage: {}\nAssigned Agent: {} ({})\nRule: Coordinator performs the handoff, the specialist executes, and artifacts should be left ready for the next gate.\n",
            coordinator.name,
            coordinator.role,
            team_name,
            stage_name,
            execution_context.stage_agent.name,
            execution_context.stage_agent.role
        );
        tokio::fs::write(&handoff_path, &handoff_body).await?;

        let artifact_id = uuid::Uuid::new_v4().to_string();
        artifact_repo::create_artifact(
            &self.db,
            &artifact_id,
            &work_item.id,
            Some(&workflow_run.id),
            None,
            &format!("manager_handoff_{stage_name}"),
            &format!(
                "Coordinator {} handed {} to {} for {}",
                coordinator.name, work_item.title, execution_context.stage_agent.name, stage_name
            ),
            &handoff_path.to_string_lossy(),
        )
        .await?;

        Ok(())
    }

    /// Execute the agent run against the model
    async fn execute_agent_run(
        &self,
        agent_run: &AgentRun,
        model_def: &ModelDefinition,
        prompt: &str,
        max_tokens: i64,
    ) -> Result<String, AppError> {
        #[cfg(test)]
        if let Some(queue_map) = TEST_MODEL_OUTPUTS.get() {
            let mut guard = queue_map
                .lock()
                .expect("failed to lock test model output queue");
            let mut next: Option<String> = None;
            let mut remove_keys: Vec<String> = Vec::new();

            for key in [agent_run.workflow_run_id.as_str(), TEST_ANY_WORKFLOW_KEY] {
                if next.is_some() {
                    break;
                }
                if let Some(queue) = guard.get_mut(key) {
                    next = queue.pop_front();
                    if queue.is_empty() {
                        remove_keys.push(key.to_string());
                    }
                }
            }

            for key in remove_keys {
                guard.remove(&key);
            }

            if let Some(next) = next {
                debug!(
                    agent_run_id = %agent_run.id,
                    model_id = %model_def.id,
                    model_name = %model_def.name,
                    "Using queued test model response"
                );
                return Ok(next);
            }
        }

        debug!(agent_run_id = %agent_run.id, model_id = %model_def.id, model_name = %model_def.name, prompt_length = prompt.len(), max_tokens = max_tokens, "Executing agent run");

        // Get the model provider
        let provider = model_repo::get_provider(&self.db, &model_def.provider_id).await?;
        debug!(agent_run_id = %agent_run.id, provider_id = %provider.id, provider_name = %provider.name, "Retrieved model provider");

        // Create the model gateway
        let gateway = self.model_service.create_gateway(&provider)?;
        debug!(agent_run_id = %agent_run.id, "Created model gateway");

        // Prepare the completion request
        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
        }];

        let request = CompletionRequest {
            model: model_def.name.clone(),
            messages,
            temperature: Some(0.7),
            max_tokens: Some(max_tokens),
        };

        // Execute the completion
        let response = gateway.run_completion(request).await?;
        info!(agent_run_id = %agent_run.id, response_length = response.content.len(), "Successfully executed agent run");

        Ok(response.content)
    }

    async fn execute_coding_with_tools(
        &self,
        agent_run: &AgentRun,
        model_def: &ModelDefinition,
        base_prompt: &str,
        stage_agent: &AgentDefinition,
        boundaries: &AgentExecutionBoundaries,
        response_token_budget: i64,
    ) -> Result<String, AppError> {
        let workflow_run =
            workflow_repo::get_workflow_run(&self.db, &agent_run.workflow_run_id).await?;
        let work_item = work_item_repo::get_work_item(&self.db, &workflow_run.work_item_id).await?;
        let Some(repo_id) = work_item.active_repo_id.as_deref() else {
            warn!(agent_run_id = %agent_run.id, work_item_id = %work_item.id, "No active repository found for coding tool loop; returning model output only");
            return self
                .execute_agent_run(agent_run, model_def, base_prompt, response_token_budget)
                .await;
        };
        let repo = crate::persistence::repository_repo::get_repository(&self.db, repo_id).await?;
        let workspace = WorkItemWorkspace::create(
            &work_item,
            &agent_run.workflow_run_id,
            &repo,
            &self.workspace_base_path,
        )
        .await?;

        let allowed_tools = self.resolve_allowed_tools(&stage_agent.allowed_tools);
        let max_steps = Self::max_tool_steps(boundaries);
        let mut tool_observations: Vec<String> = Vec::new();
        let mut trace: Vec<ToolLoopTraceEntry> = Vec::new();
        let mut changed_files: HashSet<String> = HashSet::new();
        let mut final_summary: Option<String> = None;
        let artifact_dir = self.artifact_base_path.join(&agent_run.id);
        tokio::fs::create_dir_all(&artifact_dir).await?;
        let trace_path = artifact_dir.join("tool_trace.json");
        Self::write_tool_trace_snapshot(&trace_path, &trace).await?;
        let trace_artifact_id = uuid::Uuid::new_v4().to_string();
        artifact_repo::create_artifact(
            &self.db,
            &trace_artifact_id,
            &work_item.id,
            Some(&agent_run.workflow_run_id),
            Some(&agent_run.id),
            "coding_tool_trace",
            "Tool loop trace for coding stage (live)",
            &trace_path.to_string_lossy(),
        )
        .await?;

        for step in 1..=max_steps {
            trace.push(ToolLoopTraceEntry {
                step,
                kind: "model_call_started".to_string(),
                payload: format!("step={} max_steps={}", step, max_steps),
            });
            Self::write_tool_trace_snapshot(&trace_path, &trace).await?;
            let step_prompt = self.build_coding_tool_prompt(
                base_prompt,
                &allowed_tools,
                &tool_observations,
                step,
                max_steps,
            );
            let model_output = self
                .execute_agent_run(agent_run, model_def, &step_prompt, response_token_budget)
                .await?;
            trace.push(ToolLoopTraceEntry {
                step,
                kind: "model_output".to_string(),
                payload: model_output.clone(),
            });
            Self::write_tool_trace_snapshot(&trace_path, &trace).await?;

            match Self::parse_tool_loop_response(&model_output) {
                Some(ToolLoopResponse::ToolCall {
                    tool,
                    arguments,
                    reason,
                }) => {
                    let execution = self
                        .execute_tool_call_in_workspace(
                            &workspace,
                            &tool,
                            &arguments,
                            &allowed_tools,
                            boundaries,
                            &mut changed_files,
                        )
                        .await;
                    match execution {
                        Ok(result) => {
                            let rendered = serde_json::to_string_pretty(&result)
                                .unwrap_or_else(|_| "{}".to_string());
                            tool_observations.push(format!(
                                "tool_result step={} tool={} reason={} result={}",
                                step,
                                tool,
                                reason.unwrap_or_default(),
                                rendered
                            ));
                            trace.push(ToolLoopTraceEntry {
                                step,
                                kind: "tool_result".to_string(),
                                payload: rendered,
                            });
                            Self::write_tool_trace_snapshot(&trace_path, &trace).await?;
                        }
                        Err(error) => {
                            let rendered = error.to_string();
                            tool_observations.push(format!(
                                "tool_error step={} tool={} reason={} error={}",
                                step,
                                tool,
                                reason.unwrap_or_default(),
                                rendered
                            ));
                            trace.push(ToolLoopTraceEntry {
                                step,
                                kind: "tool_error".to_string(),
                                payload: rendered,
                            });
                            Self::write_tool_trace_snapshot(&trace_path, &trace).await?;
                        }
                    }
                }
                Some(ToolLoopResponse::Final { summary, result }) => {
                    let resolved = summary
                        .or(result)
                        .unwrap_or_else(|| "Coding stage completed by tool loop.".to_string());
                    final_summary = Some(resolved);
                    trace.push(ToolLoopTraceEntry {
                        step,
                        kind: "final".to_string(),
                        payload: final_summary.clone().unwrap_or_default(),
                    });
                    Self::write_tool_trace_snapshot(&trace_path, &trace).await?;
                    break;
                }
                None => {
                    if step == 1 {
                        // Backward-compatible fallback for legacy "File: ..." output.
                        let legacy_changed = self
                            .parse_and_apply_changes(&workspace, &model_output, boundaries)
                            .await?;
                        for path in legacy_changed {
                            changed_files.insert(path);
                        }
                        final_summary =
                            Some("Applied legacy coding response format (File blocks)".to_string());
                        trace.push(ToolLoopTraceEntry {
                            step,
                            kind: "legacy_fallback".to_string(),
                            payload: "Used legacy File-block parser".to_string(),
                        });
                        Self::write_tool_trace_snapshot(&trace_path, &trace).await?;
                        break;
                    }
                    tool_observations.push(format!(
                        "tool_error step={} tool=parser error=Response did not match tool/final JSON contract",
                        step
                    ));
                }
            }
        }

        if final_summary.is_none() {
            if let Some(summary) = self
                .try_force_tool_loop_finalization(
                    agent_run,
                    model_def,
                    &tool_observations,
                    response_token_budget,
                )
                .await?
            {
                trace.push(ToolLoopTraceEntry {
                    step: max_steps + 1,
                    kind: "forced_final".to_string(),
                    payload: summary.clone(),
                });
                Self::write_tool_trace_snapshot(&trace_path, &trace).await?;
                final_summary = Some(summary);
            }
        }

        let changed_files_list = changed_files.into_iter().collect::<Vec<_>>();
        Self::write_tool_trace_snapshot(&trace_path, &trace).await?;

        if final_summary.is_none() {
            if !changed_files_list.is_empty() {
                warn!(
                    agent_run_id = %agent_run.id,
                    changed_files = changed_files_list.len(),
                    max_steps = max_steps,
                    "Coding tool loop reached max steps without final response; continuing with collected edits"
                );
                final_summary = Some(format!(
                    "Applied changes after reaching max steps ({max_steps}) without final response."
                ));
            } else {
                return Err(AppError::Validation(format!(
                    "Coding tool loop did not reach a final response within {} steps",
                    max_steps
                )));
            }
        }

        if !changed_files_list.is_empty() {
            workspace
                .sync_files_back(&repo.local_path, &changed_files_list)
                .await?;

            let applied_files_path = artifact_dir.join("applied_files.txt");
            tokio::fs::write(&applied_files_path, changed_files_list.join("\n")).await?;

            let artifact_id = uuid::Uuid::new_v4().to_string();
            artifact_repo::create_artifact(
                &self.db,
                &artifact_id,
                &work_item.id,
                Some(&agent_run.workflow_run_id),
                Some(&agent_run.id),
                "coding_applied_files",
                &format!("Applied {} files to repository", changed_files_list.len()),
                &applied_files_path.to_string_lossy(),
            )
            .await?;
        } else {
            warn!(agent_run_id = %agent_run.id, "Coding tool loop completed with no changed files");
        }

        if Self::should_keep_workspace(boundaries) {
            info!(agent_run_id = %agent_run.id, workspace_path = %workspace.base_path.display(), "Retaining workspace for inspection");
        } else {
            workspace.cleanup().await?;
        }

        Ok(format!(
            "{}\nChanged files: {}",
            final_summary.unwrap_or_default(),
            if changed_files_list.is_empty() {
                "none".to_string()
            } else {
                changed_files_list.join(", ")
            }
        ))
    }

    fn resolve_allowed_tools(&self, configured_tools: &[String]) -> HashSet<String> {
        let mut allowed = HashSet::new();
        if configured_tools.is_empty() {
            allowed.insert("repo.list_tree".to_string());
            allowed.insert("repo.read_file".to_string());
            allowed.insert("repo.search".to_string());
            allowed.insert("repo.write_file".to_string());
            allowed.insert("repo.replace_range".to_string());
            allowed.insert("repo.apply_patch".to_string());
            return allowed;
        }
        for tool in configured_tools {
            let lowered = tool.trim().to_ascii_lowercase();
            if lowered.contains("read") {
                allowed.insert("repo.read_file".to_string());
                allowed.insert("repo.list_tree".to_string());
                allowed.insert("repo.search".to_string());
            }
            if lowered.contains("write") || lowered.contains("create") {
                allowed.insert("repo.write_file".to_string());
                allowed.insert("repo.replace_range".to_string());
            }
            if lowered.contains("modify") || lowered.contains("patch") {
                allowed.insert("repo.apply_patch".to_string());
                allowed.insert("repo.replace_range".to_string());
            }
            if lowered.starts_with("repo.") {
                allowed.insert(lowered);
            }
        }
        if allowed.is_empty() {
            allowed.insert("repo.read_file".to_string());
            allowed.insert("repo.search".to_string());
            allowed.insert("repo.write_file".to_string());
            allowed.insert("repo.replace_range".to_string());
        }
        allowed
    }

    fn build_coding_tool_prompt(
        &self,
        base_prompt: &str,
        allowed_tools: &HashSet<String>,
        observations: &[String],
        step: usize,
        max_steps: usize,
    ) -> String {
        let mut prompt = String::new();
        if step == 1 {
            prompt.push_str(base_prompt);
        } else {
            prompt.push_str("Coding task recap:\n");
            prompt.push_str(&Self::condense_tool_loop_base_prompt(base_prompt));
        }
        prompt.push_str("\n\nYou are in a tool execution loop for coding.");
        prompt.push_str("\nReturn exactly one JSON object.");
        prompt.push_str("\nAllowed tools: ");
        prompt.push_str(&allowed_tools.iter().cloned().collect::<Vec<_>>().join(", "));
        prompt.push_str("\nTool argument contract:");
        prompt.push_str(
            "\n- repo.read_file -> {\"path\":\"relative/path.ext\",\"offset_chars\":0,\"length_chars\":12000,\"start_line\":1,\"end_line\":200}",
        );
        prompt.push_str(
            "\n- repo.write_file -> {\"path\":\"relative/path.ext\",\"content\":\"full file content\"}",
        );
        prompt.push_str(
            "\n- repo.replace_range -> {\"path\":\"relative/path.ext\",\"start_line\":10,\"end_line\":22,\"content\":\"replacement text\"}",
        );
        prompt.push_str(
            "\n- repo.apply_patch -> {\"path\":\"relative/path.ext\",\"patch\":\"unified diff hunk text\"}",
        );
        prompt.push_str("\n- repo.search -> {\"query\":\"text\",\"max_results\":20}");
        prompt.push_str("\n- repo.list_tree -> {\"path\":\"optional/subdir\",\"max_depth\":3}");
        prompt.push_str(
            "\nIMPORTANT: prefer repo.read_file with line/chunk arguments for large files.",
        );
        prompt.push_str(
            "\nPrefer repo.replace_range for surgical edits and repo.write_file only for intentional full-file rewrites.",
        );
        prompt.push_str(
            "\nUse repo.apply_patch only when you are certain context lines exactly match current file content.",
        );
        prompt.push_str("\nIf more information or edits are needed, return:");
        prompt.push_str(
            "\n{\"type\":\"tool_call\",\"tool\":\"...\",\"arguments\":{...},\"reason\":\"...\"}",
        );
        prompt.push_str("\nWhen implementation is complete, return:");
        prompt.push_str("\n{\"type\":\"final\",\"summary\":\"...\",\"result\":\"...\"}");
        prompt.push_str(&format!("\nCurrent step: {}/{}", step, max_steps));
        if max_steps.saturating_sub(step) <= 2 {
            prompt.push_str(
                "\nYou are close to the tool-step limit. Prefer finishing and returning type=final now unless a single essential tool call is still required.",
            );
        }
        if !observations.is_empty() {
            prompt.push_str("\n\nTool observations so far:\n");
            for item in observations.iter().rev().take(12).rev() {
                prompt.push_str("- ");
                prompt.push_str(item);
                prompt.push('\n');
            }
        }
        prompt
    }

    async fn try_force_tool_loop_finalization(
        &self,
        agent_run: &AgentRun,
        model_def: &ModelDefinition,
        observations: &[String],
        response_token_budget: i64,
    ) -> Result<Option<String>, AppError> {
        if observations.is_empty() {
            return Ok(None);
        }
        let mut prompt =
            String::from("Return exactly one JSON object with type=final. Do not call tools.\n");
        prompt.push_str("Output format:\n");
        prompt.push_str("{\"type\":\"final\",\"summary\":\"...\",\"result\":\"...\"}\n");
        prompt.push_str("Recent tool observations:\n");
        for item in observations.iter().rev().take(16).rev() {
            prompt.push_str("- ");
            prompt.push_str(item);
            prompt.push('\n');
        }
        let model_output = self
            .execute_agent_run(
                agent_run,
                model_def,
                &prompt,
                response_token_budget.min(1024),
            )
            .await?;
        match Self::parse_tool_loop_response(&model_output) {
            Some(ToolLoopResponse::Final { summary, result }) => {
                Ok(Some(summary.or(result).unwrap_or_else(|| {
                    "Coding stage finalized after tool-loop completion.".to_string()
                })))
            }
            _ => Ok(None),
        }
    }

    fn condense_tool_loop_base_prompt(base_prompt: &str) -> String {
        let mut condensed = String::new();
        for line in base_prompt.lines() {
            if line.starts_with("repository_context:") {
                condensed.push_str("repository_context: [initial repository summary omitted on follow-up turns; use tools for current file state]\n");
                continue;
            }
            condensed.push_str(line);
            condensed.push('\n');
            if condensed.len() >= 3_500 {
                condensed.push_str("...[truncated task recap]...\n");
                break;
            }
        }
        condensed
    }

    fn parse_tool_loop_response(output: &str) -> Option<ToolLoopResponse> {
        let payload = Self::extract_json_payload(output)?;
        serde_json::from_str::<ToolLoopResponse>(&payload).ok()
    }

    async fn write_tool_trace_snapshot(
        trace_path: &Path,
        trace: &[ToolLoopTraceEntry],
    ) -> Result<(), AppError> {
        let payload = serde_json::to_string_pretty(trace)?;
        tokio::fs::write(trace_path, payload)
            .await
            .map_err(|error| {
                AppError::Io(std::io::Error::other(format!(
                    "Failed to write tool trace snapshot: {}",
                    error
                )))
            })?;
        Ok(())
    }

    fn extract_json_payload(output: &str) -> Option<String> {
        let trimmed = output.trim();
        if trimmed.starts_with('{') && trimmed.ends_with('}') {
            return Some(trimmed.to_string());
        }
        if let Some(start) = trimmed.find("```json") {
            let rest = &trimmed[start + 7..];
            if let Some(end) = rest.find("```") {
                return Some(rest[..end].trim().to_string());
            }
        }
        let start = trimmed.find('{')?;
        let end = trimmed.rfind('}')?;
        if end <= start {
            return None;
        }
        Some(trimmed[start..=end].to_string())
    }

    async fn execute_tool_call_in_workspace(
        &self,
        workspace: &WorkItemWorkspace,
        tool: &str,
        arguments: &Value,
        allowed_tools: &HashSet<String>,
        boundaries: &AgentExecutionBoundaries,
        changed_files: &mut HashSet<String>,
    ) -> Result<Value, AppError> {
        let canonical_tool = tool.trim().to_ascii_lowercase();
        if !allowed_tools.contains(&canonical_tool) {
            return Err(AppError::Validation(format!(
                "Tool '{}' is not allowed for this agent",
                tool
            )));
        }

        match canonical_tool.as_str() {
            "repo.read_file" => {
                let path = arguments
                    .get("path")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        AppError::Validation("repo.read_file requires 'path'".to_string())
                    })?;
                let normalized = Self::normalize_relative_path(path).ok_or_else(|| {
                    AppError::Validation("Invalid path for repo.read_file".to_string())
                })?;
                if !Self::is_repo_relative_path_allowed(&normalized, boundaries) {
                    return Err(AppError::Validation(format!(
                        "Path is outside boundaries: {}",
                        normalized
                    )));
                }
                let content = workspace.read_file(&normalized).await?;
                let total_chars = Self::char_count(&content);
                let max_chars_per_read = Self::max_read_file_chars(boundaries);
                let requested_offset = arguments
                    .get("offset_chars")
                    .and_then(Value::as_u64)
                    .unwrap_or(0) as usize;
                let requested_length = arguments
                    .get("length_chars")
                    .and_then(Value::as_u64)
                    .map(|value| value as usize)
                    .unwrap_or(max_chars_per_read);
                let start_line = arguments.get("start_line").and_then(Value::as_u64);
                let end_line = arguments.get("end_line").and_then(Value::as_u64);

                if (start_line.is_some() || end_line.is_some())
                    && (arguments.get("offset_chars").is_some()
                        || arguments.get("length_chars").is_some())
                {
                    return Err(AppError::Validation(
                        "repo.read_file cannot mix line-range arguments with offset/length"
                            .to_string(),
                    ));
                }

                let (selected_content, selected_start_line, selected_end_line, total_lines) =
                    if start_line.is_some() || end_line.is_some() {
                        let lines = content.lines().collect::<Vec<_>>();
                        let total_lines = lines.len();
                        if total_lines == 0 {
                            (String::new(), Some(1usize), Some(0usize), 0usize)
                        } else {
                            let start = start_line.unwrap_or(1) as usize;
                            let mut end = end_line.unwrap_or((start + 199) as u64) as usize;
                            if start == 0 {
                                return Err(AppError::Validation(
                                    "repo.read_file start_line must be >= 1".to_string(),
                                ));
                            }
                            if end < start {
                                return Err(AppError::Validation(
                                    "repo.read_file end_line must be >= start_line".to_string(),
                                ));
                            }
                            if start > total_lines {
                                return Err(AppError::Validation(format!(
                                    "repo.read_file start_line {} is beyond total lines {}",
                                    start, total_lines
                                )));
                            }
                            end = end.min(total_lines);
                            let range_content = lines[start - 1..end].join("\n");
                            (range_content, Some(start), Some(end), total_lines)
                        }
                    } else {
                        (content.clone(), None, None, content.lines().count())
                    };

                let selected_total_chars = Self::char_count(&selected_content);
                if requested_offset > selected_total_chars {
                    return Err(AppError::Validation(format!(
                        "repo.read_file offset_chars {} is beyond content length {}",
                        requested_offset, selected_total_chars
                    )));
                }
                let effective_length = requested_length.clamp(1, max_chars_per_read);
                let clipped = Self::substring_by_char_range(
                    &selected_content,
                    requested_offset,
                    effective_length,
                );
                let returned_chars = Self::char_count(&clipped);
                let next_offset = requested_offset + returned_chars;
                let truncated = next_offset < selected_total_chars;
                Ok(serde_json::json!({
                    "path": normalized,
                    "content": clipped,
                    "truncated": truncated,
                    "total_chars": selected_total_chars,
                    "returned_chars": returned_chars,
                    "offset_chars": requested_offset,
                    "next_offset_chars": if truncated { Some(next_offset) } else { None::<usize> },
                    "max_chars_per_read": max_chars_per_read,
                    "selection_total_chars": selected_total_chars,
                    "file_total_chars": total_chars,
                    "selection_start_line": selected_start_line,
                    "selection_end_line": selected_end_line,
                    "file_total_lines": total_lines,
                }))
            }
            "repo.list_tree" => {
                let base_path = arguments
                    .get("path")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .trim();
                let max_depth = arguments
                    .get("max_depth")
                    .and_then(Value::as_u64)
                    .unwrap_or(3)
                    .clamp(1, 6) as usize;
                self.list_tree_from_workspace(workspace, base_path, max_depth, boundaries)
            }
            "repo.search" => {
                let query = arguments
                    .get("query")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        AppError::Validation("repo.search requires 'query'".to_string())
                    })?;
                let max_results = arguments
                    .get("max_results")
                    .and_then(Value::as_u64)
                    .unwrap_or(20)
                    .clamp(1, 80) as usize;
                self.search_workspace(workspace, query, max_results, boundaries)
            }
            "repo.write_file" => {
                let path = arguments
                    .get("path")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        AppError::Validation("repo.write_file requires 'path'".to_string())
                    })?;
                let content = arguments
                    .get("content")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        AppError::Validation("repo.write_file requires 'content'".to_string())
                    })?;
                let normalized = Self::normalize_relative_path(path).ok_or_else(|| {
                    AppError::Validation("Invalid path for repo.write_file".to_string())
                })?;
                if !Self::is_repo_relative_path_allowed(&normalized, boundaries) {
                    return Err(AppError::Validation(format!(
                        "Path is outside boundaries: {}",
                        normalized
                    )));
                }
                Self::ensure_write_limit(content, boundaries)?;
                workspace.write_file(&normalized, content).await?;
                changed_files.insert(normalized.clone());
                Ok(serde_json::json!({
                    "path": normalized,
                    "bytes_written": content.len(),
                    "chars_written": Self::char_count(content),
                }))
            }
            "repo.replace_range" => {
                let path = arguments
                    .get("path")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        AppError::Validation("repo.replace_range requires 'path'".to_string())
                    })?;
                let start_line = arguments
                    .get("start_line")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| {
                        AppError::Validation("repo.replace_range requires 'start_line'".to_string())
                    })? as usize;
                let end_line = arguments
                    .get("end_line")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| {
                        AppError::Validation("repo.replace_range requires 'end_line'".to_string())
                    })? as usize;
                let replacement = arguments
                    .get("content")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        AppError::Validation("repo.replace_range requires 'content'".to_string())
                    })?;
                if start_line == 0 {
                    return Err(AppError::Validation(
                        "repo.replace_range start_line must be >= 1".to_string(),
                    ));
                }
                if end_line < start_line {
                    return Err(AppError::Validation(
                        "repo.replace_range end_line must be >= start_line".to_string(),
                    ));
                }
                let normalized = Self::normalize_relative_path(path).ok_or_else(|| {
                    AppError::Validation("Invalid path for repo.replace_range".to_string())
                })?;
                if !Self::is_repo_relative_path_allowed(&normalized, boundaries) {
                    return Err(AppError::Validation(format!(
                        "Path is outside boundaries: {}",
                        normalized
                    )));
                }
                let existing = workspace.read_file(&normalized).await?;
                let (lines, had_trailing_newline) = Self::split_lines_preserve_trailing(&existing);
                if lines.is_empty() {
                    return Err(AppError::Validation(format!(
                        "repo.replace_range requires a non-empty file; use repo.write_file for {}",
                        normalized
                    )));
                }
                if end_line > lines.len() {
                    return Err(AppError::Validation(format!(
                        "repo.replace_range end_line {} is beyond total lines {}",
                        end_line,
                        lines.len()
                    )));
                }
                let replacement_lines = replacement
                    .split('\n')
                    .map(std::string::ToString::to_string)
                    .collect::<Vec<_>>();
                let mut merged = Vec::with_capacity(
                    lines.len() - (end_line - start_line + 1) + replacement_lines.len(),
                );
                merged.extend_from_slice(&lines[..start_line - 1]);
                merged.extend(replacement_lines);
                merged.extend_from_slice(&lines[end_line..]);
                let mut updated = merged.join("\n");
                if had_trailing_newline {
                    updated.push('\n');
                }
                Self::ensure_write_limit(&updated, boundaries)?;
                workspace.write_file(&normalized, &updated).await?;
                changed_files.insert(normalized.clone());
                Ok(serde_json::json!({
                    "path": normalized,
                    "start_line": start_line,
                    "end_line": end_line,
                    "replaced_lines": end_line.saturating_sub(start_line) + 1,
                }))
            }
            "repo.apply_patch" => {
                let path = arguments
                    .get("path")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        AppError::Validation("repo.apply_patch requires 'path'".to_string())
                    })?;
                let patch = arguments
                    .get("patch")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        AppError::Validation("repo.apply_patch requires 'patch'".to_string())
                    })?;
                let base_sha256 = arguments.get("base_sha256").and_then(Value::as_str);
                let normalized = Self::normalize_relative_path(path).ok_or_else(|| {
                    AppError::Validation("Invalid path for repo.apply_patch".to_string())
                })?;
                if !Self::is_repo_relative_path_allowed(&normalized, boundaries) {
                    return Err(AppError::Validation(format!(
                        "Path is outside boundaries: {}",
                        normalized
                    )));
                }
                let new_hash = repo_service::apply_repository_patch(
                    &workspace.repo_path.to_string_lossy(),
                    &normalized,
                    patch,
                    base_sha256,
                )?;
                changed_files.insert(normalized.clone());
                Ok(serde_json::json!({
                    "path": normalized,
                    "sha256": new_hash,
                }))
            }
            _ => Err(AppError::Validation(format!(
                "Unsupported tool call: {}",
                tool
            ))),
        }
    }

    fn list_tree_from_workspace(
        &self,
        workspace: &WorkItemWorkspace,
        base_path: &str,
        max_depth: usize,
        boundaries: &AgentExecutionBoundaries,
    ) -> Result<Value, AppError> {
        let normalized = if base_path.is_empty() {
            None
        } else {
            Some(Self::normalize_relative_path(base_path).ok_or_else(|| {
                AppError::Validation("Invalid base path for repo.list_tree".to_string())
            })?)
        };
        if let Some(path) = normalized.as_deref() {
            if !Self::is_repo_relative_path_allowed(path, boundaries) {
                return Err(AppError::Validation(format!(
                    "Path is outside boundaries: {}",
                    path
                )));
            }
        }

        let root = if let Some(path) = normalized.as_deref() {
            workspace.repo_path.join(path)
        } else {
            workspace.repo_path.clone()
        };
        if !root.exists() || !root.is_dir() {
            return Err(AppError::Validation(
                "repo.list_tree target is not a directory".to_string(),
            ));
        }

        let mut entries: Vec<Value> = Vec::new();
        for entry in WalkDir::new(&root)
            .max_depth(max_depth + 1)
            .into_iter()
            .filter_map(Result::ok)
        {
            if entries.len() >= 350 {
                break;
            }
            if entry.path() == root {
                continue;
            }
            let rel = match entry.path().strip_prefix(&workspace.repo_path) {
                Ok(value) => value.to_string_lossy().replace('\\', "/"),
                Err(_) => continue,
            };
            if !Self::is_repo_relative_path_allowed(&rel, boundaries) {
                continue;
            }
            entries.push(serde_json::json!({
                "path": rel,
                "kind": if entry.file_type().is_dir() { "directory" } else { "file" },
            }));
        }
        Ok(serde_json::json!({ "entries": entries }))
    }

    fn search_workspace(
        &self,
        workspace: &WorkItemWorkspace,
        query: &str,
        max_results: usize,
        boundaries: &AgentExecutionBoundaries,
    ) -> Result<Value, AppError> {
        let needle = query.trim().to_ascii_lowercase();
        if needle.is_empty() {
            return Err(AppError::Validation(
                "repo.search query cannot be empty".to_string(),
            ));
        }
        let mut results: Vec<Value> = Vec::new();
        for entry in WalkDir::new(&workspace.repo_path)
            .max_depth(8)
            .into_iter()
            .filter_map(Result::ok)
        {
            if results.len() >= max_results {
                break;
            }
            if !entry.file_type().is_file() {
                continue;
            }
            let rel = match entry.path().strip_prefix(&workspace.repo_path) {
                Ok(value) => value.to_string_lossy().replace('\\', "/"),
                Err(_) => continue,
            };
            if !Self::is_repo_relative_path_allowed(&rel, boundaries)
                || !Self::is_text_source_file(&rel)
            {
                continue;
            }
            let content = match std::fs::read_to_string(entry.path()) {
                Ok(text) => text,
                Err(_) => continue,
            };
            for (index, line) in content.lines().enumerate() {
                if line.to_ascii_lowercase().contains(&needle) {
                    results.push(serde_json::json!({
                        "path": rel,
                        "line": index + 1,
                        "text": line.trim(),
                    }));
                    if results.len() >= max_results {
                        break;
                    }
                }
            }
        }
        Ok(serde_json::json!({ "query": query, "results": results }))
    }

    /// Store agent output as artifacts
    async fn store_agent_output(
        &self,
        agent_run: &AgentRun,
        prompt: &str,
        output: &str,
    ) -> Result<(), AppError> {
        debug!(agent_run_id = %agent_run.id, prompt_length = prompt.len(), output_length = output.len(), "Storing agent output as artifacts");

        // Get the workflow run to obtain work_item_id
        let workflow_run =
            workflow_repo::get_workflow_run(&self.db, &agent_run.workflow_run_id).await?;
        let work_item_id = workflow_run.work_item_id;
        debug!(agent_run_id = %agent_run.id, work_item_id = %work_item_id, "Retrieved workflow run for work item");

        let artifact_dir = self.artifact_base_path.join(&agent_run.id);
        tokio::fs::create_dir_all(&artifact_dir)
            .await
            .map_err(|e| {
                AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to create artifact directory: {}", e),
                ))
            })?;

        // Write prompt to file first so storage path points to an existing file.
        let prompt_path = artifact_dir.join("prompt.txt");
        tokio::fs::write(&prompt_path, prompt).await.map_err(|e| {
            AppError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to write prompt file: {}", e),
            ))
        })?;
        debug!(agent_run_id = %agent_run.id, prompt_path = %prompt_path.display(), "Wrote prompt to file");

        // Store prompt snapshot
        let prompt_artifact_id = uuid::Uuid::new_v4().to_string();
        artifact_repo::create_artifact(
            &self.db,
            &prompt_artifact_id,
            &work_item_id,
            Some(&agent_run.workflow_run_id),
            Some(&agent_run.id),
            &format!("{}_prompt", agent_run.stage),
            "Prompt used for agent execution",
            &prompt_path.to_string_lossy(),
        )
        .await?;
        debug!(agent_run_id = %agent_run.id, artifact_id = %prompt_artifact_id, "Created prompt artifact");

        // Write output file before recording artifact metadata.
        let output_path = artifact_dir.join("output.txt");
        tokio::fs::write(&output_path, output).await.map_err(|e| {
            AppError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to write output file: {}", e),
            ))
        })?;
        debug!(agent_run_id = %agent_run.id, output_path = %output_path.display(), "Wrote output to file");

        // Store output artifact
        let output_artifact_id = uuid::Uuid::new_v4().to_string();
        artifact_repo::create_artifact(
            &self.db,
            &output_artifact_id,
            &work_item_id,
            Some(&agent_run.workflow_run_id),
            Some(&agent_run.id),
            &format!("{}_output", agent_run.stage),
            &output.chars().take(200).collect::<String>(),
            &output_path.to_string_lossy(),
        )
        .await?;
        debug!(agent_run_id = %agent_run.id, artifact_id = %output_artifact_id, "Created output artifact");

        info!(agent_run_id = %agent_run.id, "Successfully stored agent output as artifacts");
        Ok(())
    }

    /// Process agent output based on the agent type
    async fn process_agent_output(
        &self,
        agent_run: &AgentRun,
        output: &str,
        stage_name: &str,
    ) -> Result<String, AppError> {
        debug!(agent_run_id = %agent_run.id, stage_name = %stage_name, output_length = output.len(), "Processing agent output");

        match stage_name {
            "coding" => {
                // For coding agent, we need to apply the code changes
                self.apply_code_changes(agent_run, output).await?;
                info!(agent_run_id = %agent_run.id, stage_name = %stage_name, "Applied code changes from coding agent");
                Ok(output.to_string())
            }
            _ => {
                // For other agents, just return the output as-is
                debug!(agent_run_id = %agent_run.id, stage_name = %stage_name, "Returning output as-is for non-coding agent");
                Ok(output.to_string())
            }
        }
    }

    /// Apply code changes from the coding agent output
    async fn apply_code_changes(&self, agent_run: &AgentRun, output: &str) -> Result<(), AppError> {
        debug!(agent_run_id = %agent_run.id, output_length = output.len(), "Applying code changes from agent output");

        // Get the workflow run to obtain work_item_id
        let workflow_run =
            workflow_repo::get_workflow_run(&self.db, &agent_run.workflow_run_id).await?;
        let work_item_id = workflow_run.work_item_id;
        debug!(agent_run_id = %agent_run.id, work_item_id = %work_item_id, "Retrieved workflow run for work item");

        // Get the work item and repository info
        let work_item = work_item_repo::get_work_item(&self.db, &work_item_id).await?;
        let agent_def = agent_repo::get_agent_definition(&self.db, &agent_run.agent_id).await?;
        let boundaries = Self::parse_boundaries(&agent_def.boundaries);
        let repo = if let Some(repo_id) = &work_item.active_repo_id {
            Some(crate::persistence::repository_repo::get_repository(&self.db, repo_id).await?)
        } else {
            None
        };

        if let Some(repo) = repo {
            debug!(agent_run_id = %agent_run.id, repo_id = %repo.id, repo_path = %repo.local_path, "Found active repository for work item");

            // Create isolated workspace
            let workspace = WorkItemWorkspace::create(
                &work_item,
                &agent_run.workflow_run_id,
                &repo,
                &self.workspace_base_path,
            )
            .await?;
            info!(agent_run_id = %agent_run.id, workspace_path = %workspace.base_path.display(), "Created isolated workspace");

            // Parse and apply code changes
            let changed_files = self
                .parse_and_apply_changes(&workspace, output, &boundaries)
                .await?;
            info!(agent_run_id = %agent_run.id, changed_files = changed_files.len(), "Successfully parsed and applied code changes");

            if changed_files.is_empty() {
                warn!(agent_run_id = %agent_run.id, "Coding stage produced no file edits");
            } else {
                workspace
                    .sync_files_back(&repo.local_path, &changed_files)
                    .await?;
                info!(agent_run_id = %agent_run.id, changed_files = changed_files.len(), repo_path = %repo.local_path, "Synced changed files back to repository");

                let artifact_dir = self.artifact_base_path.join(&agent_run.id);
                tokio::fs::create_dir_all(&artifact_dir).await?;
                let applied_files_path = artifact_dir.join("applied_files.txt");
                let applied_summary = changed_files.join("\n");
                tokio::fs::write(&applied_files_path, &applied_summary).await?;

                let artifact_id = uuid::Uuid::new_v4().to_string();
                artifact_repo::create_artifact(
                    &self.db,
                    &artifact_id,
                    &work_item_id,
                    Some(&agent_run.workflow_run_id),
                    Some(&agent_run.id),
                    "coding_applied_files",
                    &format!("Applied {} files to repository", changed_files.len()),
                    &applied_files_path.to_string_lossy(),
                )
                .await?;
            }

            // TODO: Generate diff and store as artifact

            if Self::should_keep_workspace(&boundaries) {
                info!(agent_run_id = %agent_run.id, workspace_path = %workspace.base_path.display(), "Retaining workspace for inspection");
            } else {
                workspace.cleanup().await?;
                debug!(agent_run_id = %agent_run.id, "Cleaned up workspace after processing");
            }
        } else {
            warn!(agent_run_id = %agent_run.id, work_item_id = %work_item_id, "No active repository found for work item, skipping code changes");
        }

        info!(agent_run_id = %agent_run.id, "Successfully applied code changes");
        Ok(())
    }

    /// Parse coding agent output and apply file changes
    async fn parse_and_apply_changes(
        &self,
        workspace: &WorkItemWorkspace,
        output: &str,
        boundaries: &AgentExecutionBoundaries,
    ) -> Result<Vec<String>, AppError> {
        debug!(workspace_path = %workspace.base_path.display(), output_length = output.len(), "Parsing and applying code changes");

        // Simple parsing - look for file markers like "File: path/to/file"
        // This is a basic implementation - in production, you'd want more robust parsing

        let lines: Vec<&str> = output.lines().collect();
        let mut current_file: Option<String> = None;
        let mut file_content = String::new();
        let mut files_processed = 0usize;
        let mut changed_files: Vec<String> = Vec::new();
        let max_files = Self::max_files_per_run(boundaries);

        for line in lines {
            if line.starts_with("File: ") || line.starts_with("### File: ") {
                // Save previous file if any
                if let Some(file_path) = current_file.take() {
                    if !file_content.trim().is_empty() {
                        if files_processed >= max_files {
                            warn!(workspace_path = %workspace.base_path.display(), max_files = max_files, "Reached max files per run boundary; skipping additional file updates");
                            break;
                        }
                        let Some(normalized_path) = Self::normalize_relative_path(&file_path)
                        else {
                            warn!(workspace_path = %workspace.base_path.display(), file_path = %file_path, "Skipping invalid relative file path from agent output");
                            file_content.clear();
                            continue;
                        };
                        if !Self::is_repo_relative_path_allowed(&normalized_path, boundaries) {
                            warn!(workspace_path = %workspace.base_path.display(), file_path = %normalized_path, "Skipping file outside boundaries");
                            file_content.clear();
                            continue;
                        }
                        Self::ensure_write_limit(&file_content, boundaries)?;
                        workspace
                            .write_file(&normalized_path, &file_content)
                            .await?;
                        files_processed += 1;
                        changed_files.push(normalized_path.clone());
                        debug!(workspace_path = %workspace.base_path.display(), file_path = %normalized_path, content_length = file_content.len(), "Wrote file content");
                    }
                }

                // Start new file
                let raw = line
                    .trim_start_matches("File: ")
                    .trim_start_matches("### File: ")
                    .trim();
                current_file = Some(raw.to_string());
                file_content.clear();
                debug!(workspace_path = %workspace.base_path.display(), file_path = current_file.as_ref().unwrap(), "Started processing new file");
            } else if let Some(ref _file_path) = current_file {
                if line.trim_start().starts_with("```") {
                    continue;
                }
                file_content.push_str(line);
                file_content.push('\n');
            }
        }

        // Save the last file
        if let Some(file_path) = current_file {
            if !file_content.trim().is_empty() {
                if files_processed < max_files {
                    if let Some(normalized_path) = Self::normalize_relative_path(&file_path) {
                        if Self::is_repo_relative_path_allowed(&normalized_path, boundaries) {
                            Self::ensure_write_limit(&file_content, boundaries)?;
                            workspace
                                .write_file(&normalized_path, &file_content)
                                .await?;
                            files_processed += 1;
                            changed_files.push(normalized_path.clone());
                            debug!(workspace_path = %workspace.base_path.display(), file_path = %normalized_path, content_length = file_content.len(), "Wrote final file content");
                        } else {
                            warn!(workspace_path = %workspace.base_path.display(), file_path = %normalized_path, "Skipping final file outside boundaries");
                        }
                    }
                } else {
                    warn!(workspace_path = %workspace.base_path.display(), max_files = max_files, "Skipped final file due to max files boundary");
                }
            }
        }

        info!(workspace_path = %workspace.base_path.display(), files_processed = files_processed, "Successfully parsed and applied code changes");
        Ok(changed_files)
    }
}

#[cfg(test)]
mod tests {
    use super::AgentExecutionBoundaries;
    use super::AgentService;
    use crate::persistence::{
        agent_repo, artifact_repo, db as db_service, model_repo, product_repo, repository_repo,
        work_item_repo, workflow_repo,
    };
    use crate::services::model_service::ModelService;
    use serde_json::json;
    use std::path::PathBuf;
    use std::sync::Arc;

    #[test]
    fn substring_by_char_range_respects_offset_and_length() {
        let value = "alpha-beta-gamma";
        let sliced = AgentService::substring_by_char_range(value, 6, 4);
        assert_eq!(sliced, "beta");
    }

    #[test]
    fn ensure_write_limit_rejects_content_above_boundary() {
        let boundaries = AgentExecutionBoundaries {
            max_write_file_chars: Some(400),
            ..Default::default()
        };
        let oversized = "x".repeat(401);
        let result = AgentService::ensure_write_limit(&oversized, &boundaries);
        assert!(result.is_err(), "expected write limit validation to fail");
    }

    fn make_temp_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "aruvi_agent_service_{name}_{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&path).expect("failed to create temp directory");
        path
    }

    #[tokio::test]
    async fn coding_stage_creates_tool_trace_artifact() {
        let temp_root = make_temp_dir("coding_trace");
        let db_path = temp_root.join("aruvi-test.db");
        let db_url = format!("sqlite:{}", db_path.display());
        let pool = db_service::create_pool(&db_url)
            .await
            .expect("failed to create database pool");

        let provider_id = "test-provider";
        model_repo::create_provider(
            &pool,
            provider_id,
            "Test Provider",
            "openai_compatible",
            "http://example.invalid",
            None,
        )
        .await
        .expect("failed to create model provider");

        let model_id = "test-model";
        model_repo::create_model_definition(&pool, model_id, provider_id, "test-model", Some(8192))
            .await
            .expect("failed to create model definition");

        agent_repo::delete_agent_model_bindings_for_agent(&pool, "coding-agent")
            .await
            .expect("failed to clear coding agent model bindings");
        agent_repo::create_agent_model_binding(
            &pool,
            "test-coding-binding",
            "coding-agent",
            model_id,
            0,
        )
        .await
        .expect("failed to bind coding agent to test model");
        sqlx::query(
            "UPDATE agent_definitions
             SET boundaries='{\"max_tool_steps\":4,\"keep_workspace\":false,\"max_file_chars\":4000}',
                 enabled=1,
                 employment_status='active'
             WHERE id='coding-agent'",
        )
        .execute(&pool)
        .await
        .expect("failed to update coding agent boundaries");

        let product = product_repo::create_product(
            &pool,
            "test-product",
            "Integration Product",
            "desc",
            "vision",
            "[]",
            "[]",
        )
        .await
        .expect("failed to create product");
        let module = product_repo::create_module(
            &pool,
            "test-module",
            &product.id,
            "Core Module",
            "desc",
            "purpose",
        )
        .await
        .expect("failed to create module");

        let repo_dir = temp_root.join("repo");
        std::fs::create_dir_all(&repo_dir).expect("failed to create local repository directory");
        std::fs::write(repo_dir.join("README.md"), "# repo\n")
            .expect("failed to seed local repository");

        let repository = repository_repo::create_repository(
            &pool,
            "test-repo",
            "Test Repo",
            &repo_dir.to_string_lossy(),
            "",
            "main",
        )
        .await
        .expect("failed to register repository");

        let work_item = work_item_repo::create_work_item(
            &pool,
            "test-work-item",
            &product.id,
            Some(&module.id),
            None,
            None,
            "Implement tool loop",
            "problem",
            "description",
            "acceptance",
            "constraints",
            "feature",
            "medium",
            "medium",
        )
        .await
        .expect("failed to create work item");

        sqlx::query("UPDATE work_items SET active_repo_id=? WHERE id=?")
            .bind(&repository.id)
            .bind(&work_item.id)
            .execute(&pool)
            .await
            .expect("failed to set work item active repository");

        let workflow_run =
            workflow_repo::create_workflow_run(&pool, "test-workflow-run", &work_item.id)
                .await
                .expect("failed to create workflow run");
        AgentService::set_test_model_outputs_for_workflow(
            &workflow_run.id,
            vec![
                json!({
                    "type": "tool_call",
                    "tool": "repo.write_file",
                    "reason": "create implementation artifact",
                    "arguments": {
                        "path": "hello.txt",
                        "content": "hello from tool loop\n"
                    }
                })
                .to_string(),
                json!({
                    "type": "final",
                    "summary": "Implemented coding changes",
                    "result": "done"
                })
                .to_string(),
            ],
        );

        let db_arc = Arc::new(pool.clone());
        let model_service = Arc::new(ModelService::new(Arc::clone(&db_arc)));
        let artifact_dir = temp_root.join("artifacts");
        let workspace_dir = temp_root.join("workspaces");
        std::fs::create_dir_all(&artifact_dir).expect("failed to create artifact directory");
        std::fs::create_dir_all(&workspace_dir).expect("failed to create workspace directory");

        let service = AgentService::new(
            Arc::clone(&db_arc),
            Arc::clone(&model_service),
            artifact_dir.clone(),
            workspace_dir.clone(),
        );

        let agent_run = service
            .run_agent_for_stage(&workflow_run.id, "coding")
            .await
            .expect("coding stage run failed");
        assert_eq!(
            agent_run.status,
            crate::domain::agent::AgentRunStatus::Completed
        );

        let artifacts = artifact_repo::list_work_item_artifacts(&pool, &work_item.id)
            .await
            .expect("failed to list artifacts");
        let trace_artifact = artifacts
            .iter()
            .find(|artifact| artifact.artifact_type == "coding_tool_trace")
            .expect("missing coding_tool_trace artifact");
        assert!(
            std::path::Path::new(&trace_artifact.storage_path).exists(),
            "tool trace file does not exist: {}",
            trace_artifact.storage_path
        );
        let trace_content = std::fs::read_to_string(&trace_artifact.storage_path)
            .expect("failed to read trace file");
        assert!(
            trace_content.contains("tool_result"),
            "trace artifact did not include tool_result entry"
        );
        assert!(
            trace_content.contains("repo.write_file"),
            "trace artifact did not include invoked tool name"
        );

        let applied_file = std::fs::read_to_string(repo_dir.join("hello.txt"))
            .expect("expected file written by coding tool loop");
        assert_eq!(applied_file, "hello from tool loop\n");

        let _ = std::fs::remove_dir_all(temp_root);
    }
}
