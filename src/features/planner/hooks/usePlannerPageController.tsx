import { useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useLocation, useNavigate } from "react-router-dom";
import { revealInFinder } from "../../../lib/tauri";
import { useWorkspaceStore } from "../../../state/workspaceStore";
import { usePlannerAssistantSpeech } from "./usePlannerAssistantSpeech";
import { usePlannerDesignPacketExport } from "./usePlannerDesignPacketExport";
import { usePlannerDraftActions } from "./usePlannerDraftActions";
import { usePlannerDraftEditorState } from "./usePlannerDraftEditorState";
import { usePlannerPageLifecycle } from "./usePlannerPageLifecycle";
import { usePlannerPageViewModel } from "./usePlannerPageViewModel";
import { usePlannerPageCoreState } from "./usePlannerPageCoreState";
import { usePlannerMutationResultHandler } from "./usePlannerMutationResultHandler";
import { usePlannerPendingPlanActions } from "./usePlannerPendingPlanActions";
import { usePlannerRepositoryModalState } from "./usePlannerRepositoryModalState";
import { usePlannerSpeechSettingsState } from "./usePlannerSpeechSettingsState";
import { usePlannerTurnMutations } from "./usePlannerTurnMutations";
import { usePlannerVoiceCapture } from "./usePlannerVoiceCapture";
import { usePlannerVoiceTranscriptHandler } from "./usePlannerVoiceTranscriptHandler";
import { usePlannerWindowWidth } from "./usePlannerWindowWidth";
import { makeId } from "../lib/plannerPageModel";

export function usePlannerPageController() {
  const queryClient = useQueryClient();
  const location = useLocation();
  const navigate = useNavigate();
  const { activeProductId, activeProductAreaId, activeCapabilityId, activeWorkItemId, setActiveProduct } = useWorkspaceStore();
  const {
    activePlannerProductRef,
    composerRef,
    consumedRoutePromptRef,
    draft,
    draftTreeNodes,
    expandedDraftNodeIds,
    latestTraceEvents,
    messages,
    modelName,
    pendingPlan,
    plannerBusyRef,
    plannerView,
    providerId,
    selectedDraftNodeId,
    sessionId,
    setDraft,
    setDraftTreeNodes,
    setExpandedDraftNodeIds,
    setLatestTraceEvents,
    setMessages,
    setModelName,
    setPendingPlan,
    setPlannerView,
    setProviderId,
    setSelectedDraftNodeId,
    setSessionId,
    setShowCompactTools,
    showCompactTools,
    speechModelSelectionRef,
    submitVoiceTranscriptRef,
    transcriptRef,
  } = usePlannerPageCoreState();
  const {
    voiceEnabled,
    autoSpeak,
    speechProviderSetting,
    speechModelSetting,
    speechLocaleSetting,
    speechNativeVoiceSetting,
    reviewVoiceBeforeSend,
  } = usePlannerSpeechSettingsState();
  const { speakAssistantReply } = usePlannerAssistantSpeech({
    speechLocaleSetting,
    speechNativeVoiceSetting,
  });
  const windowWidth = usePlannerWindowWidth();
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

  return {
    allowedDraftChildTypes,
    analyzeSelectedRepository,
    browseRepositoryPathForPlanner,
    clearPendingVoiceReview,
    collapseAllDraftNodes,
    composerRef,
    composerScopeChips,
    confirmPendingPlan,
    context,
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
    messages,
    modelName,
    navigate,
    pendingPlan,
    pendingVoiceTranscript,
    plannerModelPickerOptions,
    plannerModelPickerValue,
    plannerStatusSummary,
    plannerView,
    plannerWorkItemsHasMore,
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
    setEditableVoiceTranscript,
    setModelName,
    setPlannerView,
    setProviderId,
    setRenameDraftName,
    setRepositoryPathDraft,
    setSelectedDraftNodeId,
    setSelectedRepositoryId,
    setShowCompactTools,
    setDraft,
    showCompactTools,
    showRepoModal,
    setShowRepoModal,
    submitPendingVoiceTranscript,
    toggleListening,
    toggleDraftNodeExpanded,
    transcriptRef,
    voiceElapsedMs,
    voiceEnabled,
    voiceActivity,
    applyPromptSuggestion,
    renameDraftName,
    renameSelectedDraftNode,
    addChildToSelectedDraftNode,
  };
}

export type PlannerPageController = ReturnType<typeof usePlannerPageController>;
