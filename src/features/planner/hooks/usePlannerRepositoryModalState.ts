import { useEffect, useState } from "react";
import { useMutation, type QueryClient } from "@tanstack/react-query";

import {
  analyzeRepositoryForPlanner,
  browseForRepositoryPath,
  registerRepository,
} from "../../../lib/tauri";
import type { Repository } from "../../../lib/types";
import {
  mapPlannerResponseToMutationResult,
  type PlannerMutationResult,
} from "../lib/plannerPageModel";

type PlannerRepositoryModalStateInput = {
  queryClient: QueryClient;
  repositories: Repository[];
  sessionId: string | null;
  selectedDraftNodeId: string | null;
  selectedProductId: string | null;
  isPlannerBusy: boolean;
  onAnalysisSuccess: (result: PlannerMutationResult) => void;
};

export function usePlannerRepositoryModalState({
  queryClient,
  repositories,
  sessionId,
  selectedDraftNodeId,
  selectedProductId,
  isPlannerBusy,
  onAnalysisSuccess,
}: PlannerRepositoryModalStateInput) {
  const [showRepoModal, setShowRepoModal] = useState(false);
  const [selectedRepositoryId, setSelectedRepositoryId] = useState("");
  const [repositoryPathDraft, setRepositoryPathDraft] = useState("");
  const [repoAnalysisMessage, setRepoAnalysisMessage] = useState<string | null>(null);
  const [repoAnalysisError, setRepoAnalysisError] = useState<string | null>(null);

  useEffect(() => {
    if (!selectedRepositoryId && repositories.length > 0) {
      setSelectedRepositoryId(repositories[0].id);
    }
  }, [repositories, selectedRepositoryId]);

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
    onSuccess: onAnalysisSuccess,
    onError: (error) => {
      setRepoAnalysisError(error instanceof Error ? error.message : String(error));
      setRepoAnalysisMessage(null);
    },
  });

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
    if (!selectedRepositoryId || !selectedProductId || isPlannerBusy || repositoryAnalysisMutation.isPending) {
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

  return {
    showRepoModal,
    setShowRepoModal,
    selectedRepositoryId,
    setSelectedRepositoryId,
    repositoryPathDraft,
    setRepositoryPathDraft,
    repoAnalysisMessage,
    repoAnalysisError,
    isRepositoryAnalysisPending: repositoryAnalysisMutation.isPending,
    browseRepositoryPathForPlanner,
    registerRepositoryForPlanner,
    analyzeSelectedRepository,
  };
}
