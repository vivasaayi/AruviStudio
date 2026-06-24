import React, { useEffect, useRef, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useLocation, useNavigate } from "react-router-dom";
import {
  revealInFinder,
  speakTextNatively,
} from "../../../lib/tauri";
import { speakInBrowser } from "../../shared/voice";
import { PlannerComposerPanel } from "../components/PlannerComposerPanel";
import { PlannerHeader } from "../components/PlannerHeader";
import { PlannerPageContent } from "../components/PlannerPageContent";
import { PlannerRepositoryModal } from "../components/PlannerRepositoryModal";
import { PlannerSidebar } from "../components/PlannerSidebar";
import { usePlannerDesignPacketExport } from "../hooks/usePlannerDesignPacketExport";
import { usePlannerDraftActions } from "../hooks/usePlannerDraftActions";
import { usePlannerDraftEditorState } from "../hooks/usePlannerDraftEditorState";
import { usePlannerPageLifecycle } from "../hooks/usePlannerPageLifecycle";
import { usePlannerPageViewModel } from "../hooks/usePlannerPageViewModel";
import { usePlannerMutationResultHandler } from "../hooks/usePlannerMutationResultHandler";
import { usePlannerPendingPlanActions } from "../hooks/usePlannerPendingPlanActions";
import { usePlannerRepositoryModalState } from "../hooks/usePlannerRepositoryModalState";
import { usePlannerSpeechSettingsState } from "../hooks/usePlannerSpeechSettingsState";
import { usePlannerTurnMutations } from "../hooks/usePlannerTurnMutations";
import { usePlannerVoiceCapture } from "../hooks/usePlannerVoiceCapture";
import { usePlannerVoiceTranscriptHandler } from "../hooks/usePlannerVoiceTranscriptHandler";
import { usePlannerWindowWidth } from "../hooks/usePlannerWindowWidth";
import { styles } from "../lib/plannerPageStyles";
import type { PlannerSpeechModelSelection } from "../lib/plannerModelSelection";
import {
  DEFAULT_ASSISTANT_OPENING,
  makeId,
  type PendingPlan,
  type PlannerMessage,
  type PlannerTreeNode,
} from "../lib/plannerPageModel";
import { useWorkspaceStore } from "../../../state/workspaceStore";
import type { PlannerTraceEvent } from "../../../lib/types";


