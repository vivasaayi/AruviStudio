import { useMemo } from "react";
import { useQuery } from "@tanstack/react-query";

import {
  listModelDefinitions,
  listProductAreas,
  listProducts,
  listProviders,
  listRepositories,
  listWorkItemsPage,
} from "../../../lib/tauri";
import type { ProductArea } from "../../../lib/types";
import {
  PLANNER_WORK_ITEM_PAGE_SIZE,
  buildPlannerComposerScopeChips,
  buildPlannerModelPickerOptions,
  buildDraftValidation,
  buildPlannerStatusSummary,
  buildProductAreaOnlyTree,
  buildSuggestedPrompts,
  findLatestAssistantMessage,
  findLatestDraftPlan,
  findRelevantPlanActions,
  findTreeNodeById,
  findTreeNodePath,
  getAllowedDraftChildTypes,
  resolvePlannerSpeechModelSelection,
  type PendingPlan,
  type PlannerMessage,
  type PlannerTreeNode,
  type ResolverContext,
} from "../lib/plannerPageModel";

type PlannerPageViewModelInput = {
  activeProductId: string | null;
  activeProductAreaId: string | null;
  activeCapabilityId: string | null;
  activeWorkItemId: string | null;
  selectedDraftNodeId: string | null;
  expandedDraftNodeIds: string[];
  draftTreeNodes: PlannerTreeNode[];
  messages: PlannerMessage[];
  pendingPlan: PendingPlan | null;
  voiceActivity: string | null;
  pendingVoiceTranscript: string | null;
  reviewVoiceBeforeSend: boolean;
  plannerView: "conversation" | "draft" | "trace";
  windowWidth: number;
  providerId: string;
  modelName: string;
  speechProviderSetting: string;
  speechModelSetting: string;
};

export function usePlannerPageViewModel({
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
}: PlannerPageViewModelInput) {
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
  const latestDraftPlan = useMemo(
    () => findLatestDraftPlan(messages, pendingPlan),
    [messages, pendingPlan],
  );
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
  const latestAssistantMessage = useMemo(
    () => findLatestAssistantMessage(messages),
    [messages],
  );
  const plannerStatusSummary = useMemo(() => buildPlannerStatusSummary({
    voiceActivity,
    pendingVoiceTranscript,
    reviewVoiceBeforeSend,
    draftTreeNodeCount: draftTreeNodes.length,
    draftValidation,
    selectedDraftNode,
    pendingPlan,
    latestAssistantMessage,
  }), [
    draftTreeNodes.length,
    draftValidation,
    latestAssistantMessage,
    pendingPlan,
    pendingVoiceTranscript,
    reviewVoiceBeforeSend,
    selectedDraftNode,
    voiceActivity,
  ]);
  const composerScopeChips = useMemo(() => buildPlannerComposerScopeChips({
    selectedDraftNodeId,
    selectedProductId,
    activeProductAreaId,
    activeCapabilityId,
    activeWorkItemId,
  }), [activeCapabilityId, activeProductAreaId, selectedProductId, activeWorkItemId, selectedDraftNodeId]);
  const modelOptions = useMemo(
    () => models.filter((model) => model.provider_id === providerId && model.enabled),
    [models, providerId],
  );
  const plannerModelPickerOptions = useMemo(
    () => buildPlannerModelPickerOptions(models, providers),
    [models, providers],
  );
  const plannerModelPickerValue = providerId && modelName ? `${providerId}::${modelName}` : "";
  const speechModelSelection = useMemo(() => resolvePlannerSpeechModelSelection({
    models,
    providerId,
    speechProviderSetting,
    speechModelSetting,
  }), [models, providerId, speechModelSetting, speechProviderSetting]);
  const context = useMemo<ResolverContext>(() => ({
    products,
    productTrees,
    workItems,
    activeProductId: selectedProductId,
    activeProductAreaId,
    activeCapabilityId,
    activeWorkItemId,
  }), [activeCapabilityId, activeProductAreaId, selectedProductId, activeWorkItemId, productTrees, products, workItems]);

  return {
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
    activeProductName: selectedProduct?.name ?? null,
  };
}
