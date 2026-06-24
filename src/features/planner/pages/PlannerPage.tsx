import React, { useEffect, useRef, useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useLocation, useNavigate } from "react-router-dom";
import {
  addPlannerDraftChild,
  clearPlannerPending,
  confirmPlannerPlan,
  createPlannerSession,
  deletePlannerDraftNode,
  renamePlannerDraftNode,
  revealInFinder,
  speakTextNatively,
  submitPlannerTurn,
  submitPlannerVoiceTurn,
  transcribeAudio,
} from "../../../lib/tauri";
import { blobToBase64, speakInBrowser, startWavCapture, type ActiveAudioCapture } from "../../shared/voice";
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
import { usePlannerRepositoryModalState } from "../hooks/usePlannerRepositoryModalState";
import { usePlannerSpeechSettingsState } from "../hooks/usePlannerSpeechSettingsState";
import { usePlannerWindowWidth } from "../hooks/usePlannerWindowWidth";
import { styles } from "../lib/plannerPageStyles";
import {
  DEFAULT_ASSISTANT_OPENING,
  buildPlannerMutationMessages,
  buildWorkItemTreeNodes,
  buildWorkItemTreeReport,
  executePlannerPlan,
  findCapability,
  findProductArea,
  findTree,
  findTreeNodePath,
  flattenTreeNodes,
  formatDraftChildTypeLabel,
  getPlannerMutationSpeechText,
  getPlannerNodeType,
  getPlannerVoiceViewCommand,
  getReportTreeProductName,
  isCollapseDraftVoiceCommand,
  isDraftWideVoiceTarget,
  isExpandDraftVoiceCommand,
  makeId,
  normalize,
  parseVoiceNodeReference,
  resolveVoiceNodeReference,
  type DraftEditOperation,
  type ExecutionResult,
  type PendingPlan,
  type PlannerAction,
  type PlannerMessage,
  type PlannerMutationResult,
  type PlannerPlan,
  type PlannerTreeNode,
  mapPlannerResponseToMutationResult,
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
  const [isListening, setIsListening] = useState(false);
  const [isTranscribing, setIsTranscribing] = useState(false);
  const [isVoiceSubmitting, setIsVoiceSubmitting] = useState(false);
  const [pendingVoiceTranscript, setPendingVoiceTranscript] = useState<string | null>(null);
  const [editableVoiceTranscript, setEditableVoiceTranscript] = useState("");
  const [voiceActivity, setVoiceActivity] = useState<string | null>(null);
  const [voiceCaptureStartedAt, setVoiceCaptureStartedAt] = useState<number | null>(null);
  const [voiceElapsedMs, setVoiceElapsedMs] = useState<number>(0);
  const [speechError, setSpeechError] = useState<string | null>(null);
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
  const audioCaptureRef = useRef<ActiveAudioCapture | null>(null);
  const mediaStreamRef = useRef<MediaStream | null>(null);
  const transcriptRef = useRef<HTMLDivElement | null>(null);
  const composerRef = useRef<HTMLTextAreaElement | null>(null);
  const consumedRoutePromptRef = useRef<string | null>(null);
  const activePlannerProductRef = useRef<string | null>(null);

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
  }, [draftTreeNodes, handleVoiceTranscript, selectedDraftNodeId, latestTraceEvents, pendingPlan, autoSpeak]);

  useEffect(() => {
    if (!voiceEnabled) {
      void stopVoiceCapture(false);
      setIsListening(false);
      return;
    }
    return () => {
      void stopVoiceCapture(false);
    };
  }, [voiceEnabled]);

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

  const handlePlannerMutationSuccess = (result: PlannerMutationResult) => {
    setPendingVoiceTranscript(null);
    setEditableVoiceTranscript("");
    setVoiceActivity(null);
    setIsVoiceSubmitting(false);
    setLatestTraceEvents(result.traceEvents ?? []);
    setMessages((current) => buildPlannerMutationMessages(current, result, makeId));

    if (result.draftTreeNodes) {
      setDraftTreeNodes(result.draftTreeNodes);
      if (result.draftTreeNodes.length > 0) {
        setPlannerView("draft");
      }
    }
    if (result.selectedDraftNodeId !== undefined) {
      setSelectedDraftNodeId(result.selectedDraftNodeId ?? null);
      const treeForPath = result.draftTreeNodes ?? draftTreeNodes;
      if (result.selectedDraftNodeId && treeForPath.length > 0) {
        const pathIds = findTreeNodePath(treeForPath, result.selectedDraftNodeId).map((node) => node.id);
        setExpandedDraftNodeIds((current) => Array.from(new Set([...current, ...pathIds])));
      }
    }

    if (result.mode === "confirmation_required") {
      setPendingPlan({ sourceText: result.userInput, plan: result.plan });
    } else if (result.mode === "draft_updated") {
      setPendingPlan(null);
    } else if (result.mode === "session_updated") {
      // Preserve the currently staged plan while updating draft selection or voice-driven session state.
    } else if (result.mode === "failed") {
      setPendingPlan(null);
      setPlannerView("trace");
    } else {
      setPendingPlan(null);
      if (result.mode === "executed" && !result.draftTreeNodes?.length) {
        setDraftTreeNodes([]);
        setSelectedDraftNodeId(null);
      }
      void queryClient.invalidateQueries({ queryKey: ["products"] });
      void queryClient.invalidateQueries({ queryKey: ["plannerWorkItems", selectedProductId] });
      void queryClient.invalidateQueries({ queryKey: ["sidebarWorkItems"] });
      void queryClient.invalidateQueries({ queryKey: ["productTree", selectedProductId] });
      void queryClient.invalidateQueries({ queryKey: ["plannerProductAreas", selectedProductId] });
      void queryClient.invalidateQueries({ queryKey: ["plannerProductTree", selectedProductId] });
    }

    if (autoSpeak) {
      void speakAssistantReply(getPlannerMutationSpeechText(result));
    }
  };

  const processMutation = useMutation<PlannerMutationResult, Error, string>({
    mutationFn: async (input: string) => {
      const userInput = input.trim();
      if (!selectedProductId) {
        throw new Error("Select a product before planning.");
      }
      let activeSessionId = sessionId;
      if (!activeSessionId) {
        const session = await createPlannerSession({
          providerId: providerId || undefined,
          modelName: modelName || undefined,
        });
        activeSessionId = session.session_id;
        setSessionId(session.session_id);
      }

      const response = await submitPlannerTurn({
        sessionId: activeSessionId,
        userInput,
        selectedDraftNodeId,
        productId: selectedProductId,
      });

      return mapPlannerResponseToMutationResult(response, userInput);
    },
    onSuccess: handlePlannerMutationSuccess,
    onError: (error, userInput) => {
      setPendingVoiceTranscript(null);
      setEditableVoiceTranscript("");
      setVoiceActivity(null);
      setIsVoiceSubmitting(false);
      setLatestTraceEvents([]);
      setMessages((current) => [
        ...current,
        { id: makeId(), role: "user", content: userInput, kind: "text" },
        { id: makeId(), role: "assistant", content: error instanceof Error ? error.message : String(error), meta: "Planner error", kind: "error" },
      ]);
    },
  });

  const draftEditMutation = useMutation<PlannerMutationResult, Error, DraftEditOperation>({
    mutationFn: async (operation) => {
      if (!sessionId) {
        throw new Error("Planner session is not ready.");
      }
      switch (operation.kind) {
        case "rename": {
          const response = await renamePlannerDraftNode({
            sessionId,
            nodeId: operation.nodeId,
            name: operation.name,
          });
          return mapPlannerResponseToMutationResult(
            response,
            `Rename this node to "${operation.name}".`,
          );
        }
        case "add_child": {
          const response = await addPlannerDraftChild({
            sessionId,
            parentNodeId: operation.parentNodeId,
            childType: operation.childType,
            name: operation.name,
            summary: operation.summary,
          });
          return mapPlannerResponseToMutationResult(
            response,
            `Add a ${formatDraftChildTypeLabel(operation.childType).toLowerCase()} called "${operation.name}".`,
          );
        }
        case "delete": {
          const response = await deletePlannerDraftNode({
            sessionId,
            nodeId: operation.nodeId,
          });
          return mapPlannerResponseToMutationResult(
            response,
            "Delete this node from the staged design.",
          );
        }
      }
    },
    onSuccess: handlePlannerMutationSuccess,
    onError: (error) => {
      setDraftEditError(error instanceof Error ? error.message : String(error));
      setDraftEditMessage(null);
    },
  });

  const transcribeAudioMutation = useMutation<string, Error, { audioBytesBase64: string; mimeType: string }>({
    mutationFn: async ({ audioBytesBase64, mimeType }) => {
      if (!speechModelSelection) {
        throw new Error("Configure a speech transcription provider or model before using voice input.");
      }
      const response = await transcribeAudio({
        providerId: speechModelSelection.providerId,
        modelName: speechModelSelection.modelName,
        audioBytesBase64,
        mimeType,
        locale: speechLocaleSetting || "en-US",
      });
      return response.transcript;
    },
    onError: (error) => {
      setSpeechError(error instanceof Error ? error.message : String(error));
    },
  });

  const basePlannerBusy =
    processMutation.isPending ||
    draftEditMutation.isPending ||
    transcribeAudioMutation.isPending ||
    isVoiceSubmitting;
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

  useEffect(() => {
    if (!isListening || !voiceCaptureStartedAt) {
      return undefined;
    }
    const interval = window.setInterval(() => {
      setVoiceElapsedMs(Date.now() - voiceCaptureStartedAt);
    }, 250);
    return () => window.clearInterval(interval);
  }, [isListening, voiceCaptureStartedAt]);

  const stopVoiceCapture = async (shouldTranscribe: boolean) => {
    const capture = audioCaptureRef.current;
    if (!capture) {
      return;
    }

    audioCaptureRef.current = null;
    mediaStreamRef.current = null;
    setIsListening(false);
    setVoiceCaptureStartedAt(null);

    try {
      const blob = await capture.stop();
      if (!shouldTranscribe || blob.size === 0) {
        setVoiceActivity(null);
        return;
      }

      setVoiceActivity("Transcribing audio...");
      setIsTranscribing(true);
      const audioBytesBase64 = await blobToBase64(blob);
      const transcript = await transcribeAudioMutation.mutateAsync({
        audioBytesBase64,
        mimeType: blob.type || "audio/wav",
      });
      const trimmedTranscript = transcript.trim();
      setIsTranscribing(false);
      if (!trimmedTranscript) {
        setVoiceActivity("No speech detected.");
        return;
      }
      if (reviewVoiceBeforeSend) {
        setPendingVoiceTranscript(trimmedTranscript);
        setEditableVoiceTranscript(trimmedTranscript);
        setVoiceActivity("Speech recognized. Review or edit before sending.");
        return;
      }
      setEditableVoiceTranscript(trimmedTranscript);
      setVoiceActivity("Speech recognized. Sending it to the planner...");
      await submitVoiceTranscript(trimmedTranscript);
    } catch (error) {
      if (shouldTranscribe) {
        setSpeechError(error instanceof Error ? error.message : String(error));
      }
      setIsTranscribing(false);
      setIsVoiceSubmitting(false);
      setVoiceActivity(null);
      setPendingVoiceTranscript(null);
      setEditableVoiceTranscript("");
    } finally {
      setIsTranscribing(false);
    }
  };

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

  const clearPendingVoiceReview = () => {
    setPendingVoiceTranscript(null);
    setEditableVoiceTranscript("");
    setVoiceActivity(null);
    setVoiceElapsedMs(0);
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

  const submitPendingVoiceTranscript = async () => {
    const transcript = editableVoiceTranscript.trim();
    if (!transcript || isPlannerBusy) {
      return;
    }
    await submitVoiceTranscript(transcript);
  };

  const retryVoiceCapture = async () => {
    clearPendingVoiceReview();
    await toggleListening();
  };

  const toggleListening = async () => {
    if (!voiceEnabled) {
      setSpeechError("Voice input is disabled.");
      return;
    }
    if (isListening) {
      await stopVoiceCapture(true);
      return;
    }

    if (typeof navigator === "undefined" || !navigator.mediaDevices?.getUserMedia) {
      setSpeechError("Microphone access is not available in this runtime.");
      return;
    }
    if (typeof window === "undefined" || (!window.AudioContext && !(window as typeof window & { webkitAudioContext?: typeof AudioContext }).webkitAudioContext)) {
      setSpeechError("PCM audio capture is not supported in this runtime.");
      return;
    }

    try {
      setSpeechError(null);
      setVoiceActivity("Listening...");
      setVoiceElapsedMs(0);
      setVoiceCaptureStartedAt(Date.now());
      setPendingVoiceTranscript(null);
      setEditableVoiceTranscript("");
      const capture = await startWavCapture();
      audioCaptureRef.current = capture;
      mediaStreamRef.current = capture.stream;
      setIsListening(true);
    } catch (error) {
      setSpeechError(error instanceof Error ? error.message : String(error));
      setIsListening(false);
      setVoiceActivity(null);
      setVoiceCaptureStartedAt(null);
      mediaStreamRef.current?.getTracks().forEach((track) => track.stop());
      mediaStreamRef.current = null;
      audioCaptureRef.current = null;
    }
  };

  const confirmPendingPlan = () => {
    if ((!pendingPlan && draftTreeNodes.length === 0) || isPlannerBusy || !sessionId) {
      return;
    }
    void (async () => {
      const response = await confirmPlannerPlan(sessionId);
      const execution: ExecutionResult = {
        lines: response.execution_lines,
        errors: response.execution_errors,
      };
      const plan = pendingPlan?.plan ?? {
        assistant_response: "Applied design to catalog.",
        needs_confirmation: false,
        clarification_question: null,
        actions: [],
      };
      const treeNodes = (response.tree_nodes as unknown as PlannerTreeNode[] | undefined) ?? undefined;
      setLatestTraceEvents(response.trace_events ?? []);
      setMessages((current) => [
        ...current,
        { id: makeId(), role: "user", content: "confirm", kind: "text" },
        {
          id: makeId(),
          role: "assistant",
          content: ["Applied design to catalog.", ...execution.lines, ...(execution.errors.length ? [`Errors: ${execution.errors.join(" | ")}`] : [])].join("\n"),
          meta: "Planner execution",
          kind: treeNodes ? "tree" : "execution",
          treeNodes,
          plan,
          traceEvents: response.trace_events ?? [],
        },
      ]);
      setPendingPlan(null);
      setDraftTreeNodes([]);
      setSelectedDraftNodeId(null);
      setPlannerView("conversation");
      void queryClient.invalidateQueries({ queryKey: ["products"] });
      void queryClient.invalidateQueries({ queryKey: ["plannerWorkItems", selectedProductId] });
      void queryClient.invalidateQueries({ queryKey: ["sidebarWorkItems"] });
      void queryClient.invalidateQueries({ queryKey: ["productTree", selectedProductId] });
      void queryClient.invalidateQueries({ queryKey: ["plannerProductAreas", selectedProductId] });
      void queryClient.invalidateQueries({ queryKey: ["plannerProductTree", selectedProductId] });
    })().catch((error) => {
      setMessages((current) => [
        ...current,
        { id: makeId(), role: "assistant", content: String(error), meta: "Planner error", kind: "error" },
      ]);
    });
  };

  const dismissPendingPlan = () => {
    if (!pendingPlan && draftTreeNodes.length === 0) {
      return;
    }
    if (sessionId) {
      void clearPlannerPending(sessionId).catch(() => {});
    }
    setPendingPlan(null);
    setDraftTreeNodes([]);
    setSelectedDraftNodeId(null);
    setPlannerView("conversation");
  };

  function appendVoiceCommandFeedback(transcript: string, reply: string) {
    setPendingVoiceTranscript(null);
    setEditableVoiceTranscript("");
    setVoiceActivity(null);
    setMessages((current) => [
      ...current,
      { id: makeId(), role: "user", content: transcript, kind: "text" },
      { id: makeId(), role: "assistant", content: reply, meta: "Voice command", kind: "text" },
    ]);
    if (autoSpeak) {
      void speakAssistantReply(reply);
    }
  }

  async function handleVoiceTranscript(transcript: string) {
    const spoken = transcript.trim();
    if (!spoken) {
      return true;
    }
    const normalizedTranscript = normalize(spoken);

    const viewCommand = getPlannerVoiceViewCommand(normalizedTranscript);

    if (viewCommand === "draft") {
      if (draftTreeNodes.length === 0) {
        appendVoiceCommandFeedback(spoken, "There is no staged design tree yet.");
      } else {
        setPlannerView("draft");
        appendVoiceCommandFeedback(spoken, "Opened the design review.");
      }
      return true;
    }

    if (viewCommand === "trace") {
      if (latestTraceEvents.length === 0) {
        appendVoiceCommandFeedback(spoken, "There is no planner trace available yet.");
      } else {
        setPlannerView("trace");
        appendVoiceCommandFeedback(spoken, "Opened the latest planner trace.");
      }
      return true;
    }

    if (viewCommand === "conversation") {
      setPlannerView("conversation");
      appendVoiceCommandFeedback(spoken, "Switched back to the planner conversation.");
      return true;
    }

    if (!selectedProductId) {
      appendVoiceCommandFeedback(spoken, "Select a product before planning. Create products in the Products page, then return here to design.");
      return true;
    }

    if (isExpandDraftVoiceCommand(normalizedTranscript)) {
      setPlannerView("draft");
      expandAllDraftNodes();
      appendVoiceCommandFeedback(spoken, "Expanded the staged design tree.");
      return true;
    }

    if (isCollapseDraftVoiceCommand(normalizedTranscript)) {
      collapseAllDraftNodes();
      appendVoiceCommandFeedback(spoken, "Collapsed the staged design tree.");
      return true;
    }

    const collapseMatch = normalizedTranscript.match(/^(collapse|close)\s+(.+)$/);
    if (normalizedTranscript.startsWith("expand ") || normalizedTranscript.startsWith("open ")) {
      const targetText = spoken.replace(/^(expand|open)\s+/i, "").trim();
      if (isDraftWideVoiceTarget(targetText)) {
        setPlannerView("draft");
        expandAllDraftNodes();
        appendVoiceCommandFeedback(spoken, "Expanded the staged design tree.");
        return true;
      }
    }

    if (collapseMatch) {
      const targetText = collapseMatch[2];
      if (isDraftWideVoiceTarget(targetText)) {
        collapseAllDraftNodes();
        appendVoiceCommandFeedback(spoken, "Collapsed the staged design tree.");
        return true;
      }
      const { explicitType, reference } = parseVoiceNodeReference(targetText);
      const targetNode = resolveVoiceNodeReference(draftTreeNodes, selectedDraftNodePath, reference, explicitType);
      if (!targetNode) {
        appendVoiceCommandFeedback(spoken, `I could not find a design node matching "${targetText}".`);
        return true;
      }
      setExpandedDraftNodeIds((current) => current.filter((nodeId) => nodeId !== targetNode.id));
      appendVoiceCommandFeedback(spoken, `Collapsed ${getPlannerNodeType(targetNode)} "${targetNode.label}".`);
      return true;
    }

    let activeSessionId = sessionId;
    if (!activeSessionId) {
      const session = await createPlannerSession({
        providerId: providerId || undefined,
        modelName: modelName || undefined,
      });
      activeSessionId = session.session_id;
      setSessionId(session.session_id);
    }

    const response = await submitPlannerVoiceTurn({
      sessionId: activeSessionId,
      transcript: spoken,
      selectedDraftNodeId,
      productId: selectedProductId,
    });
    handlePlannerMutationSuccess(mapPlannerResponseToMutationResult(response, spoken));
    return true;
  }

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
