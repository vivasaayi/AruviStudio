use super::AgentService;
use crate::domain::agent::AgentRun;
use crate::error::AppError;
use crate::execution::workspace::WorkItemWorkspace;
use crate::persistence::{agent_repo, artifact_repo, work_item_repo, workflow_repo};
use crate::services::agent_execution_limits::{self, AgentExecutionBoundaries};
use tracing::{debug, info, warn};

impl AgentService {
    /// Apply code changes from the legacy coding agent output format.
    pub(super) async fn apply_code_changes(
        &self,
        agent_run: &AgentRun,
        output: &str,
    ) -> Result<(), AppError> {
        debug!(agent_run_id = %agent_run.id, output_length = output.len(), "Applying code changes from agent output");

        let workflow_run =
            workflow_repo::get_workflow_run(&self.db, &agent_run.workflow_run_id).await?;
        let work_item_id = workflow_run.work_item_id;
        debug!(agent_run_id = %agent_run.id, work_item_id = %work_item_id, "Retrieved workflow run for work item");

        let work_item = work_item_repo::get_work_item(&self.db, &work_item_id).await?;
        let agent_def = agent_repo::get_agent_definition(&self.db, &agent_run.agent_id).await?;
        let boundaries = agent_execution_limits::parse_boundaries(&agent_def.boundaries);
        let repo = if let Some(repo_id) = &work_item.active_repo_id {
            Some(crate::persistence::repository_repo::get_repository(&self.db, repo_id).await?)
        } else {
            None
        };

        if let Some(repo) = repo {
            debug!(agent_run_id = %agent_run.id, repo_id = %repo.id, repo_path = %repo.local_path, "Found active repository for work item");

            let workspace = WorkItemWorkspace::create(
                &work_item,
                &agent_run.workflow_run_id,
                &repo,
                &self.workspace_base_path,
            )
            .await?;
            info!(agent_run_id = %agent_run.id, workspace_path = %workspace.base_path.display(), "Created isolated workspace");

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
                    artifact_repo::CreateArtifactInput {
                        id: &artifact_id,
                        work_item_id: &work_item_id,
                        workflow_run_id: Some(&agent_run.workflow_run_id),
                        agent_run_id: Some(&agent_run.id),
                        artifact_type: "coding_applied_files",
                        summary: &format!("Applied {} files to repository", changed_files.len()),
                        storage_path: &applied_files_path.to_string_lossy(),
                    },
                )
                .await?;
            }

            if agent_execution_limits::should_keep_workspace(&boundaries) {
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

    /// Parse legacy coding agent output and apply file changes.
    pub(super) async fn parse_and_apply_changes(
        &self,
        workspace: &WorkItemWorkspace,
        output: &str,
        boundaries: &AgentExecutionBoundaries,
    ) -> Result<Vec<String>, AppError> {
        debug!(workspace_path = %workspace.base_path.display(), output_length = output.len(), "Parsing and applying code changes");

        let lines: Vec<&str> = output.lines().collect();
        let mut current_file: Option<String> = None;
        let mut file_content = String::new();
        let mut files_processed = 0usize;
        let mut changed_files: Vec<String> = Vec::new();
        let max_files = agent_execution_limits::max_files_per_run(boundaries);

        for line in lines {
            if line.starts_with("File: ") || line.starts_with("### File: ") {
                if let Some(file_path) = current_file.take() {
                    if !file_content.trim().is_empty() {
                        if files_processed >= max_files {
                            warn!(workspace_path = %workspace.base_path.display(), max_files = max_files, "Reached max files per run boundary; skipping additional file updates");
                            break;
                        }
                        let Some(normalized_path) =
                            agent_execution_limits::normalize_relative_path(&file_path)
                        else {
                            warn!(workspace_path = %workspace.base_path.display(), file_path = %file_path, "Skipping invalid relative file path from agent output");
                            file_content.clear();
                            continue;
                        };
                        if !agent_execution_limits::is_repo_relative_path_allowed(
                            &normalized_path,
                            boundaries,
                        ) {
                            warn!(workspace_path = %workspace.base_path.display(), file_path = %normalized_path, "Skipping file outside boundaries");
                            file_content.clear();
                            continue;
                        }
                        agent_execution_limits::ensure_write_limit(&file_content, boundaries)?;
                        workspace
                            .write_file(&normalized_path, &file_content)
                            .await?;
                        files_processed += 1;
                        changed_files.push(normalized_path.clone());
                        debug!(workspace_path = %workspace.base_path.display(), file_path = %normalized_path, content_length = file_content.len(), "Wrote file content");
                    }
                }

                let raw = line
                    .trim_start_matches("File: ")
                    .trim_start_matches("### File: ")
                    .trim();
                current_file = Some(raw.to_string());
                file_content.clear();
                debug!(workspace_path = %workspace.base_path.display(), file_path = current_file.as_ref().unwrap(), "Started processing new file");
            } else if current_file.is_some() {
                if line.trim_start().starts_with("```") {
                    continue;
                }
                file_content.push_str(line);
                file_content.push('\n');
            }
        }

        if let Some(file_path) = current_file {
            if !file_content.trim().is_empty() {
                if files_processed < max_files {
                    if let Some(normalized_path) =
                        agent_execution_limits::normalize_relative_path(&file_path)
                    {
                        if agent_execution_limits::is_repo_relative_path_allowed(
                            &normalized_path,
                            boundaries,
                        ) {
                            agent_execution_limits::ensure_write_limit(&file_content, boundaries)?;
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
