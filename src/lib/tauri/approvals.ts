import { invoke } from "./core";
import type { Approval } from "../types";

// Approval commands
export const approveWorkItem = (workItemId: string, notes?: string) =>
  invoke<Approval>("approve_work_item", { workItemId, work_item_id: workItemId, notes });
export const rejectWorkItem = (workItemId: string, notes: string) =>
  invoke<Approval>("reject_work_item", { workItemId, work_item_id: workItemId, notes });
export const approveWorkItemPlan = (workItemId: string, notes?: string) =>
  invoke<Approval>("approve_work_item_plan", { workItemId, work_item_id: workItemId, notes });
export const rejectWorkItemPlan = (workItemId: string, notes: string) =>
  invoke<Approval>("reject_work_item_plan", { workItemId, work_item_id: workItemId, notes });
export const approveWorkItemTestReview = (workItemId: string, notes?: string) =>
  invoke<Approval>("approve_work_item_test_review", { workItemId, work_item_id: workItemId, notes });
export const getWorkItemApprovals = (workItemId: string) =>
  invoke<Approval[]>("get_work_item_approvals", { workItemId, work_item_id: workItemId });
