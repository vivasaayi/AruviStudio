import { PlannerComposerPanel } from "./PlannerComposerPanel";
import { PlannerHeader } from "./PlannerHeader";
import { PlannerPageContent } from "./PlannerPageContent";
import { PlannerRepositoryModal } from "./PlannerRepositoryModal";
import { PlannerSidebar } from "./PlannerSidebar";
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
    composerRef,
    composerScopeChips,
    confirmPendingPlan,
    deleteSelectedDraftNode,
    designPacketError,
    designPacketPath,
    dismissPendingPlan,
    draft,
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
    isListening,
    isPlannerBusy,
    isVoiceSubmitting,
    isTranscribing,
    latestDraftPlan,
    latestTraceEvents,
    modelName,
    navigate,
    pendingPlan,
    pendingVoiceTranscript,
    plannerModelPickerOptions,
    plannerModelPickerValue,
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
    send,
    setActiveProduct,
    setDraftChildName,
    setDraftChildSummary,
    setDraftChildType,
    setDraft,
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
    toggleListening,
    voiceElapsedMs,
    voiceEnabled,
    voiceActivity,
    applyPromptSuggestion,
    renameDraftName,
    renameSelectedDraftNode,
    addChildToSelectedDraftNode,
  } = controller;

  const plannerComposer = (
    <PlannerComposerPanel
      draft={draft}
      onDraftChange={setDraft}
      onSend={() => {
        void send();
      }}
      onToggleListening={() => {
        void toggleListening();
      }}
      onOpenDraftWorkspace={() => setPlannerView("draft")}
      onConfirm={() => setDraft("confirm")}
      onDismiss={dismissPendingPlan}
      isPlannerBusy={isPlannerBusy}
      voiceEnabled={voiceEnabled}
      isListening={isListening}
      isTranscribing={isTranscribing}
      isVoiceSubmitting={isVoiceSubmitting}
      pendingVoiceTranscript={pendingVoiceTranscript}
      draftTreeNodesLength={draftTreeNodes.length}
      pendingPlan={pendingPlan}
      voiceActivity={voiceActivity}
      composerRef={composerRef}
      scopeChips={composerScopeChips}
      isProductSelected={Boolean(selectedProductId)}
    />
  );

  const plannerSidebar = (
    <PlannerSidebar
      isCompactScreen={isCompactScreen}
      hasTreeData={hasTreeData}
      plannerWorkItemsHasMore={controller.plannerWorkItemsHasMore}
      draftTreeNodes={draftTreeNodes}
      selectedDraftNodeId={selectedDraftNodeId}
      onSelectDraftNode={setSelectedDraftNodeId}
      expandedDraftNodeIdSet={expandedDraftNodeIdSet}
      onToggleDraftNodeExpanded={toggleDraftNodeExpanded}
      pendingPlan={pendingPlan}
    />
  );

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