export function PlannerPage() {
  const queryClient = useQueryClient();
  const location = useLocation();
  const navigate = useNavigate();
  const { activeProductId, activeProductAreaId, activeCapabilityId, activeWorkItemId, setActiveProduct } = useWorkspaceStore();
  const [plannerView, setPlannerView] = useState<"conversation" | "draft" | "trace">("conversation");
  const [providerId, setProviderId] = useState("");
  const [modelName, setModelName] = useState("");
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [draft, setDraft] = useState("");
  const [messages, setMessages] = useState<PlannerMessage[]>([
    { id: makeId(), role: "assistant", content: DEFAULT_ASSISTANT_OPENING },
  ]);
  const [pendingPlan, setPendingPlan] = useState<PendingPlan | null>(null);
  const [draftTreeNodes, setDraftTreeNodes] = useState<PlannerTreeNode[]>([]);
  const [selectedDraftNodeId, setSelectedDraftNodeId] = useState<string | null>(null);
  const [expandedDraftNodeIds, setExpandedDraftNodeIds] = useState<string[]>([]);
  const [latestTraceEvents, setLatestTraceEvents] = useState<PlannerTraceEvent[]>([]);
  const {
    voiceEnabled,
    autoSpeak,
    speechProviderSetting,
    speechModelSetting,
    speechLocaleSetting,
    speechNativeVoiceSetting,
    reviewVoiceBeforeSend,
  } = usePlannerSpeechSettingsState();
  const windowWidth = usePlannerWindowWidth();
  const [showCompactTools, setShowCompactTools] = useState(false);
  const transcriptRef = useRef<HTMLDivElement | null>(null);
  const composerRef = useRef<HTMLTextAreaElement | null>(null);
  const consumedRoutePromptRef = useRef<string | null>(null);
  const activePlannerProductRef = useRef<string | null>(null);
  const plannerBusyRef = useRef(false);
  const submitVoiceTranscriptRef = useRef<(transcript: string) => Promise<void>>(async () => {});
  const speechModelSelectionRef = useRef<PlannerSpeechModelSelection | null>(null);
  const {
    isListening,
    isTranscribing,
    isVoiceSubmitting,
    setIsVoiceSubmitting,
    pendingVoiceTranscript,
    setPendingVoiceTranscript,
    editableVoiceTranscript,
    setEditableVoiceTranscript,
    voiceActivity,
    setVoiceActivity,
    voiceElapsedMs,
    speechError,
    setSpeechError,
    isVoiceCaptureBusy,
    clearPendingVoiceReview,
    submitPendingVoiceTranscript,
    retryVoiceCapture,
    toggleListening,
  } = usePlannerVoiceCapture({
    voiceEnabled,
    reviewVoiceBeforeSend,
    getSpeechModelSelection: () => speechModelSelectionRef.current,
    speechLocaleSetting,
    isPlannerBusy: () => plannerBusyRef.current,
    onSubmitVoiceTranscript: (transcript) => submitVoiceTranscriptRef.current(transcript),
  });

  const {
    products,
    providers,
    models,
    repositories,
    selectedProductId,
    selectedProduct,
    workItems,
    plannerWorkItemsHasMore,
    productTrees,
    hasTreeData,
    isFocusedWorkspaceView,
    isCompactScreen,
    selectedDraftNode,
    selectedDraftNodePath,
    expandedDraftNodeIdSet,
    latestDraftPlan,
    selectedDraftNodePrompts,
    allowedDraftChildTypes,
    draftValidation,
    selectedNodeRecentActions,
    latestAssistantMessage,
    plannerStatusSummary,
    composerScopeChips,
    modelOptions,
    plannerModelPickerOptions,
    plannerModelPickerValue,
    speechModelSelection,
    context,
    activeProductName,
  } = usePlannerPageViewModel({
    activeProductId,
    activeProductAreaId,
    activeCapabilityId,
    activeWorkItemId,
    selectedDraftNodeId,
    expandedDraftNodeIds,
    draftTreeNodes,
    messages,
    pendingPlan,
    voiceActivity,
    pendingVoiceTranscript,
    reviewVoiceBeforeSend,
    plannerView,
    windowWidth,
    providerId,
    modelName,
    speechProviderSetting,
    speechModelSetting,
  });
  speechModelSelectionRef.current = speechModelSelection;
  const {
    renameDraftName,
    setRenameDraftName,
    draftChildType,
    setDraftChildType,
    draftChildName,
    setDraftChildName,
    draftChildSummary,
    setDraftChildSummary,
    draftEditError,
    setDraftEditError,
    draftEditMessage,
    setDraftEditMessage,
  } = usePlannerDraftEditorState({
    selectedDraftNode,
    selectedDraftNodeId,
    allowedDraftChildTypes,
  });
  const {
    designPacketPath,
    designPacketError,
    isExportingDesignPacket,
    exportDesignReviewPacket,
  } = usePlannerDesignPacketExport({
    queryClient,
    selectedProductId,
    productTrees,
    activeProductName,
    products,
    workItems,
    plannerWorkItemsHasMore,
    draftTreeNodes,
    latestDraftPlan,
    draftValidation,
    selectedDraftNode,
    latestAssistantMessage,
    onAppendMessage: setMessages,
  });

  usePlannerPageLifecycle({
    selectedProductId,
    selectedProduct,
    sessionId,
    setSessionId,
    draftTreeNodes,
    setDraftTreeNodes,
    selectedDraftNodeId,
    setSelectedDraftNodeId,
    setExpandedDraftNodeIds,
    setLatestTraceEvents,
    pendingPlan,
    setPendingPlan,
    plannerView,
    setPlannerView,
    setMessages,
    providerId,
    setProviderId,
    providers,
    modelName,
    setModelName,
    modelOptions,
    transcriptRef,
    messages,
    location,
    navigate,
    setDraft,
    composerRef,
    consumedRoutePromptRef,
    activePlannerProductRef,
    isCompactScreen,
    setShowCompactTools,
  });

  const speakAssistantReply = async (text: string) => {
    const trimmed = text.trim();
    if (!trimmed) {
      return;
    }
    try {
      await speakTextNatively({
        text: trimmed,
        voice: speechNativeVoiceSetting || undefined,
        locale: speechLocaleSetting || "en-US",
      });
    } catch {
      speakInBrowser(trimmed);
    }
  };

  const handlePlannerMutationSuccess = usePlannerMutationResultHandler({
    queryClient,
    selectedProductId,
    draftTreeNodes,
    setPendingVoiceTranscript,
    setEditableVoiceTranscript,
    setVoiceActivity,
    setIsVoiceSubmitting,
    setLatestTraceEvents,
    setMessages,
    setDraftTreeNodes,
    setPlannerView,
    setSelectedDraftNodeId,
    setExpandedDraftNodeIds,
    setPendingPlan,
    autoSpeak,
    speakAssistantReply,
  });

  const {
    processMutation,
    draftEditMutation,
  } = usePlannerTurnMutations({
    selectedProductId,
    sessionId,
    setSessionId,
    providerId,
    modelName,
    selectedDraftNodeId,
    setPendingVoiceTranscript,
    setEditableVoiceTranscript,
    setVoiceActivity,
    setIsVoiceSubmitting,
    setLatestTraceEvents,
    setMessages,
    setDraftEditError,
    setDraftEditMessage,
    onPlannerMutationSuccess: handlePlannerMutationSuccess,
  });

  const basePlannerBusy =
    processMutation.isPending ||
    draftEditMutation.isPending ||
    isVoiceCaptureBusy;
  const {
    showRepoModal,
    setShowRepoModal,
    selectedRepositoryId,
    setSelectedRepositoryId,
    repositoryPathDraft,
    setRepositoryPathDraft,
    repoAnalysisMessage,
    repoAnalysisError,
    isRepositoryAnalysisPending,
    browseRepositoryPathForPlanner,
    registerRepositoryForPlanner,
    analyzeSelectedRepository,
  } = usePlannerRepositoryModalState({
    queryClient,
    repositories,
    sessionId,
    selectedDraftNodeId,
    selectedProductId,
    isPlannerBusy: basePlannerBusy,
    onAnalysisSuccess: handlePlannerMutationSuccess,
  });
  const isPlannerBusy = basePlannerBusy || isRepositoryAnalysisPending;
  plannerBusyRef.current = isPlannerBusy;

  const send = async () => {
    const content = draft.trim();
    if (!content || isPlannerBusy) {
      return;
    }
    if (!selectedProductId) {
      setMessages((current) => [
        ...current,
        { id: makeId(), role: "assistant", content: "Select a product before planning. Use Products to create one if needed.", meta: "Product required", kind: "error" },
      ]);
      return;
    }
    setDraft("");
    await processMutation.mutateAsync(content);
  };

  const submitVoiceTranscript = async (transcript: string) => {
    if (!transcript || isPlannerBusy) {
      return;
    }
    if (!selectedProductId) {
      setSpeechError("Select a product before using Planner voice input.");
      return;
    }
    setPendingVoiceTranscript(transcript);
    setVoiceActivity("Sending voice input to the planner...");
    setIsVoiceSubmitting(true);
    try {
      const handledAsVoiceCommand = await handleVoiceTranscript(transcript);
      if (!handledAsVoiceCommand) {
        setDraft((current) => (current ? `${current.trim()} ${transcript}` : transcript));
        composerRef.current?.focus();
        setVoiceActivity("Speech recognized and added to the composer.");
      }
    } catch (error) {
      setSpeechError(error instanceof Error ? error.message : String(error));
    } finally {
      setIsVoiceSubmitting(false);
      clearPendingVoiceReview();
    }
  };

  const {
    confirmPendingPlan,
    dismissPendingPlan,
  } = usePlannerPendingPlanActions({
    queryClient,
    pendingPlan,
    setPendingPlan,
    draftTreeNodes,
    setDraftTreeNodes,
    sessionId,
    isPlannerBusy,
    selectedProductId,
    setLatestTraceEvents,
    setMessages,
    setSelectedDraftNodeId,
    setPlannerView,
  });

  const {
    toggleDraftNodeExpanded,
    expandAllDraftNodes,
    collapseAllDraftNodes,
    applyPromptSuggestion,
    renameSelectedDraftNode,
    addChildToSelectedDraftNode,
    deleteSelectedDraftNode,
  } = usePlannerDraftActions({
    draftTreeNodes,
    setExpandedDraftNodeIds,
    setDraft,
    composerRef,
    selectedDraftNode,
    renameDraftName,
    draftChildType,
    draftChildName,
    setDraftChildName,
    draftChildSummary,
    setDraftChildSummary,
    allowedDraftChildTypes,
    isPlannerBusy,
    setDraftEditError,
    setDraftEditMessage,
    draftEditMutation,
  });

  const handleVoiceTranscript = usePlannerVoiceTranscriptHandler({
    autoSpeak,
    draftTreeNodes,
    latestTraceEvents,
    selectedProductId,
    sessionId,
    setSessionId,
    providerId,
    modelName,
    selectedDraftNodeId,
    selectedDraftNodePath,
    setPendingVoiceTranscript,
    setEditableVoiceTranscript,
    setVoiceActivity,
    setMessages,
    setPlannerView,
    setExpandedDraftNodeIds,
    expandAllDraftNodes,
    collapseAllDraftNodes,
    onPlannerMutationSuccess: handlePlannerMutationSuccess,
    speakAssistantReply,
  });
  submitVoiceTranscriptRef.current = submitVoiceTranscript;

  useEffect(() => {
    if (typeof window === "undefined" || !window.__ARUVI_E2E__) {
      return;
    }
    window.__ARUVI_E2E__.runPlannerVoiceTranscript = async (transcript: string) => {
      const handled = await handleVoiceTranscript(transcript);
      if (!handled) {
        setDraft((current) => (current ? `${current.trim()} ${transcript.trim()}` : transcript.trim()));
      }
    };
    return () => {
      if (window.__ARUVI_E2E__) {
        delete window.__ARUVI_E2E__.runPlannerVoiceTranscript;
      }
    };
  }, [handleVoiceTranscript]);

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
      plannerWorkItemsHasMore={plannerWorkItemsHasMore}
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
              transcriptRef={transcriptRef}
              pendingVoiceTranscript={pendingVoiceTranscript}
              reviewVoiceBeforeSend={reviewVoiceBeforeSend}
              voiceElapsedMs={voiceElapsedMs}
              isVoiceSubmitting={isVoiceSubmitting}
              editableVoiceTranscript={editableVoiceTranscript}
              isPlannerBusy={isPlannerBusy}
              messages={messages}
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
