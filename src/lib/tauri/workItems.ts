import { invoke } from "./core";
import type {
  ProductWorkItemSummary,
  WorkItem,
  WorkItemPage,
  WorkItemScopeSummary,
} from "../types";

// Work item commands
export const createWorkItem = (data: {
  productId: string; productAreaId?: string; capabilityId?: string; sourceNodeId?: string; sourceNodeType?: string; parentWorkItemId?: string;
  title: string; problemStatement: string; description: string; acceptanceCriteria: string;
  constraints: string; workItemType: string; priority: string; complexity: string;
}) =>
  invoke<WorkItem>("create_work_item", {
    request: {
      product_id: data.productId,
      product_area_id: data.productAreaId,
      capability_id: data.capabilityId,
      source_node_id: data.sourceNodeId,
      source_node_type: data.sourceNodeType,
      parent_work_item_id: data.parentWorkItemId,
      title: data.title,
      problem_statement: data.problemStatement,
      description: data.description,
      acceptance_criteria: data.acceptanceCriteria,
      constraints: data.constraints,
      work_item_type: data.workItemType,
      priority: data.priority,
      complexity: data.complexity,
    },
  });

export const getWorkItem = (id: string) => invoke<WorkItem>("get_work_item", { id });
export const listWorkItemsPage = (filters?: {
  productId?: string;
  productAreaId?: string;
  capabilityId?: string;
  sourceNodeId?: string;
  sourceNodeType?: string;
  status?: string;
  limit?: number;
  offset?: number;
  topLevelOnly?: boolean;
}) =>
  invoke<WorkItemPage>("list_work_items_page", {
    request: {
      product_id: filters?.productId,
      product_area_id: filters?.productAreaId,
      capability_id: filters?.capabilityId,
      source_node_id: filters?.sourceNodeId,
      source_node_type: filters?.sourceNodeType,
      status: filters?.status,
      limit: filters?.limit,
      offset: filters?.offset,
      top_level_only: filters?.topLevelOnly,
    },
  });
export const summarizeWorkItemsByProduct = () =>
  invoke<ProductWorkItemSummary[]>("summarize_work_items_by_product");
export const summarizeWorkItemsByScope = (filters?: { productId?: string }) =>
  invoke<WorkItemScopeSummary[]>("summarize_work_items_by_scope", {
    product_id: filters?.productId,
  });
export const updateWorkItem = (data: {
  id: string;
  title?: string;
  description?: string;
  status?: string;
  problemStatement?: string;
  acceptanceCriteria?: string;
  constraints?: string;
}) =>
  invoke<WorkItem>("update_work_item", {
    request: {
      id: data.id,
      title: data.title,
      description: data.description,
      status: data.status,
      problem_statement: data.problemStatement,
      acceptance_criteria: data.acceptanceCriteria,
      constraints: data.constraints,
    },
  });
export const assignWorkItemWorkspace = (data: { id: string; repositoryId: string | null; branchName: string | null }) =>
  invoke<WorkItem>("assign_work_item_workspace", {
    id: data.id,
    repository_id: data.repositoryId,
    branch_name: data.branchName,
    repositoryId: data.repositoryId,
    branchName: data.branchName,
  });
export const deleteWorkItem = (id: string) => invoke("delete_work_item", { id });
export const getSubWorkItems = (
  workItemId: string,
  options?: { limit?: number; offset?: number },
) =>
  invoke<WorkItem[]>("get_sub_work_items", {
    work_item_id: workItemId,
    limit: options?.limit,
    offset: options?.offset,
  });
export const reorderWorkItems = (orderedIds: string[]) => invoke("reorder_work_items", { ordered_ids: orderedIds });
