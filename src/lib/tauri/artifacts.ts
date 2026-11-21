import { invoke } from "./core";
import type { Artifact, Finding } from "../types";

// Artifact commands
export const listWorkItemArtifacts = (workItemId: string) =>
  invoke<Artifact[]>("list_work_item_artifacts", { workItemId, work_item_id: workItemId });
export const readArtifactContent = (artifactId: string) =>
  invoke<string>("read_artifact_content", { artifactId, artifact_id: artifactId });
export type ArtifactPage = { content: string; offset: number; next_offset: number | null; total_bytes: number; version: string };
export const readArtifactPage = (artifactId: string, offset: number, version: string | null) =>
  invoke<ArtifactPage>("read_artifact_page", { artifactId, artifact_id: artifactId, offset, version });

// Finding commands
export const listWorkItemFindings = (workItemId: string) => invoke<Finding[]>("list_work_item_findings", { work_item_id: workItemId });
