export interface WorkflowStagePolicy {
  id: string;
  stage_name: string;
  primary_roles: string[];
  fallback_roles: string[];
  coordinator_required: boolean;
  created_at: string;
  updated_at: string;
}

export interface WorkflowRun {
  id: string;
  work_item_id: string;
  workflow_version: string;
  status: "running" | "paused" | "completed" | "failed" | "cancelled";
  current_stage: string;
  assigned_team_id: string | null;
  coordinator_agent_id: string | null;
  pending_stage_name: string | null;
  retry_count: number;
  max_retries: number;
  error_message: string | null;
  started_at: string;
  ended_at: string | null;
  updated_at: string;
}

export interface WorkflowStageHistory {
  id: string;
  workflow_run_id: string;
  from_stage: string;
  to_stage: string;
  trigger: string;
  notes: string;
  transitioned_at: string;
}

export interface AgentRun {
  id: string;
  workflow_run_id: string;
  agent_id: string;
  stage: string;
  status: "pending" | "running" | "completed" | "failed" | "cancelled";
  prompt_snapshot_path: string | null;
  output_snapshot_path: string | null;
  token_count_input: number | null;
  token_count_output: number | null;
  duration_ms: number | null;
  error_message: string | null;
  started_at: string;
  ended_at: string | null;
  created_at: string;
}

export interface ExternalCliRun {
  id: string;
  work_item_id: string;
  provider: "codex" | "claude" | "cursor" | "copilot";
  label: string;
  command: string;
  args: string[];
  prompt: string;
  cwd: string;
  status: "running" | "completed" | "failed" | "cancelled";
  exit_code: number | null;
  duration_ms: number | null;
  stdout_chars: number;
  stderr_chars: number;
  session_log_path: string;
  output_artifact_id: string | null;
  error_message: string | null;
  started_at: string;
  ended_at: string | null;
  created_at: string;
}

export interface ExternalCliRunEvent {
  id: string;
  run_id: string;
  work_item_id: string;
  stream: "lifecycle" | "stdout" | "stderr" | "error";
  message: string;
  sequence: number;
  created_at: string;
}

export interface ModelCall {
  id: string;
  source_kind: string;
  source_id: string | null;
  source_label: string;
  workflow_run_id: string | null;
  agent_run_id: string | null;
  work_item_id: string | null;
  product_id: string | null;
  session_id: string | null;
  agent_id: string | null;
  stage: string | null;
  provider_id: string;
  provider_name: string;
  provider_type: string;
  provider_base_url: string;
  model_id: string | null;
  model_name: string;
  call_index: number;
  request_message_count: number;
  prompt_chars: number;
  response_chars: number;
  request_snapshot_path: string | null;
  response_snapshot_path: string | null;
  max_tokens: number | null;
  temperature: number | null;
  token_count_input: number | null;
  token_count_output: number | null;
  duration_ms: number | null;
  status: "completed" | "failed";
  error_message: string | null;
  created_at: string;
}
