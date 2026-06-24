import React, { useEffect, useMemo, useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useLocation, useNavigate } from "react-router-dom";
import {
  addPlannerDraftChild,
  clearPlannerPending,
  confirmPlannerPlan,
  createPlannerSession,
  deletePlannerDraftNode,
  exportProductOverviewHtml,
  getSetting,
  getProductTree,
  listRepositories,
  listModelDefinitions,
  listProducts,
  listProductAreas,
  listProviders,
  listWorkItemsPage,
  browseForRepositoryPath,
  registerRepository,
  analyzeRepositoryForPlanner,
  renamePlannerDraftNode,
  revealInFinder,
  speakTextNatively,
  submitPlannerTurn,
  submitPlannerVoiceTurn,
  transcribeAudio,
  updatePlannerSession,
} from "../../../lib/tauri";
import { blobToBase64, speakInBrowser, startWavCapture, type ActiveAudioCapture } from "../../shared/voice";
import { styles } from "../lib/plannerPageStyles";
import {
  DEFAULT_ASSISTANT_OPENING,
  PLANNER_WORK_ITEM_PAGE_SIZE,
  SPEECH_AUTO_SPEAK_REPLIES_KEY,
  SPEECH_ENABLE_MIC_KEY,
  SPEECH_LOCALE_KEY,
  SPEECH_MODEL_KEY,
  SPEECH_NATIVE_VOICE_KEY,
  SPEECH_PROVIDER_KEY,
  SPEECH_REVIEW_BEFORE_SEND_KEY,
  FormattedPlannerText,
  PlannerComposer,
  SelectableTreeNodeView,
  TreeNodeView,
  buildDesignReviewPacketHtml,
  buildDraftValidation,
  buildProductAreaOnlyTree,
  buildProposalTreeNodes,
  buildSuggestedPrompts,
  buildWorkItemTreeNodes,
  buildWorkItemTreeReport,
  collectTreeNodeIds,
  executePlannerPlan,
  findCapability,
  findProductArea,
  findRelevantPlanActions,
  findTree,
  findTreeNodeById,
  findTreeNodePath,
  flattenTreeNodes,
  formatDraftChildTypeLabel,
  formatElapsedMs,
  getAllowedDraftChildTypes,
  getPlannerNodeType,
  getReportTreeProductName,
  isInformationalOnly,
  makeId,
  normalize,
  resolveVoiceNodeReference,
  slugifyPacketName,
  summarizeAction,
  type DraftEditOperation,
  type DraftValidationSummary,
  type ExecutionResult,
  type PendingPlan,
  type PlannerAction,
  type PlannerMessage,
  type PlannerMutationResult,
  type PlannerPlan,
  type PlannerTreeNode,
  type ResolverContext,
} from "../lib/plannerPageModel";
import { useWorkspaceStore } from "../../../state/workspaceStore";
import type {
  ModelDefinition,
  PlannerDraftChildType,
  PlannerTraceEvent,
  PlannerTurnResponse,
  Product,
  ProductArea,
  ProductTree,
  WorkItem,
} from "../../../lib/types";


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
  const [renameDraftName, setRenameDraftName] = useState("");
  const [draftChildType, setDraftChildType] = useState<PlannerDraftChildType>("product_area");
  const [draftChildName, setDraftChildName] = useState("");
  const [draftChildSummary, setDraftChildSummary] = useState("");
  const [draftEditError, setDraftEditError] = useState<string | null>(null);
  const [draftEditMessage, setDraftEditMessage] = useState<string | null>(null);
  const [selectedRepositoryId, setSelectedRepositoryId] = useState("");
  const [repositoryPathDraft, setRepositoryPathDraft] = useState("");
  const [repoAnalysisMessage, setRepoAnalysisMessage] = useState<string | null>(null);
  const [repoAnalysisError, setRepoAnalysisError] = useState<string | null>(null);
  const [designPacketPath, setDesignPacketPath] = useState<string | null>(null);
  const [designPacketError, setDesignPacketError] = useState<string | null>(null);
  const [isExportingDesignPacket, setIsExportingDesignPacket] = useState(false);
  const [voiceEnabled, setVoiceEnabled] = useState(true);
  const [autoSpeak, setAutoSpeak] = useState(false);
  const [isListening, setIsListening] = useState(false);
  const [isTranscribing, setIsTranscribing] = useState(false);
  const [isVoiceSubmitting, setIsVoiceSubmitting] = useState(false);
  const [pendingVoiceTranscript, setPendingVoiceTranscript] = useState<string | null>(null);
  const [editableVoiceTranscript, setEditableVoiceTranscript] = useState("");
  const [voiceActivity, setVoiceActivity] = useState<string | null>(null);
  const [voiceCaptureStartedAt, setVoiceCaptureStartedAt] = useState<number | null>(null);
  const [voiceElapsedMs, setVoiceElapsedMs] = useState<number>(0);
  const [speechError, setSpeechError] = useState<string | null>(null);
  const [speechProviderSetting, setSpeechProviderSetting] = useState("");
  const [speechModelSetting, setSpeechModelSetting] = useState("");
  const [speechLocaleSetting, setSpeechLocaleSetting] = useState("en-US");
  const [speechNativeVoiceSetting, setSpeechNativeVoiceSetting] = useState("");
  const [reviewVoiceBeforeSend, setReviewVoiceBeforeSend] = useState(false);
  const [showRepoModal, setShowRepoModal] = useState(false);
  const [windowWidth, setWindowWidth] = useState<number>(() => (typeof window === "undefined" ? 1440 : window.innerWidth));
  const [showCompactTools, setShowCompactTools] = useState(false);
  const audioCaptureRef = useRef<ActiveAudioCapture | null>(null);
  const mediaStreamRef = useRef<MediaStream | null>(null);
  const transcriptRef = useRef<HTMLDivElement | null>(null);
  const composerRef = useRef<HTMLTextAreaElement | null>(null);
  const consumedRoutePromptRef = useRef<string | null>(null);
  const activePlannerProductRef = useRef<string | null>(null);

  const { data: products = [] } = useQuery({ queryKey: ["products"], queryFn: listProducts });
  const { data: providers = [] } = useQuery({ queryKey: ["plannerProviders"], queryFn: listProviders });
  const { data: models = [] } = useQuery({ queryKey: ["plannerModels"], queryFn: listModelDefinitions });
  const { data: repositories = [] } = useQuery({ queryKey: ["plannerRepositories"], queryFn: listRepositories });
  const selectedProductId = useMemo(
    () => products.some((product) => product.id === activeProductId) ? activeProductId : null,
    [activeProductId, products],
  );
  const selectedProduct = useMemo(
    () => products.find((product) => product.id === selectedProductId) ?? null,
    [products, selectedProductId],
  );
  const { data: selectedProductAreas = [] } = useQuery<ProductArea[]>({
    queryKey: ["plannerProductAreas", selectedProductId],
    queryFn: () => listProductAreas(selectedProductId!),
    enabled: !!selectedProductId,
  });
  const { data: workItemPage } = useQuery({
    queryKey: ["plannerWorkItems", selectedProductId, PLANNER_WORK_ITEM_PAGE_SIZE],
    queryFn: () => listWorkItemsPage({
      productId: selectedProductId ?? undefined,
      limit: PLANNER_WORK_ITEM_PAGE_SIZE,
      offset: 0,
    }),
    enabled: !!selectedProductId,
  });
  const workItems = workItemPage?.items ?? [];
  const plannerWorkItemsHasMore = workItemPage?.has_more ?? false;
  const productTrees = useMemo(() => {
    if (selectedProduct && selectedProductAreas.length > 0) {
      return [buildProductAreaOnlyTree(selectedProduct, selectedProductAreas)];
    }
    return [];
  }, [selectedProduct, selectedProductAreas]);
  const hasTreeData = productTrees.length > 0;
  const isFocusedWorkspaceView = plannerView === "draft" || plannerView === "trace";
  const isCompactScreen = windowWidth <= 1360;
  const selectedDraftNode = useMemo(
    () => findTreeNodeById(draftTreeNodes, selectedDraftNodeId),
    [draftTreeNodes, selectedDraftNodeId],
  );
  const selectedDraftNodePath = useMemo(
    () => findTreeNodePath(draftTreeNodes, selectedDraftNodeId),
    [draftTreeNodes, selectedDraftNodeId],
  );
  const expandedDraftNodeIdSet = useMemo(
    () => new Set(expandedDraftNodeIds),
    [expandedDraftNodeIds],
  );
  const latestDraftPlan = useMemo(() => {
    if (pendingPlan?.plan) {
      return pendingPlan.plan;
    }
    for (let index = messages.length - 1; index >= 0; index -= 1) {
      const entry = messages[index];
      if (entry.role === "assistant" && entry.plan && entry.plan.actions.length > 0) {
        return entry.plan;
      }
    }
    return null;
  }, [messages, pendingPlan]);
  const selectedDraftNodePrompts = useMemo(
    () => buildSuggestedPrompts(selectedDraftNode),
    [selectedDraftNode],
  );
  const allowedDraftChildTypes = useMemo(
    () => getAllowedDraftChildTypes(selectedDraftNode),
    [selectedDraftNode],
  );
  const draftValidation = useMemo(
    () => buildDraftValidation(draftTreeNodes),
    [draftTreeNodes],
  );
  const selectedNodeRecentActions = useMemo(
    () => findRelevantPlanActions(latestDraftPlan, selectedDraftNode),
    [latestDraftPlan, selectedDraftNode],
  );
  const latestAssistantMessage = useMemo(() => {
    for (let index = messages.length - 1; index >= 0; index -= 1) {
      if (messages[index].role === "assistant") {
        return messages[index];
      }
    }
    return null;
  }, [messages]);
  const plannerStatusSummary = useMemo(() => {
    if (voiceActivity) {
      return {
        title: voiceActivity,
        detail: pendingVoiceTranscript
          ? reviewVoiceBeforeSend
            ? "The transcript is ready for review before it becomes a planner turn."
            : "The transcript is being sent to the planner."
          : "Voice capture is in progress.",
      };
    }
    if (pendingVoiceTranscript && reviewVoiceBeforeSend) {
      return {
        title: "Voice transcript ready",
        detail: "Review or edit the transcript, then send it to the planner.",
      };
    }
    if (draftTreeNodes.length > 0) {
      return {
        title: `Design active: ${draftValidation.counts.product} product, ${draftValidation.counts["product area"]} product area, ${draftValidation.counts.capability} capability/feature, ${draftValidation.counts["work item"]} story/task`,
        detail: selectedDraftNode
          ? `Selected node: ${selectedDraftNode.label}.`
          : "Select a node and keep refining before apply.",
      };
    }
    if (pendingPlan) {
      return {
        title: "Proposal waiting for confirmation",
        detail: `${pendingPlan.plan.actions.length} proposed changes are ready for review.`,
      };
    }
    if (latestAssistantMessage) {
      return {
        title: latestAssistantMessage.meta ?? "Planner ready",
        detail: latestAssistantMessage.content.split("\n")[0] || "Describe the product area, capability, feature, story, or task you want.",
      };
    }
    return {
      title: "Planner ready",
      detail: "Describe the product area, capability, feature, story, or task you want to stage.",
    };
  }, [
    draftTreeNodes.length,
    draftValidation.counts,
    latestAssistantMessage,
    pendingPlan,
    pendingVoiceTranscript,
    reviewVoiceBeforeSend,
    selectedDraftNode,
    voiceActivity,
  ]);
  const composerScopeChips = useMemo(() => {
    const chips: string[] = [];
    if (selectedDraftNodeId) {
      chips.push("design node selected");
    }
    if (selectedProductId) {
      chips.push("product selected");
    }
    if (activeProductAreaId) {
      chips.push("product area selected");
    }
    if (activeCapabilityId) {
      chips.push("capability selected");
    }
    if (activeWorkItemId) {
      chips.push("story/task selected");
    }
    return chips;
  }, [activeCapabilityId, activeProductAreaId, selectedProductId, activeWorkItemId, selectedDraftNodeId]);
  const composerScopeHint =
    "If you omit names, the planner first tries the selected design node, then the selected workspace scope, then asks follow-up questions if it still cannot resolve the target cleanly.";

  const modelOptions = useMemo(
    () => models.filter((model) => model.provider_id === providerId && model.enabled),
    [models, providerId],
  );
  const plannerModelPickerOptions = useMemo(
    () =>
      models
        .filter((model) => model.enabled)
        .map((model) => {
          const provider = providers.find((entry) => entry.id === model.provider_id);
          return {
            value: `${model.provider_id}::${model.name}`,
            label: `${provider?.name ?? "Unknown Provider"} / ${model.name}`,
          };
        }),
    [models, providers],
  );
  const plannerModelPickerValue = providerId && modelName ? `${providerId}::${modelName}` : "";
  const speechModelSelection = useMemo(() => {
    const looksLikeSpeechModel = (model: ModelDefinition) =>
      model.capability_tags.some((tag) => ["speech_to_text", "transcription", "audio"].includes(tag))
      || /whisper|transcrib/i.test(model.name);

    if (speechProviderSetting || speechModelSetting) {
      if (speechProviderSetting && speechModelSetting) {
        return { providerId: speechProviderSetting, modelName: speechModelSetting, source: "settings" as const };
      }
      if (speechProviderSetting) {
        const providerSpeechModel = models.find((model) => model.enabled && model.provider_id === speechProviderSetting && looksLikeSpeechModel(model));
        return {
          providerId: speechProviderSetting,
          modelName: providerSpeechModel?.name ?? speechModelSetting ?? "whisper-1",
          source: "settings" as const,
        };
      }
      const namedSpeechModel = models.find((model) => model.enabled && model.name === speechModelSetting);
      if (namedSpeechModel) {
        return { providerId: namedSpeechModel.provider_id, modelName: namedSpeechModel.name, source: "settings" as const };
      }
    }

    const sameProvider = models.find((model) => model.enabled && model.provider_id === providerId && looksLikeSpeechModel(model));
    if (sameProvider) {
      return { providerId: sameProvider.provider_id, modelName: sameProvider.name, source: "planner" as const };
    }

    const anySpeechModel = models.find((model) => model.enabled && looksLikeSpeechModel(model));
    if (anySpeechModel) {
      return { providerId: anySpeechModel.provider_id, modelName: anySpeechModel.name, source: "auto" as const };
    }

    if (providerId) {
      return { providerId, modelName: "whisper-1", source: "fallback" as const };
    }
    return null;
  }, [models, providerId, speechModelSetting, speechProviderSetting]);

  const context = useMemo<ResolverContext>(() => ({
    products,
    productTrees,
    workItems,
    activeProductId: selectedProductId,
    activeProductAreaId,
    activeCapabilityId,
    activeWorkItemId,
  }), [activeCapabilityId, activeProductAreaId, selectedProductId, activeWorkItemId, productTrees, products, workItems]);
  const activeProductName = selectedProduct?.name ?? null;

  useEffect(() => {
    if (activePlannerProductRef.current === selectedProductId) {
      return;
    }
    const previousProductId = activePlannerProductRef.current;
    activePlannerProductRef.current = selectedProductId;
    if (previousProductId === null && !sessionId && draftTreeNodes.length === 0 && !pendingPlan) {
      return;
    }
    setPendingPlan(null);
    setDraftTreeNodes([]);
    setSelectedDraftNodeId(null);
    setExpandedDraftNodeIds([]);
    setLatestTraceEvents([]);
    setSessionId(null);
    setPlannerView("conversation");
    setMessages([
      {
        id: makeId(),
        role: "assistant",
        content: selectedProduct
          ? `Planning is now scoped to ${selectedProduct.name}. Describe what you want to design or change inside this product.`
          : "Select a product first. Create products in the Products page, then return here to design structure and work.",
      },
    ]);
  }, [draftTreeNodes.length, pendingPlan, selectedProduct, selectedProductId, sessionId]);

  useEffect(() => {
    if (typeof window === "undefined") {
      return undefined;
    }
    const onResize = () => setWindowWidth(window.innerWidth);
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  }, []);

  useEffect(() => {
    if (!providerId && providers.length > 0) {
      setProviderId(providers[0].id);
    }
  }, [providerId, providers]);

  useEffect(() => {
    if (!selectedRepositoryId && repositories.length > 0) {
      setSelectedRepositoryId(repositories[0].id);
    }
  }, [repositories, selectedRepositoryId]);

  useEffect(() => {
    let cancelled = false;
    void Promise.all([
      getSetting(SPEECH_PROVIDER_KEY),
      getSetting(SPEECH_MODEL_KEY),
      getSetting(SPEECH_LOCALE_KEY),
      getSetting(SPEECH_NATIVE_VOICE_KEY),
      getSetting(SPEECH_ENABLE_MIC_KEY),
      getSetting(SPEECH_AUTO_SPEAK_REPLIES_KEY),
      getSetting(SPEECH_REVIEW_BEFORE_SEND_KEY),
    ]).then(([providerSetting, modelSetting, localeSetting, nativeVoiceSetting, micEnabledSetting, autoSpeakSetting, reviewBeforeSendSetting]) => {
      if (cancelled) {
        return;
      }
      if (providerSetting) {
        setSpeechProviderSetting(providerSetting);
      }
      if (modelSetting) {
        setSpeechModelSetting(modelSetting);
      }
      if (localeSetting) {
        setSpeechLocaleSetting(localeSetting);
      }
      if (nativeVoiceSetting) {
        setSpeechNativeVoiceSetting(nativeVoiceSetting);
      }
      if (typeof micEnabledSetting === "string") {
        setVoiceEnabled(micEnabledSetting.trim().toLowerCase() !== "false");
      }
      if (typeof autoSpeakSetting === "string") {
        setAutoSpeak(autoSpeakSetting.trim().toLowerCase() === "true");
      }
      if (typeof reviewBeforeSendSetting === "string") {
        setReviewVoiceBeforeSend(reviewBeforeSendSetting.trim().toLowerCase() === "true");
      }
    }).catch(() => {});
    return () => {
      cancelled = true;
    };
  }, []);

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
    let cancelled = false;
    const ensureSession = async () => {
      if (sessionId) {
        return;
      }
      try {
        const session = await createPlannerSession({
          providerId: providerId || undefined,
          modelName: modelName || undefined,
        });
        if (!cancelled) {
          setSessionId(session.session_id);
        }
      } catch (error) {
        if (!cancelled) {
          setMessages((current) => [
            ...current,
            { id: makeId(), role: "assistant", content: String(error), meta: "Planner error", kind: "error" },
          ]);
        }
      }
    };
    void ensureSession();
    return () => {
      cancelled = true;
    };
  }, [modelName, providerId, sessionId]);

  useEffect(() => {
    if (!providerId) {
      return;
    }
    if (!modelName || !modelOptions.some((entry) => entry.name === modelName)) {
      setModelName(modelOptions[0]?.name ?? "");
    }
  }, [modelName, modelOptions, providerId]);

  useEffect(() => {
    if (!sessionId) {
      return;
    }
    void updatePlannerSession({
      sessionId,
      providerId: providerId || undefined,
      modelName: modelName || undefined,
    });
  }, [modelName, providerId, sessionId]);

  useEffect(() => {
    transcriptRef.current?.scrollTo({ top: transcriptRef.current.scrollHeight });
  }, [messages]);

  useEffect(() => {
    const state = location.state as { plannerPrompt?: string; plannerView?: "conversation" | "draft" | "trace" } | null;
    const prompt = state?.plannerPrompt?.trim();
    if (!prompt || consumedRoutePromptRef.current === prompt) {
      return;
    }
    consumedRoutePromptRef.current = prompt;
    setDraft(prompt);
    setPlannerView(state?.plannerView ?? "conversation");
    composerRef.current?.focus();
    navigate(location.pathname, { replace: true, state: null });
  }, [location.pathname, location.state, navigate]);

  useEffect(() => {
    if (draftTreeNodes.length === 0 && plannerView === "draft") {
      setPlannerView("conversation");
    }
  }, [draftTreeNodes.length, plannerView]);

  useEffect(() => {
    if (!isCompactScreen) {
      setShowCompactTools(false);
    }
  }, [isCompactScreen]);

  useEffect(() => {
    const allNodeIds = collectTreeNodeIds(draftTreeNodes);
    if (allNodeIds.length === 0) {
      setExpandedDraftNodeIds([]);
      return;
    }
    setExpandedDraftNodeIds((current) => {
      const currentSet = new Set(current.filter((nodeId) => allNodeIds.includes(nodeId)));
      if (currentSet.size === 0) {
        return allNodeIds;
      }
      if (selectedDraftNodeId && !currentSet.has(selectedDraftNodeId)) {
        currentSet.add(selectedDraftNodeId);
      }
      return Array.from(currentSet);
    });
  }, [draftTreeNodes, selectedDraftNodeId]);

  useEffect(() => {
    setRenameDraftName(selectedDraftNode?.label ?? "");
    setDraftEditError(null);
    setDraftEditMessage(null);
  }, [selectedDraftNodeId, selectedDraftNode?.label]);

  useEffect(() => {
    if (allowedDraftChildTypes.length === 0) {
      return;
    }
    if (!allowedDraftChildTypes.includes(draftChildType)) {
      setDraftChildType(allowedDraftChildTypes[0]);
    }
  }, [allowedDraftChildTypes, draftChildType]);

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

  const mapPlannerResponseToMutationResult = (
    response: PlannerTurnResponse,
    userInput: string,
  ): PlannerMutationResult => {
    const backendPlan = (response.pending_plan as unknown as PlannerPlan) ?? {
      assistant_response: response.assistant_message,
      needs_confirmation: false,
      clarification_question: response.status === "clarification" ? response.assistant_message : null,
      actions: [],
    };
    const execution: ExecutionResult = {
      lines: response.execution_lines,
      errors: response.execution_errors,
    };
    const treeNodes = (response.tree_nodes as unknown as PlannerTreeNode[] | undefined) ?? undefined;
    const responseDraftTreeNodes = (response.draft_tree_nodes as unknown as PlannerTreeNode[] | undefined) ?? undefined;
    const responseSelectedDraftNodeId = response.selected_draft_node_id ?? null;
    const traceEvents = response.trace_events ?? [];

    if (response.status === "proposal" && responseDraftTreeNodes) {
      return {
        mode: "draft_updated",
        userInput,
        plan: backendPlan,
        execution,
        treeNodes,
        draftTreeNodes: responseDraftTreeNodes,
        selectedDraftNodeId: responseSelectedDraftNodeId,
        traceEvents,
      };
    }

    if (response.status === "proposal") {
      return {
        mode: "confirmation_required",
        userInput,
        plan: backendPlan,
        execution: null,
        treeNodes,
        draftTreeNodes: responseDraftTreeNodes,
        selectedDraftNodeId: responseSelectedDraftNodeId,
        traceEvents,
      };
    }

    if (response.status === "clarification") {
      return {
        mode: "clarification",
        userInput,
        plan: backendPlan,
        execution: null,
        treeNodes,
        draftTreeNodes: responseDraftTreeNodes,
        selectedDraftNodeId: responseSelectedDraftNodeId,
        traceEvents,
      };
    }

    if (response.status === "session_update") {
      return {
        mode: "session_updated",
        userInput,
        plan: backendPlan,
        execution,
        treeNodes,
        draftTreeNodes: responseDraftTreeNodes,
        selectedDraftNodeId: responseSelectedDraftNodeId,
        traceEvents,
      };
    }

    if (response.status === "error") {
      return {
        mode: "failed",
        userInput,
        plan: backendPlan,
        execution,
        treeNodes,
        draftTreeNodes: responseDraftTreeNodes,
        selectedDraftNodeId: responseSelectedDraftNodeId,
        traceEvents,
      };
    }

    return {
      mode: "executed",
      userInput,
      plan: backendPlan,
      execution,
      treeNodes,
      draftTreeNodes: responseDraftTreeNodes,
      selectedDraftNodeId: responseSelectedDraftNodeId,
      traceEvents,
    };
  };

  const handlePlannerMutationSuccess = (result: PlannerMutationResult) => {
    setPendingVoiceTranscript(null);
    setEditableVoiceTranscript("");
    setVoiceActivity(null);
    setIsVoiceSubmitting(false);
    setLatestTraceEvents(result.traceEvents ?? []);
    setMessages((current) => {
      const next: PlannerMessage[] = [...current, { id: makeId(), role: "user", content: result.userInput, kind: "text" }];
      if (result.mode === "confirmation_required") {
        next.push({
          id: makeId(),
          role: "assistant",
          content: result.plan.assistant_response,
          meta: "Suggestion awaiting confirmation",
          kind: "proposal",
          plan: result.plan,
          treeNodes: result.treeNodes,
          traceEvents: result.traceEvents,
        });
        return next;
      }
      if (result.mode === "draft_updated") {
        const output = [
          result.plan.assistant_response,
          ...(result.execution?.lines ?? []),
        ].join("\n");
        next.push({
          id: makeId(),
          role: "assistant",
          content: output,
          meta: "Design updated",
          kind: "proposal",
          plan: result.plan,
          treeNodes: result.treeNodes,
          traceEvents: result.traceEvents,
        });
        return next;
      }
      if (result.mode === "confirmed") {
        const output = [
          "Executed pending plan.",
          ...(result.execution?.lines ?? []),
          ...(result.execution?.errors.length ? [`Errors: ${result.execution.errors.join(" | ")}`] : []),
        ].join("\n");
        next.push({
          id: makeId(),
          role: "assistant",
          content: output,
          meta: "Planner execution",
          kind: result.treeNodes ? "tree" : "execution",
          treeNodes: result.treeNodes,
          plan: result.plan,
          traceEvents: result.traceEvents,
        });
        return next;
      }
      if (result.mode === "clarification") {
        next.push({
          id: makeId(),
          role: "assistant",
          content: result.plan.clarification_question ?? result.plan.assistant_response,
          meta: "Need more detail",
          kind: "text",
          traceEvents: result.traceEvents,
        });
        return next;
      }
      if (result.mode === "session_updated") {
        const output = [
          result.plan.assistant_response,
          ...(result.execution?.lines ?? []),
          ...(result.execution?.errors.length ? [`Errors: ${result.execution.errors.join(" | ")}`] : []),
        ].join("\n");
        next.push({
          id: makeId(),
          role: "assistant",
          content: output,
          meta: "Planner state updated",
          kind: "text",
          traceEvents: result.traceEvents,
        });
        return next;
      }
      if (result.mode === "failed") {
        next.push({
          id: makeId(),
          role: "assistant",
          content: [result.plan.assistant_response, ...(result.execution.errors.length ? [`Errors: ${result.execution.errors.join(" | ")}`] : [])].join("\n"),
          meta: "Planner error",
          kind: "error",
          traceEvents: result.traceEvents,
        });
        return next;
      }
      const output = [
        result.plan.assistant_response,
        ...(result.execution?.lines ?? []),
        ...(result.execution?.errors.length ? [`Errors: ${result.execution.errors.join(" | ")}`] : []),
      ].join("\n");
      next.push({
        id: makeId(),
        role: "assistant",
        content: output,
        meta: isInformationalOnly(result.plan) ? "Status report" : "Planner execution",
        kind: result.treeNodes ? "tree" : isInformationalOnly(result.plan) ? "report" : "execution",
        treeNodes: result.treeNodes,
        plan: result.plan,
        traceEvents: result.traceEvents,
      });
      return next;
    });

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
      const lastAssistant = result.mode === "clarification"
        ? result.plan.clarification_question ?? result.plan.assistant_response
        : result.mode === "confirmation_required"
          ? `${result.plan.assistant_response}. Say confirm to apply the proposal.`
          : result.mode === "draft_updated"
            ? `${result.plan.assistant_response}. The design tree has been updated.`
            : result.mode === "session_updated"
              ? result.plan.assistant_response
            : result.mode === "confirmed"
              ? "Executed the pending planner actions."
              : result.plan.assistant_response;
      void speakAssistantReply(lastAssistant);
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

  const repositoryAnalysisMutation = useMutation<PlannerMutationResult, Error, string>({
    mutationFn: async (repositoryId: string) => {
      if (!sessionId) {
        throw new Error("Planner session is not ready.");
      }
      const response = await analyzeRepositoryForPlanner({
        sessionId,
        repositoryId,
        selectedDraftNodeId,
        productId: selectedProductId,
      });
      return mapPlannerResponseToMutationResult(
        response,
        `Analyze repository ${repositoryId} into a design packet.`,
      );
    },
    onSuccess: handlePlannerMutationSuccess,
    onError: (error) => {
      setRepoAnalysisError(error instanceof Error ? error.message : String(error));
      setRepoAnalysisMessage(null);
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

  const isPlannerBusy =
    processMutation.isPending ||
    draftEditMutation.isPending ||
    repositoryAnalysisMutation.isPending ||
    transcribeAudioMutation.isPending ||
    isVoiceSubmitting;

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

  const browseRepositoryPathForPlanner = async () => {
    try {
      setRepoAnalysisError(null);
      const selectedPath = await browseForRepositoryPath();
      if (selectedPath) {
        setRepositoryPathDraft(selectedPath);
      }
    } catch (error) {
      setRepoAnalysisError(String(error));
    }
  };

  const registerRepositoryForPlanner = async () => {
    const localPath = repositoryPathDraft.trim();
    if (!localPath) {
      return;
    }
    try {
      setRepoAnalysisError(null);
      setRepoAnalysisMessage(null);
      const segments = localPath.split(/[\\/]/).filter(Boolean);
      const inferredName = segments[segments.length - 1] ?? "repository";
      const repository = await registerRepository({
        name: inferredName,
        localPath,
        remoteUrl: "",
        defaultBranch: "main",
      });
      setSelectedRepositoryId(repository.id);
      setRepositoryPathDraft("");
      setRepoAnalysisMessage(`Registered repository "${repository.name}".`);
      void queryClient.invalidateQueries({ queryKey: ["plannerRepositories"] });
    } catch (error) {
      setRepoAnalysisError(String(error));
    }
  };

  const analyzeSelectedRepository = async () => {
    if (!selectedRepositoryId || !selectedProductId || isPlannerBusy) {
      return;
    }
    try {
      setRepoAnalysisError(null);
      setRepoAnalysisMessage(null);
      await repositoryAnalysisMutation.mutateAsync(selectedRepositoryId);
      setRepoAnalysisMessage("Repository analysis staged a design update.");
    } catch {
      // Error state is handled by the mutation.
    }
  };

  const exportDesignReviewPacket = async () => {
    if (isExportingDesignPacket) {
      return;
    }
    const packetRootName = draftTreeNodes[0]?.label ?? activeProductName ?? "Design Review Packet";
    try {
      setIsExportingDesignPacket(true);
      setDesignPacketError(null);
      const exportProductTrees = selectedProductId
        ? [
            await queryClient.fetchQuery({
              queryKey: ["plannerProductTree", selectedProductId],
              queryFn: () => getProductTree(selectedProductId),
            }),
          ]
        : productTrees;
      const html = buildDesignReviewPacketHtml({
        title: packetRootName,
        generatedAt: new Date().toLocaleString(),
        activeProductName,
        currentProducts: products,
        currentProductTrees: exportProductTrees,
        currentWorkItems: workItems,
        currentWorkItemsHasMore: plannerWorkItemsHasMore,
        draftTreeNodes,
        plan: latestDraftPlan,
        validation: draftValidation,
        selectedNode: selectedDraftNode,
        latestAssistantText: latestAssistantMessage?.content ?? null,
      });
      const path = await exportProductOverviewHtml({
        fileName: `${slugifyPacketName(packetRootName)}-design-review-packet.html`,
        html,
      });
      setDesignPacketPath(path);
      setMessages((current) => [
        ...current,
        {
          id: makeId(),
          role: "assistant",
          content: `Generated a design review packet for "${packetRootName}".\n${path}`,
          meta: "Design packet exported",
          kind: "text",
        },
      ]);
    } catch (error) {
      setDesignPacketPath(null);
      setDesignPacketError(error instanceof Error ? error.message : String(error));
    } finally {
      setIsExportingDesignPacket(false);
    }
  };

  const toggleDraftNodeExpanded = (nodeId: string) => {
    setExpandedDraftNodeIds((current) =>
      current.includes(nodeId) ? current.filter((value) => value !== nodeId) : [...current, nodeId],
    );
  };

  const expandAllDraftNodes = () => {
    setExpandedDraftNodeIds(collectTreeNodeIds(draftTreeNodes));
  };

  const collapseAllDraftNodes = () => {
    setExpandedDraftNodeIds([]);
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

  function parseVoiceNodeReference(
    spokenRemainder: string,
  ): { explicitType?: string; reference: string } {
    const trimmed = spokenRemainder.trim();
    const prefixes: Array<{ prefix: string; type: string }> = [
      { prefix: "work item ", type: "work item" },
      { prefix: "work-item ", type: "work item" },
      { prefix: "capability ", type: "capability" },
      { prefix: "product area ", type: "product area" },
      { prefix: "product ", type: "product" },
      { prefix: "node ", type: "node" },
    ];
    for (const option of prefixes) {
      if (trimmed === option.prefix.trim()) {
        return { explicitType: option.type, reference: `selected ${option.type}` };
      }
      if (trimmed.startsWith(option.prefix)) {
        return { explicitType: option.type, reference: trimmed.slice(option.prefix.length).trim() };
      }
    }
    return { reference: trimmed };
  }

  async function handleVoiceTranscript(transcript: string) {
    const spoken = transcript.trim();
    if (!spoken) {
      return true;
    }
    const normalizedTranscript = normalize(spoken);

    if ([
      "view draft",
      "open draft",
      "show draft",
      "show draft tree",
      "view draft tree",
      "view design",
      "open design",
      "show design",
      "show design tree",
      "view design tree",
      "open workspace",
      "show workspace",
    ].includes(normalizedTranscript)) {
      if (draftTreeNodes.length === 0) {
        appendVoiceCommandFeedback(spoken, "There is no staged design tree yet.");
      } else {
        setPlannerView("draft");
        appendVoiceCommandFeedback(spoken, "Opened the design review.");
      }
      return true;
    }

    if (["view trace", "show trace", "open trace"].includes(normalizedTranscript)) {
      if (latestTraceEvents.length === 0) {
        appendVoiceCommandFeedback(spoken, "There is no planner trace available yet.");
      } else {
        setPlannerView("trace");
        appendVoiceCommandFeedback(spoken, "Opened the latest planner trace.");
      }
      return true;
    }

    if (["view conversation", "open conversation", "show conversation", "back to chat", "view chat"].includes(normalizedTranscript)) {
      setPlannerView("conversation");
      appendVoiceCommandFeedback(spoken, "Switched back to the planner conversation.");
      return true;
    }

    if (!selectedProductId) {
      appendVoiceCommandFeedback(spoken, "Select a product before planning. Create products in the Products page, then return here to design.");
      return true;
    }

    if (["expand draft", "expand the draft", "expand tree", "expand all", "open all branches"].includes(normalizedTranscript)) {
      setPlannerView("draft");
      expandAllDraftNodes();
      appendVoiceCommandFeedback(spoken, "Expanded the staged design tree.");
      return true;
    }

    if (["collapse draft", "collapse the draft", "collapse tree", "collapse all"].includes(normalizedTranscript)) {
      collapseAllDraftNodes();
      appendVoiceCommandFeedback(spoken, "Collapsed the staged design tree.");
      return true;
    }

    const collapseMatch = normalizedTranscript.match(/^(collapse|close)\s+(.+)$/);
    if (normalizedTranscript.startsWith("expand ") || normalizedTranscript.startsWith("open ")) {
      const targetText = spoken.replace(/^(expand|open)\s+/i, "").trim();
      if (["draft", "tree", "all"].includes(normalize(targetText))) {
        setPlannerView("draft");
        expandAllDraftNodes();
        appendVoiceCommandFeedback(spoken, "Expanded the staged design tree.");
        return true;
      }
    }

    if (collapseMatch) {
      const targetText = collapseMatch[2];
      if (["draft", "tree", "all"].includes(normalize(targetText))) {
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

  const applyPromptSuggestion = (prompt: string) => {
    setDraft(prompt);
    composerRef.current?.focus();
  };

  const renameSelectedDraftNode = async () => {
    if (!selectedDraftNode || !renameDraftName.trim() || isPlannerBusy) {
      return;
    }
    setDraftEditError(null);
    setDraftEditMessage(null);
    try {
      await draftEditMutation.mutateAsync({
        kind: "rename",
        nodeId: selectedDraftNode.id,
        name: renameDraftName.trim(),
      });
      setDraftEditMessage(`Renamed to "${renameDraftName.trim()}".`);
    } catch {
      // Error state is handled by the mutation.
    }
  };

  const addChildToSelectedDraftNode = async () => {
    if (!selectedDraftNode || !draftChildName.trim() || allowedDraftChildTypes.length === 0 || isPlannerBusy) {
      return;
    }
    setDraftEditError(null);
    setDraftEditMessage(null);
    try {
      await draftEditMutation.mutateAsync({
        kind: "add_child",
        parentNodeId: selectedDraftNode.id,
        childType: draftChildType,
        name: draftChildName.trim(),
        summary: draftChildSummary.trim() || undefined,
      });
      setDraftChildName("");
      setDraftChildSummary("");
      setDraftEditMessage(`Added ${formatDraftChildTypeLabel(draftChildType).toLowerCase()} "${draftChildName.trim()}".`);
    } catch {
      // Error state is handled by the mutation.
    }
  };

  const deleteSelectedDraftNode = async () => {
    if (!selectedDraftNode || isPlannerBusy) {
      return;
    }
    setDraftEditError(null);
    setDraftEditMessage(null);
    const deletedLabel = selectedDraftNode.label;
    try {
      await draftEditMutation.mutateAsync({
        kind: "delete",
        nodeId: selectedDraftNode.id,
      });
      setDraftEditMessage(`Removed "${deletedLabel}" from the design.`);
    } catch {
      // Error state is handled by the mutation.
    }
  };

  const renderAssistantMessage = (message: PlannerMessage) => {
    if (message.kind === "proposal" && message.plan) {
      const proposalTreeNodes = buildProposalTreeNodes(message.plan);
      return (
        <>
          <FormattedPlannerText content={message.content} />
          <div style={styles.card}>
            <div style={styles.cardTitle}>Proposed Changes</div>
            {message.plan.actions.map((action, index) => {
              const summary = summarizeAction(action);
              const symbolStyle = summary.tone === "add"
                ? styles.diffSymbolAdd
                : summary.tone === "update"
                  ? styles.diffSymbolUpdate
                  : styles.diffSymbolWarn;
              return (
                <div key={`${action.type}-${index}`} style={styles.diffRow}>
                  <div style={symbolStyle}>{summary.symbol}</div>
                  <div>
                    <div style={styles.diffPrimary}>{summary.title}</div>
                    {summary.detail ? <div style={styles.diffSecondary}>{summary.detail}</div> : null}
                  </div>
                </div>
              );
            })}
            {proposalTreeNodes.length > 0 ? (
              <div style={styles.cardSection}>
                <div style={styles.cardTitle}>Proposed Design</div>
                <div style={styles.treePanel}>
                  {proposalTreeNodes.map((node) => (
                    <TreeNodeView key={node.id} node={node} />
                  ))}
                </div>
              </div>
            ) : null}
            {message.treeNodes && message.treeNodes.length > 0 ? (
              <div style={styles.cardSection}>
                <div style={styles.cardTitle}>Current Design</div>
                <div style={styles.treePanel}>
                  {message.treeNodes.map((node) => (
                    <TreeNodeView key={node.id} node={node} />
                  ))}
                </div>
              </div>
            ) : null}
            <div style={styles.inlineButtonRow}>
              <button style={styles.btnGhost} onClick={() => void exportDesignReviewPacket()} disabled={isExportingDesignPacket}>
                {isExportingDesignPacket ? "Generating Packet..." : "Generate Review Packet"}
              </button>
              <button style={styles.btn} onClick={confirmPendingPlan} disabled={isPlannerBusy || (!pendingPlan && draftTreeNodes.length === 0)}>
                {draftTreeNodes.length > 0 ? "Apply Design" : "Confirm Proposal"}
              </button>
              <button style={styles.btnGhost} onClick={dismissPendingPlan} disabled={!pendingPlan && draftTreeNodes.length === 0}>
                {draftTreeNodes.length > 0 ? "Clear Design" : "Dismiss"}
              </button>
            </div>
            {designPacketPath ? (
              <div style={{ ...styles.helper, marginTop: 8 }}>
                Packet exported: {designPacketPath}
              </div>
            ) : null}
            {designPacketError ? <div style={styles.error}>{designPacketError}</div> : null}
          </div>
        </>
      );
    }

    if (message.kind === "tree" && message.treeNodes) {
      return (
        <>
          <FormattedPlannerText content={message.content} />
          <div style={styles.treePanel}>
            {message.treeNodes.map((node) => (
              <TreeNodeView key={node.id} node={node} />
            ))}
          </div>
        </>
      );
    }

    return <FormattedPlannerText content={message.content} />;
  };

  const renderPlannerSidebar = () => (
    <div style={styles.panel}>
      <div style={isCompactScreen ? styles.compactPanelBody : styles.panelBody}>
        <div style={styles.sectionTitle}>Planner Controls</div>
        <div style={styles.sideCard}>
          <div style={styles.helper}>
            {hasTreeData ? "Product area context is loaded. Full capability trees load only for packet export or backend planner actions." : "Product area context will activate once product structure finishes loading."}
          </div>
          {plannerWorkItemsHasMore ? (
            <div style={{ ...styles.warning, marginTop: 8 }}>
              Planner context is showing the first {PLANNER_WORK_ITEM_PAGE_SIZE} story/task items. Use Work Items for full paged delivery browsing.
            </div>
          ) : null}
        </div>

        <div style={styles.sideCard}>
          <div style={styles.label}>Design Tree</div>
          <div style={styles.helper}>
            Build the plan here first. Select a node, then ask follow-up questions like “expand this capability” or “add stories under this feature.”
          </div>
          <div style={{ height: 10 }} />
          {draftTreeNodes.length > 0 ? (
            <div style={styles.treePanel}>
              <div style={styles.treeExplorer}>
                {draftTreeNodes.map((node) => (
                  <SelectableTreeNodeView
                    key={node.id}
                    node={node}
                    selectedNodeId={selectedDraftNodeId}
                    onSelect={setSelectedDraftNodeId}
                    expandedNodeIds={expandedDraftNodeIdSet}
                    onToggle={toggleDraftNodeExpanded}
                  />
                ))}
              </div>
            </div>
          ) : (
            <div style={styles.helper}>No staged design yet. Select a product, then ask the planner to design product areas, capabilities, features, and starter stories inside it.</div>
          )}
        </div>

        {pendingPlan || draftTreeNodes.length > 0 ? (
          <div style={styles.sideCard}>
            <div style={styles.label}>Design Snapshot</div>
            <div style={styles.helper}>
              The planner stages structure here first. Generate a review packet, keep refining the tree, then apply when the design looks right.
            </div>
            {pendingPlan ? (
              <div style={styles.list}>
                {pendingPlan.plan.actions.map((action, index) => (
                  <div key={`${action.type}-${index}`} style={styles.listItem}>
                    <div style={styles.listItemTitle}>{action.type}</div>
                    <div style={styles.listItemMeta}>{JSON.stringify(action, null, 2)}</div>
                  </div>
                ))}
              </div>
            ) : (
              <div style={{ ...styles.helper, marginTop: 10 }}>
                The current staged design is active in the tree above. Select a node and keep iterating, or apply it when approved.
              </div>
            )}
          </div>
        ) : null}
      </div>
    </div>
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
            <div style={styles.sectionHeader}>
              <div style={styles.sectionTitle}>
                {plannerView === "draft" ? "Design Review" : plannerView === "trace" ? "Planner Trace" : "Conversation"}
              </div>
              <div style={styles.viewToggleRow}>
                <button
                  aria-label="Reverse engineer repository"
                  style={styles.iconButton}
                  onClick={() => setShowRepoModal(true)}
                >
                  ⌕ Repo
                </button>
                <select
                  aria-label="Planner product"
                  style={{ ...styles.select, width: 240 }}
                  value={selectedProductId ?? ""}
                  onChange={(event) => {
                    const nextProductId = event.target.value || null;
                    setActiveProduct(nextProductId);
                  }}
                >
                  <option value="">Select product</option>
                  {products.map((product) => (
                    <option key={product.id} value={product.id}>
                      {product.name}
                    </option>
                  ))}
                </select>
                <button style={styles.btnGhost} onClick={() => navigate("/products")}>
                  Create Product
                </button>
                <select
                  aria-label="Planner model"
                  style={{ ...styles.select, width: 260 }}
                  value={plannerModelPickerValue}
                  onChange={(event) => {
                    const [nextProviderId, nextModelName] = event.target.value.split("::");
                    setProviderId(nextProviderId ?? "");
                    setModelName(nextModelName ?? "");
                  }}
                >
                  <option value="">Select model</option>
                  {plannerModelPickerOptions.map((option) => (
                    <option key={option.value} value={option.value}>
                      {option.label}
                    </option>
                  ))}
                </select>
                <button
                  data-testid="planner-view-conversation"
                  style={plannerView === "conversation" ? styles.btn : styles.btnGhost}
                  onClick={() => setPlannerView("conversation")}
                >
                  Conversation
                </button>
                <button
                  data-testid="planner-view-draft"
                  style={plannerView === "draft" ? styles.btn : styles.btnGhost}
                  onClick={() => setPlannerView("draft")}
                  disabled={draftTreeNodes.length === 0}
                >
                  View Design
                </button>
                <button
                  data-testid="planner-view-trace"
                  style={plannerView === "trace" ? styles.btn : styles.btnGhost}
                  onClick={() => setPlannerView("trace")}
                  disabled={latestTraceEvents.length === 0}
                >
                  View Trace
                </button>
              </div>
            </div>

            <div style={styles.statusBanner}>
              <div>
                <div style={styles.statusBannerStrong}>{plannerStatusSummary.title}</div>
                <div style={styles.statusBannerMeta}>{plannerStatusSummary.detail}</div>
              </div>
              <div style={styles.chipRow}>
                {providerId ? <div style={styles.chip}>{providers.find((provider) => provider.id === providerId)?.name ?? "provider selected"}</div> : null}
                {modelName ? <div style={styles.chip}>{modelName}</div> : null}
                {selectedDraftNode ? <div style={styles.chip}>selected: {selectedDraftNode.label}</div> : null}
              </div>
            </div>

            {isCompactScreen && plannerView === "conversation" ? (
              <>
                <div style={styles.compactControlStrip}>
                  <button style={styles.btnGhost} onClick={() => setShowCompactTools((value) => !value)}>
                    {showCompactTools ? "Hide Tools" : "Show Tools"}
                  </button>
                  <button style={styles.btnGhost} onClick={() => setPlannerView("draft")} disabled={draftTreeNodes.length === 0}>
                    Open Design
                  </button>
                  <button style={styles.btnGhost} onClick={() => setPlannerView("trace")} disabled={latestTraceEvents.length === 0}>
                    Open Trace
                  </button>
                </div>
                <div style={styles.compactSummaryCard}>
                  <div style={styles.compactSummaryGrid}>
                    <div style={styles.compactSummaryItem}>
                      <div style={styles.compactSummaryLabel}>Design</div>
                      <div style={styles.compactSummaryValue}>{draftTreeNodes.length > 0 ? `${draftValidation.counts["product area"]} product areas staged` : "No active design"}</div>
                    </div>
                    <div style={styles.compactSummaryItem}>
                      <div style={styles.compactSummaryLabel}>Selection</div>
                      <div style={styles.compactSummaryValue}>{selectedDraftNode?.label ?? "None"}</div>
                    </div>
                    <div style={styles.compactSummaryItem}>
                      <div style={styles.compactSummaryLabel}>Readiness</div>
                      <div style={styles.compactSummaryValue}>{draftTreeNodes.length > 0 ? `${draftValidation.score}` : "n/a"}</div>
                    </div>
                    <div style={styles.compactSummaryItem}>
                      <div style={styles.compactSummaryLabel}>State</div>
                      <div style={styles.compactSummaryValue}>
                        {pendingVoiceTranscript ? "Review transcript" : isPlannerBusy ? "Working" : pendingPlan ? "Need confirm" : "Ready"}
                      </div>
                    </div>
                  </div>
                </div>
              </>
            ) : null}

            {plannerView === "draft" ? (
              <div style={styles.draftWorkspace}>
                <div style={styles.draftWorkspaceMain}>
                  <div style={styles.draftCanvas}>
                    <div style={styles.draftCanvasHeader}>
                      <div>
                        <div style={styles.draftCanvasTitle}>Staged Design Tree</div>
                        <div style={styles.helper}>
                          Select a node, then refine it in natural language. The composer below will use the selected design node as planning context.
                        </div>
                      </div>
                      <div style={styles.chipRow}>
                        {selectedDraftNode ? <div style={styles.chip}>selected: {selectedDraftNode.label}</div> : null}
                        <div style={styles.chip}>{draftTreeNodes.length} root {draftTreeNodes.length === 1 ? "node" : "nodes"}</div>
                      </div>
                    </div>
                    <div style={styles.readinessBanner}>
                      <div>
                        <div style={styles.label}>Apply Readiness</div>
                        <div style={styles.readinessMeta}>
                          {draftValidation.issues.filter((issue) => issue.tone === "warn").length === 0
                            ? "This design is structurally solid enough to apply."
                            : "There are still weak spots in the staged tree. Fix them before applying if you want a cleaner catalog."}
                        </div>
                      </div>
                      <div style={styles.readinessScore}>{draftValidation.score}</div>
                    </div>
                    <div style={styles.metricGrid}>
                      <div style={styles.metricCard}>
                        <div style={styles.metricLabel}>Products</div>
                        <div style={styles.metricValue}>{draftValidation.counts.product}</div>
                      </div>
                      <div style={styles.metricCard}>
                        <div style={styles.metricLabel}>Product Areas</div>
                        <div style={styles.metricValue}>{draftValidation.counts["product area"]}</div>
                      </div>
                      <div style={styles.metricCard}>
                        <div style={styles.metricLabel}>Capabilities</div>
                        <div style={styles.metricValue}>{draftValidation.counts.capability}</div>
                      </div>
                      <div style={styles.metricCard}>
                        <div style={styles.metricLabel}>Delivery Items</div>
                        <div style={styles.metricValue}>{draftValidation.counts["work item"]}</div>
                      </div>
                    </div>
                    <div style={styles.treeToolbar}>
                      <button data-testid="draft-expand-all" style={styles.btnGhost} onClick={expandAllDraftNodes} disabled={draftTreeNodes.length === 0}>
                        Expand All
                      </button>
                      <button data-testid="draft-collapse-all" style={styles.btnGhost} onClick={collapseAllDraftNodes} disabled={draftTreeNodes.length === 0}>
                        Collapse All
                      </button>
                      <div style={styles.treeToolbarSpacer} />
                      <div style={styles.helper}>
                        Select a node to scope prompts. Expand branches to inspect the staged structure.
                      </div>
                    </div>
                    {draftTreeNodes.length > 0 ? (
                      <div style={styles.treeExplorer}>
                        {draftTreeNodes.map((node) => (
                          <SelectableTreeNodeView
                            key={node.id}
                            node={node}
                            selectedNodeId={selectedDraftNodeId}
                            onSelect={setSelectedDraftNodeId}
                            expandedNodeIds={expandedDraftNodeIdSet}
                            onToggle={toggleDraftNodeExpanded}
                          />
                        ))}
                      </div>
                    ) : (
                      <div style={styles.emptyState}>
                        No staged design yet. Ask the planner to design product areas, capabilities, features, stories, or tasks for the selected product, then switch back here to inspect and refine it.
                      </div>
                    )}
                  </div>
                  <PlannerComposer
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
                    scopeHint={composerScopeHint}
                    isProductSelected={Boolean(selectedProductId)}
                  />
                </div>

                <div style={styles.draftWorkspaceSide}>
                  <div style={styles.sideCard}>
                    <div style={styles.label}>Selected Node</div>
                    {selectedDraftNode ? (
                      <>
                        <div style={styles.cardTitle}>{selectedDraftNode.label}</div>
                        <div style={styles.helper}>Type: {getPlannerNodeType(selectedDraftNode)}</div>
                        {selectedDraftNode.summary ? <div style={{ ...styles.helper, marginTop: 8 }}>{selectedDraftNode.summary}</div> : null}
                        {selectedDraftNode.source || selectedDraftNode.confidence ? (
                          <div style={styles.chipRow}>
                            {selectedDraftNode.source ? <div style={styles.chip}>source: {selectedDraftNode.source.replace("_", " ")}</div> : null}
                            {selectedDraftNode.confidence ? <div style={styles.chip}>{selectedDraftNode.confidence} confidence</div> : null}
                          </div>
                        ) : null}
                        {selectedDraftNodePath.length > 0 ? (
                          <div style={styles.pathText}>
                            Path: {selectedDraftNodePath.map((node) => node.label).join(" / ")}
                          </div>
                        ) : null}
                        {selectedDraftNode.evidence && selectedDraftNode.evidence.length > 0 ? (
                          <>
                            <div style={styles.sectionDivider} />
                            <div style={styles.label}>Evidence</div>
                            <div style={styles.list}>
                              {selectedDraftNode.evidence.map((line: string) => (
                                <div key={line} style={styles.listItem}>
                                  <div style={styles.listItemMeta}>{line}</div>
                                </div>
                              ))}
                            </div>
                          </>
                        ) : null}
                        <div style={styles.sectionDivider} />
                        <div style={styles.label}>Edit Node</div>
                        <div style={styles.fieldGroup}>
                          <input
                            data-testid="draft-node-rename-input"
                            style={styles.input}
                            value={renameDraftName}
                            onChange={(event) => setRenameDraftName(event.target.value)}
                            placeholder="Rename this node"
                          />
                          <div style={styles.inlineButtonRow}>
                            <button
                              data-testid="draft-node-rename-save"
                              style={styles.mutedButton}
                              onClick={() => {
                                void renameSelectedDraftNode();
                              }}
                              disabled={!renameDraftName.trim() || isPlannerBusy}
                            >
                              Save Name
                            </button>
                            <button
                              data-testid="draft-node-delete"
                              style={styles.btnDanger}
                              onClick={() => {
                                void deleteSelectedDraftNode();
                              }}
                              disabled={isPlannerBusy}
                            >
                              Delete Node
                            </button>
                          </div>
                        </div>
                        <div style={styles.sectionDivider} />
                        <div style={styles.label}>Add Child</div>
                        {allowedDraftChildTypes.length > 0 ? (
                          <div style={styles.fieldGroup}>
                            <select
                              data-testid="draft-node-add-child-type"
                              style={styles.select}
                              value={draftChildType}
                              onChange={(event) => setDraftChildType(event.target.value as PlannerDraftChildType)}
                            >
                              {allowedDraftChildTypes.map((option: PlannerDraftChildType) => (
                                <option key={option} value={option}>
                                  {formatDraftChildTypeLabel(option)}
                                </option>
                              ))}
                            </select>
                            <input
                              data-testid="draft-node-add-child-name"
                              style={styles.input}
                              value={draftChildName}
                              onChange={(event) => setDraftChildName(event.target.value)}
                              placeholder={`Name the new ${formatDraftChildTypeLabel(draftChildType).toLowerCase()}`}
                            />
                            <textarea
                              data-testid="draft-node-add-child-summary"
                              style={styles.compactTextarea}
                              value={draftChildSummary}
                              onChange={(event) => setDraftChildSummary(event.target.value)}
                              placeholder="Optional summary or brief description"
                            />
                            <button
                              data-testid="draft-node-add-child-save"
                              style={styles.btnGhost}
                              onClick={() => {
                                void addChildToSelectedDraftNode();
                              }}
                              disabled={!draftChildName.trim() || isPlannerBusy}
                            >
                              Add {formatDraftChildTypeLabel(draftChildType)}
                            </button>
                          </div>
                        ) : (
                          <div style={styles.helper}>
                            This node is a leaf in the staged hierarchy. Use rename or delete, or select a higher branch to add more structure.
                          </div>
                        )}
                        {draftEditMessage ? <div style={styles.success}>{draftEditMessage}</div> : null}
                        {draftEditError ? <div style={styles.error}>{draftEditError}</div> : null}
                        <div style={styles.sectionDivider} />
                        <div style={styles.label}>Suggested Next Prompts</div>
                        <div style={styles.promptList}>
                          {selectedDraftNodePrompts.map((prompt) => (
                            <button key={prompt} style={styles.promptButton} onClick={() => applyPromptSuggestion(prompt)}>
                              {prompt}
                            </button>
                          ))}
                        </div>
                        <div style={styles.sectionDivider} />
                        <div style={styles.label}>Recent AI Changes For This Node</div>
                        {selectedNodeRecentActions.length > 0 ? (
                          <div style={styles.list}>
                            {selectedNodeRecentActions.map((action, index) => {
                              const summary = summarizeAction(action);
                              const symbolStyle = summary.tone === "add"
                                ? styles.diffSymbolAdd
                                : summary.tone === "update"
                                  ? styles.diffSymbolUpdate
                                  : styles.diffSymbolWarn;
                              return (
                                <div key={`${action.type}-${index}`} style={styles.diffRow}>
                                  <div style={symbolStyle}>{summary.symbol}</div>
                                  <div>
                                    <div style={styles.diffPrimary}>{summary.title}</div>
                                    <div style={styles.diffSecondary}>{summary.detail}</div>
                                  </div>
                                </div>
                              );
                            })}
                          </div>
                        ) : (
                          <div style={styles.helper}>
                            No recent planner operations are directly tied to this node yet.
                          </div>
                        )}
                      </>
                    ) : (
                      <div style={styles.helper}>
                        Select a node in the tree to anchor follow-up planning turns to that part of the design.
                      </div>
                    )}
                  </div>

                  <div style={styles.sideCard}>
                    <div style={styles.label}>Design Validation</div>
                    <div style={styles.helper}>
                      Structural checks for the staged tree before you apply it into the real catalog.
                    </div>
                    <div style={styles.issueList}>
                      {draftValidation.issues.slice(0, 6).map((issue, index) => {
                        const issueStyle = issue.tone === "ok"
                          ? styles.issueCardOk
                          : styles.issueCardWarn;
                        return (
                          <div key={`${issue.title}-${index}`} style={issueStyle}>
                            <div style={styles.issueTitle}>{issue.title}</div>
                            <div style={styles.issueDetail}>{issue.detail}</div>
                          </div>
                        );
                      })}
                    </div>
                  </div>

                  <div style={styles.sideCard}>
                    <div style={styles.label}>Design Review Packet</div>
                    <div style={styles.helper}>
                      Generate a reviewable HTML packet with architecture diagrams, feature changes, risks, work breakdown, and the approval-ready change set before applying anything.
                    </div>
                    <div style={styles.inlineButtonRow}>
                      <button style={styles.btnGhost} onClick={() => void exportDesignReviewPacket()} disabled={isExportingDesignPacket}>
                        {isExportingDesignPacket ? "Generating..." : "Generate Packet"}
                      </button>
                      {designPacketPath ? (
                        <button style={styles.btnGhost} onClick={() => void revealInFinder(designPacketPath)}>
                          Reveal Packet
                        </button>
                      ) : null}
                      <button data-testid="draft-commit" style={styles.btn} onClick={confirmPendingPlan} disabled={draftTreeNodes.length === 0 || isPlannerBusy}>
                        Apply Design
                      </button>
                      <button style={styles.btnGhost} onClick={() => setPlannerView("conversation")}>
                        Back to Chat
                      </button>
                      <button style={styles.btnDanger} onClick={dismissPendingPlan} disabled={draftTreeNodes.length === 0}>
                        Clear Design
                      </button>
                    </div>
                    {designPacketPath ? <div style={{ ...styles.success, marginTop: 10 }}>Packet exported to {designPacketPath}</div> : null}
                    {designPacketError ? <div style={{ ...styles.error, marginTop: 10 }}>{designPacketError}</div> : null}
                  </div>

                  <div style={styles.sideCard}>
                    <div style={styles.label}>Latest Design Ops</div>
                    {latestDraftPlan ? (
                      <>
                        <div style={styles.helper}>{latestDraftPlan.assistant_response}</div>
                        <div style={styles.list}>
                          {latestDraftPlan.actions.slice(0, 8).map((action, index) => {
                            const summary = summarizeAction(action);
                            const symbolStyle = summary.tone === "add"
                              ? styles.diffSymbolAdd
                              : summary.tone === "update"
                                ? styles.diffSymbolUpdate
                                : styles.diffSymbolWarn;
                            return (
                              <div key={`${action.type}-${index}`} style={styles.diffRow}>
                                <div style={symbolStyle}>{summary.symbol}</div>
                                <div>
                                  <div style={styles.diffPrimary}>{summary.title}</div>
                                  <div style={styles.diffSecondary}>{summary.detail}</div>
                                </div>
                              </div>
                            );
                          })}
                        </div>
                      </>
                    ) : (
                      <div style={styles.helper}>
                        No pending proposal snapshot. Use the chat to add structure, then review and keep refining the staged tree here.
                      </div>
                    )}
                  </div>
                </div>
              </div>
            ) : plannerView === "trace" ? (
              <div style={styles.draftWorkspaceMain}>
                <div style={styles.draftCanvas}>
                  <div style={styles.draftCanvasHeader}>
                    <div>
                      <div style={styles.draftCanvasTitle}>Latest Planner Turn Trace</div>
                      <div style={styles.helper}>
                        Inspect the raw planning flow: input context, model completions, tool calls, parsed plan, and any backend validation failure.
                      </div>
                    </div>
                    <div style={styles.chipRow}>
                      <div style={styles.chip}>{latestTraceEvents.length} events</div>
                    </div>
                  </div>
                  {latestTraceEvents.length > 0 ? (
                    <div style={styles.list}>
                      {latestTraceEvents.map((event) => (
                        <div key={`${event.step}-${event.title}`} style={styles.listItem}>
                          <div style={styles.listItemTitle}>
                            {event.step}. {event.title}
                          </div>
                          <div style={styles.helper}>{event.stage}</div>
                          <div style={{ ...styles.listItemMeta, marginTop: 8 }}>{event.detail}</div>
                        </div>
                      ))}
                    </div>
                  ) : (
                    <div style={styles.emptyState}>
                      No trace captured yet. Send a planner turn, then open this view to inspect the latest model/tool trace.
                    </div>
                  )}
                </div>
              </div>
            ) : (
              <div ref={transcriptRef} style={{ ...styles.transcript, flex: 1, minHeight: 0, overflow: "auto" }}>
                {pendingVoiceTranscript && reviewVoiceBeforeSend ? (
                  <div style={styles.voiceReviewCard}>
                    <div style={styles.voiceReviewHeader}>
                      <div>
                        <div style={styles.voiceReviewTitle}>Voice Transcript Preview</div>
                        <div style={styles.helper}>
                          Review or edit the recognized speech before sending it to the planner.
                        </div>
                      </div>
                      <div style={styles.chipRow}>
                        <div style={styles.chip}>elapsed {formatElapsedMs(voiceElapsedMs)}</div>
                        <div style={styles.chip}>{isVoiceSubmitting ? "sending" : "ready to send"}</div>
                      </div>
                    </div>
                    <textarea
                      style={{ ...styles.compactTextarea, minHeight: 88 }}
                      value={editableVoiceTranscript}
                      onChange={(event) => setEditableVoiceTranscript(event.target.value)}
                    />
                    <div style={styles.inlineButtonRow}>
                      <button style={styles.btn} onClick={() => void submitPendingVoiceTranscript()} disabled={!editableVoiceTranscript.trim() || isPlannerBusy}>
                        Send Transcript
                      </button>
                      <button style={styles.btnGhost} onClick={() => void retryVoiceCapture()} disabled={isPlannerBusy}>
                        Retry
                      </button>
                      <button style={styles.btnDanger} onClick={clearPendingVoiceReview} disabled={isPlannerBusy}>
                        Cancel
                      </button>
                    </div>
                  </div>
                ) : null}
                {messages.map((message) => (
                  <div key={message.id} style={message.role === "user" ? styles.bubbleUser : styles.bubbleAssistant}>
                    {message.role === "assistant" ? renderAssistantMessage(message) : message.content}
                    {message.meta ? <span style={styles.bubbleMeta}>{message.meta}</span> : null}
                  </div>
                ))}
              </div>
            )}
            {plannerView !== "draft" ? (
              <PlannerComposer
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
                scopeHint={composerScopeHint}
                isProductSelected={Boolean(selectedProductId)}
              />
            ) : null}
          </div>
        </div>

        {!isFocusedWorkspaceView && !isCompactScreen ? renderPlannerSidebar() : null}
        {!isFocusedWorkspaceView && isCompactScreen && showCompactTools ? renderPlannerSidebar() : null}
      </div>

      {showRepoModal ? (
        <div style={styles.modalOverlay} onClick={() => setShowRepoModal(false)}>
          <div style={styles.modalCard} onClick={(event) => event.stopPropagation()}>
            <div style={styles.modalHeader}>
              <div>
                <div style={styles.modalTitle}>Reverse Engineer Repository</div>
                <div style={styles.helper}>
                  Point the planner at an existing repository and let the model infer a staged product area, capability, feature, story, and task tree from the codebase.
                </div>
              </div>
              <button style={styles.btnGhost} onClick={() => setShowRepoModal(false)}>
                Close
              </button>
            </div>
            <label style={styles.label}>Registered Repository</label>
            <select
              style={styles.select}
              value={selectedRepositoryId}
              onChange={(event) => setSelectedRepositoryId(event.target.value)}
            >
              <option value="">Select a repository</option>
              {repositories.map((repository) => (
                <option key={repository.id} value={repository.id}>
                  {repository.name}
                </option>
              ))}
            </select>
            <div style={{ height: 10 }} />
            <label style={styles.label}>Add Existing Repo Path</label>
            <input
              style={styles.input}
              value={repositoryPathDraft}
              onChange={(event) => setRepositoryPathDraft(event.target.value)}
              placeholder="/absolute/path/to/repository"
            />
            <div style={styles.inlineButtonRow}>
              <button style={styles.btnGhost} onClick={() => void browseRepositoryPathForPlanner()}>
                Browse Path
              </button>
              <button
                style={styles.btnGhost}
                onClick={() => void registerRepositoryForPlanner()}
                disabled={!repositoryPathDraft.trim()}
              >
                Register Repo
              </button>
              <button
                style={styles.btn}
                onClick={() => void analyzeSelectedRepository()}
                disabled={!selectedRepositoryId || !selectedProductId || isPlannerBusy || !providerId || !modelName}
              >
                Analyze Repo Into Design
              </button>
            </div>
            {!selectedProductId ? (
              <div style={{ ...styles.helper, marginTop: 10 }}>
                Select a product in the Planner toolbar before analyzing a repository.
              </div>
            ) : null}
            {!providerId || !modelName ? (
              <div style={{ ...styles.helper, marginTop: 10 }}>
                Configure a planner model first. Repository reverse engineering depends on the selected LLM.
              </div>
            ) : null}
            {repoAnalysisMessage ? <div style={{ ...styles.success, marginTop: 10 }}>{repoAnalysisMessage}</div> : null}
            {repoAnalysisError ? <div style={{ ...styles.error, marginTop: 10 }}>{repoAnalysisError}</div> : null}
          </div>
        </div>
      ) : null}
    </div>
  );
}
