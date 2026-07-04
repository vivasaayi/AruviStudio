export interface Artifact {
  id: string;
  work_item_id: string;
  workflow_run_id: string | null;
  agent_run_id: string | null;
  artifact_type: string;
  storage_path: string;
  summary: string;
  content_type: string;
  size_bytes: number | null;
  created_at: string;
}

export interface Finding {
  id: string;
  work_item_id: string;
  source_agent_run_id: string | null;
  category: "security" | "performance";
  severity: "critical" | "high" | "medium" | "low" | "info";
  title: string;
  description: string;
  status: "open" | "resolved" | "wont_fix" | "deferred";
  is_blocking: boolean;
  linked_followup_work_item_id: string | null;
  created_at: string;
  updated_at: string;
}
