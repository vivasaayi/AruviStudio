import type { MutableRefObject } from "react";
import type { QueryClient } from "@tanstack/react-query";

import type { Repository } from "../../../lib/types";
import type { PlannerMutationResult } from "../lib/plannerPageModel";
import { usePlannerRepositoryModalState } from "./usePlannerRepositoryModalState";

type PlannerRepositoryAnalysisInput = {
  draftEditMutation: { isPending: boolean };
  handlePlannerMutationSuccess: (result: PlannerMutationResult) => void;
  isVoiceCaptureBusy: boolean;
  plannerBusyRef: MutableRefObject<boolean>;
  processMutation: { isPending: boolean };
  queryClient: QueryClient;
  repositories: Repository[];
  selectedDraftNodeId: string | null;
  selectedProductId: string | null;
  sessionId: string | null;
};

export function usePlannerRepositoryAnalysis({
  draftEditMutation,
  handlePlannerMutationSuccess,
  isVoiceCaptureBusy,
  plannerBusyRef,
  processMutation,
  queryClient,
  repositories,
  selectedDraftNodeId,
  selectedProductId,
  sessionId,
}: PlannerRepositoryAnalysisInput) {
  const basePlannerBusy =
    processMutation.isPending ||
    draftEditMutation.isPending ||
    isVoiceCaptureBusy;
  const repositoryModalState = usePlannerRepositoryModalState({
    queryClient,
    repositories,
    sessionId,
    selectedDraftNodeId,
    selectedProductId,
    isPlannerBusy: basePlannerBusy,
    onAnalysisSuccess: handlePlannerMutationSuccess,
  });
  const isPlannerBusy = basePlannerBusy || repositoryModalState.isRepositoryAnalysisPending;
  plannerBusyRef.current = isPlannerBusy;

  return {
    ...repositoryModalState,
    isPlannerBusy,
  };
}
