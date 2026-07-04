import { WorkItemArtifactModal } from "./WorkItemArtifactModal";
import { WorkItemBacklogTab } from "./WorkItemBacklogTab";
import { WorkItemDetailTab } from "./WorkItemDetailTab";
import { WorkItemExternalCliTab } from "./WorkItemExternalCliTab";
import {
  WorkItemCreateModal,
  WorkItemEditModal,
} from "./WorkItemFormModals";
import { WorkItemReviewSummaryCards } from "./WorkItemReviewSummaryCards";
import { WorkItemReviewWorkflowCard } from "./WorkItemReviewWorkflowCard";
import { WorkItemWorkspaceReadinessCard } from "./WorkItemWorkspaceReadinessCard";
import type { WorkItemListPageController } from "../hooks/useWorkItemListPageController";
import { styles } from "../lib/workItemListPageStyles";

type WorkItemListPageBodyProps = {
  controller: WorkItemListPageController;
};

export function WorkItemListPageBody({ controller }: WorkItemListPageBodyProps) {
  const {
    actionError,
    actionInfo,
    activeCapability,
    activeExternalCliRun,
    activeExternalCliRunId,
    activeProduct,
    activeProductArea,
    activeProductId,
    activeWorkflowStage,
    approveMutation,
    approvals,
    artifactModalArtifact,
    artifactModalContent,
    artifacts,
    artifactsByAgentRunId,
    backlogRenderedRangeLabel,
    backlogViewportRef,
    backlogWindow,
    bulkActionInFlight,
    closeWorkItemCreateDialog,
    completedStages,
    createForm,
    createMutation,
    createWorkspace,
    createWorkItemScopeLabel,
    dagNodeById,
    deleteMutation,
    draggedWorkItemId,
    externalCliMutation,
    externalCliProviderInFlight,
    externalCliRunEvents,
    externalCliRuns,
    externalCliTerminalOutput,
    failWorkflowRunMutation,
    findingSeverityCounts,
    findings,
    formError,
    hasNextWorkItemPage,
    isCreateWorkspacePending,
    isEditingWorkItem,
    isLoading,
    isRowActionPending,
    isStaleRun,
    laneStatusById,
    latestApproval,
    latestArtifact,
    latestExternalCliEvent,
    latestExternalCliRun,
    latestWorkflowRun,
    latestWorkflowRunByWorkItemId,
    modelCallsByAgentRunId,
    openOverflowWorkItemId,
    openWorkItemCreateDialog,
    orderedWorkItems,
    planApprovalMutation,
    planRejectMutation,
    rejectMutation,
    reorderWorkItemsMutation,
    resolvedRepository,
    restartWorkflowMutation,
    runBulkApprovalAction,
    runRowApprovalAction,
    scopeDescriptor,
    selectedBacklogItemIds,
    selectedBacklogItems,
    selectedDagNode,
    selectedDagNodeId,
    selectedWorkItemId,
    selectedWorkItemSummary,
    setActiveWorkItem,
    setArtifactModalArtifact,
    setBacklogScrollTop,
    setCreateForm,
    setDraggedWorkItemId,
    setIsEditingWorkItem,
    setOpenOverflowWorkItemId,
    setSelectedArtifactStage,
    setSelectedBacklogItemIds,
    setSelectedExternalCliRunId,
    setShowCreateForm,
    setStatusFilter,
    setWorkItemDraft,
    setWorkItemOrderIds,
    setWorkItemPageIndex,
    setWorkItemWorkspaceTab,
    showCreateForm,
    stageHistoryForFocusedStage,
    stageLabel,
    statusFilter,
    subWorkItems,
    testReviewApproveMutation,
    testReviewRejectMutation,
    topArtifactTypes,
    updateWorkItemMutation,
    workItemCreateDialogOpen,
    workItemDraft,
    workItemOrderIds,
    workItemOwnerMap,
    workItemPageEnd,
    workItemPageIndex,
    workItemPageStart,
    workItemWorkspaceTab,
    workflowElapsedLabel,
    workflowModelUsage,
    workflowMutation,
    workflowReadiness,
    workflowRunId,
    workspaceAssignmentPanel,
  } = controller;

  return (
    <div style={styles.page}>
      <div style={styles.header}>
        <div style={styles.titleBlock}>
          <h1 style={styles.title}>Delivery / Builder</h1>
          <div style={styles.subtitle}>Execute product delivery as stories and tasks, inspect evidence, and keep implementation scoped to the selected product management node.</div>
        </div>
      </div>

      <div style={styles.panel}>
        <div style={styles.panelInner}>
          <div style={styles.tabBar}>
            <button style={workItemWorkspaceTab === "backlog" ? styles.tabActive : styles.tab} onClick={() => setWorkItemWorkspaceTab("backlog")}>Backlog</button>
            <button style={workItemWorkspaceTab === "detail" ? styles.tabActive : styles.tab} onClick={() => setWorkItemWorkspaceTab("detail")}>Story Detail</button>
            <button style={workItemWorkspaceTab === "external_cli" ? styles.tabActive : styles.tab} onClick={() => setWorkItemWorkspaceTab("external_cli")}>External CLI</button>
            <button style={workItemWorkspaceTab === "review" ? styles.tabActive : styles.tab} onClick={() => setWorkItemWorkspaceTab("review")}>Review</button>
          </div>

          {workItemWorkspaceTab === "backlog" && (
            <WorkItemBacklogTab
              activeProduct={activeProduct}
              activeProductArea={activeProductArea}
              activeCapability={activeCapability}
              selectedBacklogItems={selectedBacklogItems}
              bulkActionInFlight={bulkActionInFlight}
              onRunBulkApprovalAction={(action) => void runBulkApprovalAction(action)}
              onOpenCreateStory={() => {
                setCreateForm((current) => ({ ...current, parentWorkItemId: null }));
                openWorkItemCreateDialog();
              }}
              activeProductId={activeProductId}
              actionError={actionError}
              statusFilter={statusFilter}
              onStatusFilterChange={setStatusFilter}
              scopeDescriptor={scopeDescriptor}
              workItemPageIndex={workItemPageIndex}
              workItemPageStart={workItemPageStart}
              workItemPageEnd={workItemPageEnd}
              backlogRenderedRangeLabel={backlogRenderedRangeLabel}
              onWorkItemPageIndexChange={setWorkItemPageIndex}
              hasNextWorkItemPage={hasNextWorkItemPage}
              isLoading={isLoading}
              orderedWorkItems={orderedWorkItems}
              backlogViewportRef={backlogViewportRef}
              onBacklogScrollTopChange={setBacklogScrollTop}
              backlogWindow={backlogWindow}
              latestWorkflowRunByWorkItemId={latestWorkflowRunByWorkItemId}
              workItemOwnerMap={workItemOwnerMap}
              selectedWorkItemId={selectedWorkItemId}
              draggedWorkItemId={draggedWorkItemId}
              onDraggedWorkItemIdChange={setDraggedWorkItemId}
              workItemOrderIds={workItemOrderIds}
              onWorkItemOrderIdsChange={setWorkItemOrderIds}
              onReorderWorkItems={(orderedIds) => reorderWorkItemsMutation.mutate(orderedIds)}
              onSelectWorkItem={(workItemId) => {
                setActiveWorkItem(workItemId);
                setWorkItemWorkspaceTab("detail");
              }}
              selectedBacklogItemIds={selectedBacklogItemIds}
              onSelectedBacklogItemIdsChange={setSelectedBacklogItemIds}
              onRunRowApprovalAction={(workItemId, action) => void runRowApprovalAction(workItemId, action)}
              isRowActionPending={isRowActionPending}
              openOverflowWorkItemId={openOverflowWorkItemId}
              onOpenOverflowWorkItemIdChange={setOpenOverflowWorkItemId}
              onDeleteWorkItem={(workItemId) => deleteMutation.mutate(workItemId)}
            />
          )}

          {workItemWorkspaceTab === "detail" && (
            <WorkItemDetailTab
              workItem={selectedWorkItemSummary}
              workflowReadiness={workflowReadiness}
              isWorkflowPending={workflowMutation.isPending}
              onStartWorkflow={() => workflowMutation.mutate()}
              onApprove={() => approveMutation.mutate()}
              onReject={() => rejectMutation.mutate()}
              workflowRunId={workflowRunId}
              isRestartWorkflowPending={restartWorkflowMutation.isPending}
              onRestartWorkflow={() => restartWorkflowMutation.mutate()}
              onEditStory={() => setIsEditingWorkItem(true)}
              onCreateTask={() => {
                if (!selectedWorkItemSummary) {
                  return;
                }
                setCreateForm((current) => ({ ...current, parentWorkItemId: selectedWorkItemSummary.id }));
                openWorkItemCreateDialog();
              }}
              resolvedRepository={resolvedRepository}
              isExternalCliPending={externalCliMutation.isPending}
              externalCliProviderInFlight={externalCliProviderInFlight}
              onRunExternalCli={(provider) => externalCliMutation.mutate(provider)}
              latestExternalCliRun={latestExternalCliRun}
              onOpenExternalCliRun={(runId) => {
                setSelectedExternalCliRunId(runId);
                setWorkItemWorkspaceTab("external_cli");
              }}
              actionError={actionError}
              actionInfo={actionInfo}
              workspaceAssignmentPanel={workspaceAssignmentPanel}
              isCreateWorkspacePending={isCreateWorkspacePending}
              onCreateWorkspace={createWorkspace}
              latestWorkflowRun={latestWorkflowRun}
            />
          )}

          {workItemWorkspaceTab === "external_cli" && (
            <WorkItemExternalCliTab
              selectedWorkItemId={selectedWorkItemId}
              resolvedRepository={resolvedRepository}
              isRunPending={externalCliMutation.isPending}
              providerInFlight={externalCliProviderInFlight}
              onRunProvider={(provider) => externalCliMutation.mutate(provider)}
              actionError={actionError}
              actionInfo={actionInfo}
              activeRun={activeExternalCliRun}
              activeRunId={activeExternalCliRunId}
              latestEvent={latestExternalCliEvent}
              events={externalCliRunEvents}
              terminalOutput={externalCliTerminalOutput}
              runs={externalCliRuns}
              artifacts={artifacts}
              onOpenArtifact={setArtifactModalArtifact}
              onSelectRun={setSelectedExternalCliRunId}
            />
          )}

          {workItemWorkspaceTab === "review" && (
            <>
              <WorkItemWorkspaceReadinessCard
                resolvedRepository={resolvedRepository}
                selectedWorkItem={selectedWorkItemSummary}
                workspaceAssignmentPanel={workspaceAssignmentPanel}
                isCreateWorkspacePending={isCreateWorkspacePending}
                onCreateWorkspace={createWorkspace}
              />
              <div style={styles.sectionTitle}>Review Signals</div>
              <WorkItemReviewWorkflowCard
                workflowRunId={workflowRunId}
                latestWorkflowRun={latestWorkflowRun}
                activeWorkflowStage={activeWorkflowStage}
                stageLabel={stageLabel}
                workflowElapsedLabel={workflowElapsedLabel}
                isStaleRun={isStaleRun}
                workflowModelUsage={workflowModelUsage}
                laneStatusById={laneStatusById}
                dagNodeById={dagNodeById}
                completedStages={completedStages}
                selectedDagNodeId={selectedDagNodeId}
                selectedDagNode={selectedDagNode}
                stageRuns={controller.stageRuns}
                artifactsByAgentRunId={artifactsByAgentRunId}
                modelCallsByAgentRunId={modelCallsByAgentRunId}
                stageHistoryForFocusedStage={stageHistoryForFocusedStage}
                isFailWorkflowRunPending={failWorkflowRunMutation.isPending}
                isRestartWorkflowPending={restartWorkflowMutation.isPending}
                isPlanApprovalPending={planApprovalMutation.isPending}
                isPlanRejectPending={planRejectMutation.isPending}
                isTestReviewApprovePending={testReviewApproveMutation.isPending}
                isTestReviewRejectPending={testReviewRejectMutation.isPending}
                onFailWorkflowRun={() => failWorkflowRunMutation.mutate()}
                onRestartWorkflow={() => restartWorkflowMutation.mutate()}
                onApprovePlan={() => planApprovalMutation.mutate()}
                onRejectPlan={() => planRejectMutation.mutate()}
                onApproveTestReview={() => testReviewApproveMutation.mutate()}
                onRejectTestReview={() => testReviewRejectMutation.mutate()}
                onSelectArtifactStage={setSelectedArtifactStage}
                onOpenArtifact={setArtifactModalArtifact}
              />
              <WorkItemReviewSummaryCards
                subWorkItems={subWorkItems}
                approvals={approvals}
                artifacts={artifacts}
                findings={findings}
                latestApproval={latestApproval}
                latestArtifact={latestArtifact}
                topArtifactTypes={topArtifactTypes}
                findingSeverityCounts={findingSeverityCounts}
              />
            </>
          )}
        </div>
      </div>

      {artifactModalArtifact && (
        <WorkItemArtifactModal
          artifact={artifactModalArtifact}
          content={artifactModalContent}
          onClose={() => setArtifactModalArtifact(null)}
        />
      )}

      {(showCreateForm || workItemCreateDialogOpen) && (
        <WorkItemCreateModal
          createForm={createForm}
          setCreateForm={setCreateForm}
          creationScopeLabel={createWorkItemScopeLabel}
          hasActiveProduct={!!activeProductId}
          formError={formError}
          isPending={createMutation.isPending}
          onClose={() => { setShowCreateForm(false); closeWorkItemCreateDialog(); }}
          onSubmit={() => createMutation.mutate()}
        />
      )}

      {isEditingWorkItem && selectedWorkItemSummary && (
        <WorkItemEditModal
          draft={workItemDraft}
          setDraft={setWorkItemDraft}
          formError={formError}
          isPending={updateWorkItemMutation.isPending}
          onClose={() => setIsEditingWorkItem(false)}
          onSubmit={() => updateWorkItemMutation.mutate()}
        />
      )}
    </div>
  );
}
