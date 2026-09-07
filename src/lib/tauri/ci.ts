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

export const getLocalCiStatus = () => invoke<{ status: string; protocol: number }>("get_local_ci_status");
export const listLocalCiTargets = () => invoke<Array<{ id: string; profile: string }>>("list_local_ci_targets");
export const listLocalCiRuns = () => invoke<LocalCiRun[]>("list_local_ci_runs");
export const queueLocalPreview = (commit: string, productId?: string) =>
  invoke<{ run_id: number }>("queue_local_preview", { commit, productId });
export const openLocalPreview = () => invoke<void>("open_local_preview");
