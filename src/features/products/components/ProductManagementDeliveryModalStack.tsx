import type { ProductManagementModalStackProps } from "./ProductManagementModalStack";
import {
  DeleteHierarchyNodeModal,
  DeleteManagementWorkItemModal,
  ManagementWorkItemFormModal,
} from "./ProductManagementDeliveryModals";

type ProductManagementDeliveryModalStackProps = Pick<
  ProductManagementModalStackProps,
  | "deleteHierarchyCandidate"
  | "deleteHierarchyConfirmName"
  | "deleteHierarchyConfirmChecked"
  | "deleteHierarchyReady"
  | "isDeleteHierarchyPending"
  | "onCloseDeleteHierarchy"
  | "onDeleteHierarchyConfirmNameChange"
  | "onDeleteHierarchyConfirmCheckedChange"
  | "onDeleteHierarchy"
  | "deleteWorkItemCandidate"
  | "deleteWorkItemConfirmName"
  | "deleteWorkItemConfirmChecked"
  | "deleteWorkItemReady"
  | "isDeleteWorkItemPending"
  | "onCloseDeleteWorkItem"
  | "onDeleteWorkItemConfirmNameChange"
  | "onDeleteWorkItemConfirmCheckedChange"
  | "onDeleteWorkItem"
  | "storyDialogMode"
  | "selectedFeatureTitle"
  | "storyDraft"
  | "setStoryDraft"
  | "canSubmitStory"
  | "isCreateStoryPending"
  | "isUpdateStoryPending"
  | "onCloseStoryDialog"
  | "onSubmitStory"
  | "taskDialogMode"
  | "selectedStoryTitle"
  | "taskDraft"
  | "setTaskDraft"
  | "canSubmitTask"
  | "isCreateTaskPending"
  | "isUpdateTaskPending"
  | "onCloseTaskDialog"
  | "onSubmitTask"
  | "formError"
>;

export function ProductManagementDeliveryModalStack({
  deleteHierarchyCandidate,
  deleteHierarchyConfirmName,
  deleteHierarchyConfirmChecked,
  deleteHierarchyReady,
  isDeleteHierarchyPending,
  onCloseDeleteHierarchy,
  onDeleteHierarchyConfirmNameChange,
  onDeleteHierarchyConfirmCheckedChange,
  onDeleteHierarchy,
  deleteWorkItemCandidate,
  deleteWorkItemConfirmName,
  deleteWorkItemConfirmChecked,
  deleteWorkItemReady,
  isDeleteWorkItemPending,
  onCloseDeleteWorkItem,
  onDeleteWorkItemConfirmNameChange,
  onDeleteWorkItemConfirmCheckedChange,
  onDeleteWorkItem,
  storyDialogMode,
  selectedFeatureTitle,
  storyDraft,
  setStoryDraft,
  canSubmitStory,
  isCreateStoryPending,
  isUpdateStoryPending,
  onCloseStoryDialog,
  onSubmitStory,
  taskDialogMode,
  selectedStoryTitle,
  taskDraft,
  setTaskDraft,
  canSubmitTask,
  isCreateTaskPending,
  isUpdateTaskPending,
  onCloseTaskDialog,
  onSubmitTask,
  formError,
}: ProductManagementDeliveryModalStackProps) {
  return (
    <>
      {deleteHierarchyCandidate && (
        <DeleteHierarchyNodeModal
          candidate={deleteHierarchyCandidate}
          confirmName={deleteHierarchyConfirmName}
          confirmChecked={deleteHierarchyConfirmChecked}
          isReady={deleteHierarchyReady}
          isPending={isDeleteHierarchyPending}
          formError={formError}
          onClose={onCloseDeleteHierarchy}
          onConfirmNameChange={onDeleteHierarchyConfirmNameChange}
          onConfirmCheckedChange={onDeleteHierarchyConfirmCheckedChange}
          onDelete={onDeleteHierarchy}
        />
      )}

      {deleteWorkItemCandidate && (
        <DeleteManagementWorkItemModal
          candidate={deleteWorkItemCandidate}
          confirmName={deleteWorkItemConfirmName}
          confirmChecked={deleteWorkItemConfirmChecked}
          isReady={deleteWorkItemReady}
          isPending={isDeleteWorkItemPending}
          formError={formError}
          onClose={onCloseDeleteWorkItem}
          onConfirmNameChange={onDeleteWorkItemConfirmNameChange}
          onConfirmCheckedChange={onDeleteWorkItemConfirmCheckedChange}
          onDelete={onDeleteWorkItem}
        />
      )}

      {storyDialogMode !== "closed" && (
        <ManagementWorkItemFormModal
          kind="story"
          mode={storyDialogMode}
          contextLabel="Feature"
          contextTitle={selectedFeatureTitle}
          draft={storyDraft}
          setDraft={setStoryDraft}
          canSubmit={canSubmitStory}
          isCreatePending={isCreateStoryPending}
          isUpdatePending={isUpdateStoryPending}
          formError={formError}
          onClose={onCloseStoryDialog}
          onSubmit={onSubmitStory}
        />
      )}

      {taskDialogMode !== "closed" && (
        <ManagementWorkItemFormModal
          kind="task"
          mode={taskDialogMode}
          contextLabel="Story"
          contextTitle={selectedStoryTitle}
          draft={taskDraft}
          setDraft={setTaskDraft}
          canSubmit={canSubmitTask}
          isCreatePending={isCreateTaskPending}
          isUpdatePending={isUpdateTaskPending}
          formError={formError}
          onClose={onCloseTaskDialog}
          onSubmit={onSubmitTask}
        />
      )}
    </>
  );
}
