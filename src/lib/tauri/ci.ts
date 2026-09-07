import { invoke } from "./core";

export type LocalCiRun = {
  id: number;
  target: string;
  commit: string;
  status: string;
  created_at: string;
  finished_at?: string | null;
  error?: string | null;
  stages: Array<{ name: string; status: string; details?: Record<string, unknown> }>;
  artifacts: Array<{ id: number; kind: string; sha256: string }>;
};
export type CiTargetBinding = { id: string; product_id: string; repository_id: string; target_id: string; auto_preview: boolean; created_at: string; updated_at: string };
export type CiFeedback = { id: string; ci_run_id: number; commit_sha: string; product_id?: string | null; work_item_id?: string | null; verdict: "works" | "needs_changes" | "blocked"; notes: string; created_at: string };

export const getLocalCiStatus = () => invoke<{ status: string; protocol: number }>("get_local_ci_status");
export const listLocalCiTargets = () => invoke<Array<{ id: string; profile: string }>>("list_local_ci_targets");
export const listLocalCiRuns = () => invoke<LocalCiRun[]>("list_local_ci_runs");
export const queueLocalPreview = (commit: string, productId?: string) =>
  invoke<{ run_id: number }>("queue_local_preview", { commit, productId });
export const queueLocalRelease = (target: "aruvi-studio" | "aruvi-studio-intel", commit: string, productId?: string) =>
  invoke<{ run_id: number }>("queue_local_release", { target, commit, productId });
export const openLocalPreview = () => invoke<void>("open_local_preview");
export const listCiTargetBindings = () => invoke<CiTargetBinding[]>("list_ci_target_bindings");
export const saveCiTargetBinding = (request: { product_id: string; repository_id: string; target_id: string; auto_preview: boolean }) => invoke<CiTargetBinding>("save_ci_target_binding", { request });
export const listCiFeedback = (ciRunId: number) => invoke<CiFeedback[]>("list_ci_feedback", { ciRunId });
export const createCiFeedback = (data: { ciRunId: number; commitSha: string; productId?: string; workItemId?: string; verdict: CiFeedback["verdict"]; notes: string }) => invoke<CiFeedback>("create_ci_feedback", data);
