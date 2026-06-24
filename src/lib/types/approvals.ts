export interface Approval {
  id: string;
  work_item_id: string;
  workflow_run_id: string | null;
  approval_type: "task_approval" | "plan_approval" | "test_review";
  status: "pending" | "approved" | "rejected";
  notes: string;
  acted_at: string | null;
  created_at: string;
}
