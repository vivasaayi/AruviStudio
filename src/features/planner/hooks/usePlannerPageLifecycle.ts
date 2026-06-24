import { useEffect, type Dispatch, type RefObject, type SetStateAction } from "react";
import type { Location, NavigateFunction } from "react-router-dom";

import { createPlannerSession, updatePlannerSession } from "../../../lib/tauri";
import type { ModelDefinition, ModelProvider, PlannerTraceEvent, Product } from "../../../lib/types";
import {
  DEFAULT_ASSISTANT_OPENING,
  collectTreeNodeIds,
  makeId,
  type PendingPlan,
  type PlannerMessage,
  type PlannerTreeNode,
} from "../lib/plannerPageModel";

type PlannerView = "conversation" | "draft" | "trace";

type PlannerPageLifecycleInput = {
  selectedProductId: string | null;
  selectedProduct: Product | null;
  sessionId: string | null;
  setSessionId: Dispatch<SetStateAction<string | null>>;
  draftTreeNodes: PlannerTreeNode[];
  setDraftTreeNodes: Dispatch<SetStateAction<PlannerTreeNode[]>>;
  selectedDraftNodeId: string | null;
  setSelectedDraftNodeId: Dispatch<SetStateAction<string | null>>;
  setExpandedDraftNodeIds: Dispatch<SetStateAction<string[]>>;
  setLatestTraceEvents: Dispatch<SetStateAction<PlannerTraceEvent[]>>;
  pendingPlan: PendingPlan | null;
  setPendingPlan: Dispatch<SetStateAction<PendingPlan | null>>;
  plannerView: PlannerView;
  setPlannerView: Dispatch<SetStateAction<PlannerView>>;
  setMessages: Dispatch<SetStateAction<PlannerMessage[]>>;
  providerId: string;
  setProviderId: Dispatch<SetStateAction<string>>;
  providers: ModelProvider[];
  modelName: string;
  setModelName: Dispatch<SetStateAction<string>>;
  modelOptions: ModelDefinition[];
  transcriptRef: RefObject<HTMLDivElement | null>;
  messages: PlannerMessage[];
  location: Location;
  navigate: NavigateFunction;
  setDraft: Dispatch<SetStateAction<string>>;
  composerRef: RefObject<HTMLTextAreaElement | null>;
  consumedRoutePromptRef: RefObject<string | null>;
  activePlannerProductRef: RefObject<string | null>;
  isCompactScreen: boolean;
  setShowCompactTools: Dispatch<SetStateAction<boolean>>;
};

export function usePlannerPageLifecycle({
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
}: PlannerPageLifecycleInput) {
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
  }, [
    activePlannerProductRef,
    draftTreeNodes.length,
    pendingPlan,
    selectedProduct,
    selectedProductId,
    sessionId,
    setDraftTreeNodes,
    setExpandedDraftNodeIds,
    setLatestTraceEvents,
    setMessages,
    setPendingPlan,
    setPlannerView,
    setSelectedDraftNodeId,
    setSessionId,
  ]);

  useEffect(() => {
    if (!providerId && providers.length > 0) {
      setProviderId(providers[0].id);
    }
  }, [providerId, providers, setProviderId]);

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
  }, [modelName, providerId, sessionId, setMessages, setSessionId]);

  useEffect(() => {
    if (!providerId) {
      return;
    }
    if (!modelName || !modelOptions.some((entry) => entry.name === modelName)) {
      setModelName(modelOptions[0]?.name ?? "");
    }
  }, [modelName, modelOptions, providerId, setModelName]);

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
  }, [messages, transcriptRef]);

  useEffect(() => {
    const state = location.state as { plannerPrompt?: string; plannerView?: PlannerView } | null;
    const prompt = state?.plannerPrompt?.trim();
    if (!prompt || consumedRoutePromptRef.current === prompt) {
      return;
    }
    consumedRoutePromptRef.current = prompt;
    setDraft(prompt);
    setPlannerView(state?.plannerView ?? "conversation");
    composerRef.current?.focus();
    navigate(location.pathname, { replace: true, state: null });
  }, [composerRef, consumedRoutePromptRef, location.pathname, location.state, navigate, setDraft, setPlannerView]);

  useEffect(() => {
    if (draftTreeNodes.length === 0 && plannerView === "draft") {
      setPlannerView("conversation");
    }
  }, [draftTreeNodes.length, plannerView, setPlannerView]);

  useEffect(() => {
    if (!isCompactScreen) {
      setShowCompactTools(false);
    }
  }, [isCompactScreen, setShowCompactTools]);

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
  }, [draftTreeNodes, selectedDraftNodeId, setExpandedDraftNodeIds]);
}
