use crate::domain::agent::AgentDefinition;
use crate::error::AppError;
use crate::services::agent_execution_limits::AgentExecutionBoundaries;
use crate::services::agent_service::TeamExecutionContext;
use std::collections::HashMap;
use tracing::{debug, info};

pub(crate) fn build_agent_prompt(
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

    append_stage_instructions(&mut prompt, stage_name);
    append_context(&mut prompt, context, stage_name);
    append_boundaries(&mut prompt, boundaries);

    prompt.push_str("\nProvide your response in a clear, structured format.");

    info!(agent_id = %agent_def.id, stage_name = %stage_name, prompt_length = prompt.len(), "Successfully built agent prompt");
    Ok(prompt)
}

fn append_stage_instructions(prompt: &mut String, stage_name: &str) {
    match stage_name {
        "requirement_analysis" => {
            prompt.push_str(
                "Analyze the following work item and provide a detailed requirement analysis. ",
            );
            prompt.push_str(
                "Identify any missing information, clarify ambiguities, and suggest improvements. ",
            );
            prompt.push_str("Consider the broader product context and technical constraints.\n\n");
        }
        "planning" => {
            prompt.push_str("Create a detailed implementation plan for the work item. ");
            prompt.push_str("Break down the work into specific steps, identify files that need to be created or modified, ");
            prompt.push_str("and outline the testing approach.\n\n");
        }
        "coding" => {
            prompt.push_str("Implement the code changes according to the approved plan. ");
            prompt.push_str(
                "Use tool-calling JSON to inspect files, search code, and apply precise edits. ",
            );
            prompt.push_str(
                "Start with minimal context, then fetch additional files on demand through tools. ",
            );
            prompt.push_str(
                "Prefer targeted edits with repo.replace_range for function/class-level changes. ",
            );
            prompt.push_str(
                "Use repo.write_file for full-file rewrites only after reading current content. ",
            );
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
            prompt.push_str("Describe the user interactions and expected behaviors to test.\n\n");
        }
        "qa_validation" => {
            prompt.push_str("Review the implementation, tests, and outputs. ");
            prompt
                .push_str("Validate that acceptance criteria are met and identify any issues.\n\n");
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
            prompt.push_str("Execute your assigned work item based on the provided context.\n\n");
        }
    }
}

fn append_context(prompt: &mut String, context: &HashMap<String, String>, stage_name: &str) {
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
}

fn append_boundaries(prompt: &mut String, boundaries: &AgentExecutionBoundaries) {
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
}
