ALTER TABLE planner_sessions
ADD COLUMN draft_plan_json TEXT NULL;

ALTER TABLE planner_sessions
ADD COLUMN selected_draft_node_id TEXT NULL;
