import type { Product, WorkItem } from "../../../lib/types";

type DeleteWorkItemCandidate = {
  workItem: WorkItem;
};

type DeleteHierarchyCandidate = {
  name: string;
};

export function isDeleteProductReady(
  candidate: Product | null,
  confirmName: string,
  archiveConfirmed: boolean,
) {
  return !!candidate
    && confirmName.trim() === candidate.name
    && archiveConfirmed;
}

export function isResetPlanReady(
  candidate: Product | null,
  confirmName: string,
  treeConfirmed: boolean,
) {
  return !!candidate
    && confirmName.trim() === candidate.name
    && treeConfirmed;
}

export function isDeleteHierarchyReady(
  candidate: DeleteHierarchyCandidate | null,
  confirmName: string,
  checked: boolean,
) {
  return !!candidate
    && confirmName.trim() === candidate.name
    && checked;
}

export function isDeleteManagementWorkItemReady(
  candidate: DeleteWorkItemCandidate | null,
  confirmName: string,
  checked: boolean,
) {
  return !!candidate
    && confirmName.trim() === candidate.workItem.title
    && checked;
}

type ProductModalReadinessInput = {
  deleteProductCandidate: Product | null;
  deleteConfirmName: string;
  deleteConfirmArchive: boolean;
  resetPlanCandidate: Product | null;
  resetPlanConfirmName: string;
  resetPlanConfirmTree: boolean;
  deleteHierarchyCandidate: DeleteHierarchyCandidate | null;
  deleteHierarchyConfirmName: string;
  deleteHierarchyConfirmChecked: boolean;
  deleteWorkItemCandidate: DeleteWorkItemCandidate | null;
  deleteWorkItemConfirmName: string;
  deleteWorkItemConfirmChecked: boolean;
};

export function getProductModalReadiness({
  deleteProductCandidate,
  deleteConfirmName,
  deleteConfirmArchive,
  resetPlanCandidate,
  resetPlanConfirmName,
  resetPlanConfirmTree,
  deleteHierarchyCandidate,
  deleteHierarchyConfirmName,
  deleteHierarchyConfirmChecked,
  deleteWorkItemCandidate,
  deleteWorkItemConfirmName,
  deleteWorkItemConfirmChecked,
}: ProductModalReadinessInput) {
  return {
    deleteConfirmationReady: isDeleteProductReady(
      deleteProductCandidate,
      deleteConfirmName,
      deleteConfirmArchive,
    ),
    resetPlanReady: isResetPlanReady(
      resetPlanCandidate,
      resetPlanConfirmName,
      resetPlanConfirmTree,
    ),
    deleteHierarchyReady: isDeleteHierarchyReady(
      deleteHierarchyCandidate,
      deleteHierarchyConfirmName,
      deleteHierarchyConfirmChecked,
    ),
    deleteManagementWorkItemReady: isDeleteManagementWorkItemReady(
      deleteWorkItemCandidate,
      deleteWorkItemConfirmName,
      deleteWorkItemConfirmChecked,
    ),
  };
}
