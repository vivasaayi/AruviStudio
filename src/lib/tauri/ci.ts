import { invoke } from "./core";

export type LocalCiArtifact = {
  id: number;
  run_id: number;
  stage: string;
  kind: string;
  path: string;
  sha256: string;
  size: number;
  created_at: string;
};
export type LocalCiStage = { name: string; status: string; details?: Record<string, unknown>; updated_at?: string };
export type LocalCiRun = {
  id: number;
  target: string;
  commit: string;
  plan_digest: string;
  status: string;
  context?: Record<string, unknown>;
  cancellation_requested?: boolean;
  created_at: string;
  started_at?: string | null;
  finished_at?: string | null;
  error?: string | null;
  stages: LocalCiStage[];
  artifacts: LocalCiArtifact[];
};
export type LocalCiApproval = { approved: boolean; status: "approved" | "approval_required" | "pipeline_changed" | "invalid_approval_record"; current_digest: string; approved_digest?: string | null; approved_at?: string | null; error?: string };
export type LocalCiTarget = {
  id: string;
  profile: string;
  repository?: string;
  plan?: { digest?: string; issues?: string[]; stages?: Array<{ id: string; label: string }> };
  approval?: LocalCiApproval;
  capabilities?: { build?: boolean; preview?: boolean; cancel?: boolean; retry?: boolean; logs?: boolean; artifacts?: boolean };
};
export type LocalCiHealth = {
  status: string;
  protocol: number;
  ready?: boolean;
  database?: string;
  schema?: number;
  queue?: { total: number; queued: number; running: number; terminal: number };
  storage?: { run_directory: string; run_directory_exists: boolean };
  last_run_at?: string | null;
  last_finished_at?: string | null;
};
export type CiPromotionReadiness = {
  artifact_id: number; run_id: number; target: string; commit: string; plan_digest: string;
  kind: string; stage: string; path: string; sha256: string; size: number;
  run_status: string; eligible: boolean; promotion_eligible: boolean; reason: string;
  manual_promotion_only: true;
};
export type CiTargetBinding = { id: string; product_id: string; repository_id: string; target_id: string; auto_preview: boolean; created_at: string; updated_at: string };
export type CiFeedback = { id: string; ci_run_id: number; commit_sha: string; product_id?: string | null; work_item_id?: string | null; verdict: "works" | "needs_changes" | "blocked"; notes: string; created_at: string };
export type CiGitCommit = { sha: string; summary: string; committed_at: number };
export type CiGitBranch = { name: string; commit: CiGitCommit };

export type CiTargetKind = "preview" | "release" | "docker";
export const ciTargetKind = (target: Pick<LocalCiTarget, "id" | "profile">): CiTargetKind =>
  target.id.endsWith("-preview") ? "preview" : target.profile === "rust-docker@1" ? "docker" : "release";
export const ciTargetActionLabel = (target: Pick<LocalCiTarget, "id" | "profile">) => {
  switch (ciTargetKind(target)) {
    case "preview": return "Build Preview";
    case "docker": return "Build OCI artifact";
    default: return "Build release artifact";
  }
};
export const ciTargetDescription = (target: Pick<LocalCiTarget, "id" | "profile">) => {
  switch (ciTargetKind(target)) {
    case "preview": return "Safe local Preview candidate";
    case "docker": return "Verified OCI archive; deployment unavailable";
    default: return "Signed release artifact; installation unavailable";
  }
};

export const getLocalCiStatus = () => invoke<LocalCiHealth>("get_local_ci_status");
export const listLocalCiTargets = () => invoke<LocalCiTarget[]>("list_local_ci_targets");
export const listLocalCiRuns = () => invoke<LocalCiRun[]>("list_local_ci_runs");
export const getLocalCiRun = (id: number) => invoke<LocalCiRun>("get_local_ci_run", { id });
export const cancelLocalCiRun = (id: number) => invoke<LocalCiRun>("cancel_local_ci_run", { id });
export const retryLocalCiRun = (id: number) => invoke<LocalCiRun & { run_id: number }>("retry_local_ci_run", { id });
export const getLocalCiLog = (id: number, stage: string, offset = 0) => invoke<{ text: string; next_offset: number }>("get_local_ci_log", { id, stage, offset });
export const getCiPromotionReadiness = (artifactId: number) => invoke<CiPromotionReadiness>("get_ci_promotion_readiness", { artifactId });
export const revealCiArtifact = (artifactId: number) => invoke<void>("reveal_ci_artifact", { artifactId });
export const queueLocalPreview = (commit: string, productId?: string) =>
  invoke<{ run_id: number }>("queue_local_preview", { commit, productId });
export const queueLocalRelease = (target: "aruvi-studio" | "aruvi-studio-intel", commit: string, productId?: string) =>
  invoke<{ run_id: number }>("queue_local_release", { target, commit, productId });
export const listCiRepositoryBranches = (repositoryId: string) => invoke<CiGitBranch[]>("list_ci_repository_branches", { repositoryId });
export const listCiRepositoryCommits = (repositoryId: string) => invoke<CiGitCommit[]>("list_ci_repository_commits", { repositoryId });
export const queueLocalPreviewReference = (data: { repositoryId: string; branch?: string; commit?: string; productId?: string }) =>
  invoke<{ run_id: number }>("queue_local_preview_reference", data);
export const queueLocalTargetReference = (data: { targetId: string; repositoryId: string; branch?: string; commit?: string; productId: string }) =>
  invoke<{ run_id: number }>("queue_local_target_reference", data);
export const queueLocalReleaseReference = (data: { target: "aruvi-studio" | "aruvi-studio-intel"; repositoryId: string; branch?: string; commit?: string; productId?: string }) =>
  invoke<{ run_id: number }>("queue_local_release_reference", data);
export const openLocalPreview = () => invoke<void>("open_local_preview");
export const listCiTargetBindings = () => invoke<CiTargetBinding[]>("list_ci_target_bindings");
export const saveCiTargetBinding = (request: { product_id: string; repository_id: string; target_id: string; auto_preview: boolean }) => invoke<CiTargetBinding>("save_ci_target_binding", { request });
export const deleteCiTargetBinding = (id: string) => invoke<void>("delete_ci_target_binding", { id });
export const listCiFeedback = (ciRunId: number) => invoke<CiFeedback[]>("list_ci_feedback", { ciRunId });
export const createCiFeedback = (data: { ciRunId: number; commitSha: string; productId?: string; workItemId?: string; verdict: CiFeedback["verdict"]; notes: string }) => invoke<CiFeedback>("create_ci_feedback", data);
