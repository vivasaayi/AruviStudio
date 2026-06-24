import { PlannerHeader } from "./PlannerHeader";
import { PlannerPageContent } from "./PlannerPageContent";
import { PlannerRepositoryModal } from "./PlannerRepositoryModal";
import type { PlannerPageController } from "../hooks/usePlannerPageController";
import { styles } from "../lib/plannerPageStyles";

type PlannerPageBodyProps = {
  controller: PlannerPageController;
};

export function PlannerPageBody({ controller }: PlannerPageBodyProps) {
  const {
    allowedDraftChildTypes,
    analyzeSelectedRepository,
    browseRepositoryPathForPlanner,
    clearPendingVoiceReview,
    collapseAllDraftNodes,
    confirmPendingPlan,
    deleteSelectedDraftNode,
    designPacketError,
    designPacketPath,
    dismissPendingPlan,
    draftChildName,
    draftChildSummary,
    draftChildType,
    draftEditError,
    draftEditMessage,
    draftTreeNodes,
    draftValidation,
    editableVoiceTranscript,
    expandedDraftNodeIdSet,
    expandAllDraftNodes,
    exportDesignReviewPacket,
    hasTreeData,
    isCompactScreen,
    isExportingDesignPacket,
    isFocusedWorkspaceView,
    isPlannerBusy,
    isVoiceSubmitting,
    latestDraftPlan,
    latestTraceEvents,
    modelName,
    navigate,
    pendingPlan,
    pendingVoiceTranscript,
    plannerComposer,
    plannerModelPickerOptions,
    plannerModelPickerValue,
    plannerSidebar,
    plannerStatusSummary,
    plannerView,
    products,
    providerId,
    providers,
    registerRepositoryForPlanner,
    repoAnalysisError,
    repoAnalysisMessage,
    repositories,
    repositoryPathDraft,
    retryVoiceCapture,
    reviewVoiceBeforeSend,
    revealInFinder,
    selectedDraftNode,
    selectedDraftNodeId,
    selectedDraftNodePath,
    selectedDraftNodePrompts,
    selectedNodeRecentActions,
    selectedProductId,
    selectedRepositoryId,
    setActiveProduct,
    setDraftChildName,
    setDraftChildSummary,
    setDraftChildType,
    setEditableVoiceTranscript,
    setModelName,
    setPlannerView,
    setProviderId,
    setRenameDraftName,
    setRepositoryPathDraft,
    setSelectedDraftNodeId,
    setSelectedRepositoryId,
    setShowCompactTools,
    showCompactTools,
    showRepoModal,
    setShowRepoModal,
    submitPendingVoiceTranscript,
    toggleDraftNodeExpanded,
    voiceElapsedMs,
    applyPromptSuggestion,
    renameDraftName,
    renameSelectedDraftNode,
    addChildToSelectedDraftNode,
  } = controller;

  return (
    <div style={styles.page}>
      <div
        style={
          isCompactScreen
            ? styles.compactStack
            : {
                ...styles.topGrid,
                gridTemplateColumns: isFocusedWorkspaceView ? "minmax(0, 1fr)" : styles.topGrid.gridTemplateColumns,
              }
        }
      >
        <div style={styles.panel}>
          <div style={{ ...(isCompactScreen ? styles.compactPanelBody : styles.panelBody), display: "flex", flexDirection: "column" }}>
            <PlannerHeader
              plannerView={plannerView}
              selectedProductId={selectedProductId}
              products={products}
              plannerModelPickerValue={plannerModelPickerValue}
              plannerModelPickerOptions={plannerModelPickerOptions}
              providerId={providerId}
              providers={providers}
              modelName={modelName}
              selectedDraftNode={selectedDraftNode}
              draftTreeNodesLength={draftTreeNodes.length}
              latestTraceEventsLength={latestTraceEvents.length}
              plannerStatusSummary={plannerStatusSummary}
              isCompactScreen={isCompactScreen}
              showCompactTools={showCompactTools}
              draftValidation={draftValidation}
              pendingVoiceTranscript={pendingVoiceTranscript}
              isPlannerBusy={isPlannerBusy}
              hasPendingPlan={!!pendingPlan}
              onOpenRepositoryModal={() => setShowRepoModal(true)}
              onProductChange={setActiveProduct}
              onCreateProduct={() => navigate("/products")}
              onPlannerModelChange={(nextProviderId, nextModelName) => {
                setProviderId(nextProviderId);
                setModelName(nextModelName);
              }}
              onPlannerViewChange={setPlannerView}
              onToggleCompactTools={() => setShowCompactTools((value) => !value)}
            />

            <PlannerPageContent
              plannerView={plannerView}
              plannerComposer={plannerComposer}
              transcriptRef={controller.transcriptRef}
              pendingVoiceTranscript={pendingVoiceTranscript}
              reviewVoiceBeforeSend={reviewVoiceBeforeSend}
              voiceElapsedMs={voiceElapsedMs}
              isVoiceSubmitting={isVoiceSubmitting}
              editableVoiceTranscript={editableVoiceTranscript}
              isPlannerBusy={isPlannerBusy}
              messages={controller.messages}
              isExportingDesignPacket={isExportingDesignPacket}
              pendingPlan={pendingPlan}
              draftTreeNodes={draftTreeNodes}
              designPacketPath={designPacketPath}
              designPacketError={designPacketError}
              selectedDraftNode={selectedDraftNode}
              draftValidation={draftValidation}
              selectedDraftNodeId={selectedDraftNodeId}
              expandedDraftNodeIds={expandedDraftNodeIdSet}
              selectedDraftNodePath={selectedDraftNodePath}
              renameDraftName={renameDraftName}
              allowedDraftChildTypes={allowedDraftChildTypes}
              draftChildType={draftChildType}
              draftChildName={draftChildName}
              draftChildSummary={draftChildSummary}
              draftEditMessage={draftEditMessage}
              draftEditError={draftEditError}
              selectedDraftNodePrompts={selectedDraftNodePrompts}
              selectedNodeRecentActions={selectedNodeRecentActions}
              latestDraftPlan={latestDraftPlan}
              latestTraceEvents={latestTraceEvents}
              onEditableVoiceTranscriptChange={setEditableVoiceTranscript}
              onSubmitPendingVoiceTranscript={() => void submitPendingVoiceTranscript()}
              onRetryVoiceCapture={() => void retryVoiceCapture()}
              onClearPendingVoiceReview={clearPendingVoiceReview}
              onExportDesignReviewPacket={() => void exportDesignReviewPacket()}
              onConfirmPendingPlan={confirmPendingPlan}
              onDismissPendingPlan={dismissPendingPlan}
              onSelectDraftNode={setSelectedDraftNodeId}
              onToggleDraftNodeExpanded={toggleDraftNodeExpanded}
              onExpandAllDraftNodes={expandAllDraftNodes}
              onCollapseAllDraftNodes={collapseAllDraftNodes}
              onRenameDraftNameChange={setRenameDraftName}
              onRenameSelectedDraftNode={() => void renameSelectedDraftNode()}
              onDeleteSelectedDraftNode={() => void deleteSelectedDraftNode()}
              onDraftChildTypeChange={setDraftChildType}
              onDraftChildNameChange={setDraftChildName}
              onDraftChildSummaryChange={setDraftChildSummary}
              onAddChildToSelectedDraftNode={() => void addChildToSelectedDraftNode()}
              onApplyPromptSuggestion={applyPromptSuggestion}
              onRevealDesignPacket={(path) => void revealInFinder(path)}
              onBackToChat={() => setPlannerView("conversation")}
            />
          </div>
        </div>

        {!isFocusedWorkspaceView && !isCompactScreen ? plannerSidebar : null}
        {!isFocusedWorkspaceView && isCompactScreen && showCompactTools ? plannerSidebar : null}
      </div>

      {showRepoModal ? (
        <PlannerRepositoryModal
          repositories={repositories}
          selectedRepositoryId={selectedRepositoryId}
          repositoryPathDraft={repositoryPathDraft}
          isProductSelected={!!selectedProductId}
          isPlannerBusy={isPlannerBusy}
          hasPlannerModel={!!providerId && !!modelName}
          repoAnalysisMessage={repoAnalysisMessage}
          repoAnalysisError={repoAnalysisError}
          onClose={() => setShowRepoModal(false)}
          onSelectedRepositoryIdChange={setSelectedRepositoryId}
          onRepositoryPathDraftChange={setRepositoryPathDraft}
          onBrowseRepositoryPath={() => void browseRepositoryPathForPlanner()}
          onRegisterRepository={() => void registerRepositoryForPlanner()}
          onAnalyzeRepository={() => void analyzeSelectedRepository()}
        />
      ) : null}
    </div>
  );
}
