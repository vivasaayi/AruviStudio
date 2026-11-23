import { useEffect, useRef, type Dispatch, type RefObject, type SetStateAction } from "react";
import type { Location, NavigateFunction } from "react-router-dom";

import { createPlannerSession, stagePlannerCatalogNode, updatePlannerSession } from "../../../lib/tauri";
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
type PlannerRouteState = { plannerPrompt?: string; plannerView?: PlannerView; plannerCatalogNode?: { productId: string; nodeType: "product" | "product_area" | "capability" | "work_item"; nodeId: string } | null };

type PlannerPageLifecycleInput = {
  selectedProductId: string | null;
  selectedProduct: Product | null;
  productsPending: boolean;
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
  draft: string;
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
  productsPending,
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
  draft,
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
          if ((location.state as PlannerRouteState | null)?.plannerCatalogNode && window.location.pathname === location.pathname && window.history.state?.key === location.key) {
            navigate(location.pathname, { replace: true, state: null });
          }
        }
      }
    };
    void ensureSession();
    return () => {
      cancelled = true;
    };
  }, [location.key, location.pathname, location.state, modelName, navigate, providerId, sessionId, setMessages, setSessionId]);

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

  const draftRef = useRef(draft);
  draftRef.current = draft;
  const routeState = location.state as PlannerRouteState | null;
  const routeCatalogNode = routeState?.plannerCatalogNode;
  const isRouteStaging = Boolean(routeCatalogNode);
  useEffect(() => {
    const state = location.state as PlannerRouteState | null;
    const prompt = state?.plannerPrompt?.trim();
    if (!prompt) return;
    const context = state?.plannerCatalogNode;
    if (!context) {
      if (consumedRoutePromptRef.current === prompt) return;
      consumedRoutePromptRef.current = prompt;
      setDraft(prompt);
      setPlannerView(state?.plannerView ?? "conversation");
      composerRef.current?.focus();
      navigate(location.pathname, { replace: true, state: null });
      return;
    }
    if (productsPending) return;
    if (selectedProductId !== context.productId) {
      setMessages((current) => [...current, { id: makeId(), role: "assistant", content: "The selected product is no longer available; return to Products and try again.", meta: "Planner error", kind: "error" }]);
      navigate(location.pathname, { replace: true, state: null });
      return;
    }
    if (!sessionId) return;
    let cancelled = false;
    const originalDraft = draftRef.current;
    void stagePlannerCatalogNode({ sessionId, ...context }).then((response) => {
      if (cancelled || window.location.pathname !== location.pathname || window.history.state?.key !== location.key) return;
      if (response.selected_draft_node_id !== context.nodeId || !response.draft_tree_nodes) {
        throw new Error("Planner did not stage the selected item. Try again.");
      }
      setDraftTreeNodes(response.draft_tree_nodes as unknown as PlannerTreeNode[]);
      setSelectedDraftNodeId(context.nodeId);
      if (!originalDraft.trim() && draftRef.current === originalDraft) {
        setDraft(prompt);
      } else {
        setMessages((current) => [...current, { id: makeId(), role: "assistant", content: "The selected item is staged. Your unsent request was preserved.", meta: "Planner", kind: "text" }]);
      }
      setPlannerView(state?.plannerView ?? "conversation");
      composerRef.current?.focus();
      navigate(location.pathname, { replace: true, state: null });
    }).catch((error: unknown) => {
      if (cancelled || window.location.pathname !== location.pathname || window.history.state?.key !== location.key) return;
      setMessages((current) => [...current, { id: makeId(), role: "assistant", content: error instanceof Error ? error.message : String(error), meta: "Planner error", kind: "error" }]);
      setSelectedDraftNodeId(null);
      navigate(location.pathname, { replace: true, state: null });
    });
    return () => { cancelled = true; };
  }, [composerRef, consumedRoutePromptRef, location.key, location.pathname, location.state, navigate, productsPending, selectedProductId, sessionId, setDraft, setDraftTreeNodes, setMessages, setPlannerView, setSelectedDraftNodeId]);

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
  return { isRouteStaging };
}
