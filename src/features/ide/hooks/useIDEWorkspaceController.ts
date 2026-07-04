import React, { useEffect, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import {
  attachRepository,
  browseForRepositoryPath,
  createLocalWorkspace,
  getCapability,
  listModelDefinitions,
  listProductAreas,
  listProducts,
  listProviders,
  listRepositories,
  listRepositoryTree,
  readRepositoryFile,
  registerRepository,
  resolveRepositoryForScope,
  resolveRepositoryForWorkItem,
  writeRepositoryFile,
} from "../../../lib/tauri";
import { useEditorStore } from "../../../state/editorStore";
import { useWorkspaceStore } from "../../../state/workspaceStore";
import { useIDECopilot } from "./useIDECopilot";
import {
  detectLanguage,
  filterTreeNodes,
  normalizePath,
} from "../lib/idePageModel";

export function useIDEWorkspaceController() {
  const queryClient = useQueryClient();
  const {
    openFiles,
    activeFileId,
    openFile: openFileInEditor,
    setActiveFile,
    updateFileContent,
    replaceFileContent,
    markFileSaved,
  } = useEditorStore();
  const {
    activeProductId,
    activeProductAreaId,
    activeCapabilityId,
    activeWorkItemId,
    setActiveProduct,
    setActiveRepo,
  } = useWorkspaceStore();

  const [selectedRepoId, setSelectedRepoId] = useState<string>("");
  const [treeFilter, setTreeFilter] = useState("");
  const deferredTreeFilter = React.useDeferredValue(treeFilter);
  const [expandedDirs, setExpandedDirs] = useState<Record<string, boolean>>({});
  const [isSaving, setIsSaving] = useState(false);
  const [fileError, setFileError] = useState<string | null>(null);
  const { data: repositories = [] } = useQuery({
    queryKey: ["repositories"],
    queryFn: listRepositories,
  });
  const { data: products = [], isLoading: productsLoading } = useQuery({
    queryKey: ["products"],
    queryFn: listProducts,
  });
  const selectedProductId = products.some((product) => product.id === activeProductId)
    ? activeProductId
    : null;

  useEffect(() => {
    if (productsLoading || activeProductId) {
      return;
    }
    if (products[0]) {
      setActiveProduct(products[0].id);
    }
  }, [activeProductId, products, productsLoading, setActiveProduct]);

  const { data: activeProductAreas = [] } = useQuery({
    queryKey: ["ideProductAreas", selectedProductId],
    queryFn: () => listProductAreas(selectedProductId!),
    enabled: !!selectedProductId,
  });
  const { data: activeCapability = null } = useQuery({
    queryKey: ["ideCapability", activeCapabilityId],
    queryFn: () => getCapability(activeCapabilityId!),
    enabled: !!activeCapabilityId,
  });
  const { data: providers = [] } = useQuery({ queryKey: ["providers"], queryFn: listProviders });
  const { data: models = [] } = useQuery({ queryKey: ["model-definitions"], queryFn: listModelDefinitions });
  const { data: scopeResolvedRepo } = useQuery({
    queryKey: ["ideScopeRepo", selectedProductId, activeProductAreaId],
    queryFn: () => resolveRepositoryForScope({ productId: selectedProductId, productAreaId: activeProductAreaId }),
    enabled: !!selectedProductId || !!activeProductAreaId,
    staleTime: 30000,
  });
  const { data: workItemResolvedRepo } = useQuery({
    queryKey: ["ideWorkItemRepo", activeWorkItemId],
    queryFn: () => resolveRepositoryForWorkItem(activeWorkItemId!),
    enabled: !!activeWorkItemId,
    staleTime: 30000,
  });

  const { data: repositoryTree = [], isFetching: isTreeRefreshing, refetch: refetchTree } = useQuery({
    queryKey: ["ideRepositoryTree", selectedRepoId],
    queryFn: () => listRepositoryTree({ repositoryId: selectedRepoId, includeHidden: false, maxDepth: 12 }),
    enabled: !!selectedRepoId,
    staleTime: 30000,
  });

  useEffect(() => {
    const preferredRepoId =
      workItemResolvedRepo?.id ??
      scopeResolvedRepo?.id ??
      null;

    if (!preferredRepoId || !selectedProductId) {
      if (selectedRepoId) {
        React.startTransition(() => {
          setSelectedRepoId("");
          setActiveRepo(null);
          setExpandedDirs({});
          setFileError(null);
        });
      }
      return;
    }

    const preferredExists = repositories.some((repo) => repo.id === preferredRepoId);
    if (!preferredExists || selectedRepoId === preferredRepoId) {
      return;
    }

    React.startTransition(() => {
      setSelectedRepoId(preferredRepoId);
      setActiveRepo(preferredRepoId);
      setExpandedDirs({});
      setFileError(null);
    });
  }, [repositories, scopeResolvedRepo?.id, selectedProductId, selectedRepoId, setActiveRepo, workItemResolvedRepo?.id]);

  const filteredTree = filterTreeNodes(repositoryTree, deferredTreeFilter);
  const selectedRepository = repositories.find((repo) => repo.id === selectedRepoId) ?? null;
  const rawActiveFile = openFiles.find((entry) => entry.id === activeFileId) ?? null;
  const activeFileRepositoryId = rawActiveFile?.id.split(":")[0] ?? null;
  const activeFile = activeFileRepositoryId === selectedRepoId ? rawActiveFile : null;
  const activeProduct = products.find((product) => product.id === selectedProductId) ?? null;
  const activeProductArea = activeProductAreas.find((productArea) => productArea.id === activeProductAreaId) ?? null;

  const {
    modelOptions,
    copilotMode,
    setCopilotMode,
    copilotProviderId,
    setCopilotProviderId,
    copilotModelName,
    setCopilotModelName,
    copilotTemp,
    setCopilotTemp,
    copilotMaxTokens,
    setCopilotMaxTokens,
    copilotSystemPrompt,
    setCopilotSystemPrompt,
    includeActiveFileContext,
    setIncludeActiveFileContext,
    contextBudgetChars,
    setContextBudgetChars,
    copilotMessages,
    patchProposal,
    copilotDraft,
    setCopilotDraft,
    copilotError,
    isCopilotSending,
    isApplyingProposal,
    resetCopilot,
    sendCopilot,
    applyPatchProposal,
  } = useIDECopilot({
    providers,
    models,
    activeFile,
    selectedRepoId,
    openFiles,
    replaceFileContent,
    markFileSaved,
    refetchTree,
  });

  const handleSelectRepository = (repositoryId: string) => {
    setSelectedRepoId(repositoryId);
    setActiveRepo(repositoryId || null);
    setExpandedDirs({});
    setFileError(null);
  };

  const handleSelectProduct = (productId: string) => {
    setActiveProduct(productId || null);
    setSelectedRepoId("");
    setActiveRepo(null);
    setExpandedDirs({});
    setTreeFilter("");
    setFileError(null);
    resetCopilot();
  };

  const toggleDirectory = (relativePath: string) => {
    setExpandedDirs((current) => ({ ...current, [relativePath]: !(current[relativePath] ?? false) }));
  };

  const openRepositoryFile = async (relativePath: string) => {
    if (!selectedRepoId) {
      setFileError("Select a workspace first.");
      return;
    }
    const fileId = `${selectedRepoId}:${relativePath}`;
    const existing = openFiles.find((entry) => entry.id === fileId);
    if (existing) {
      setActiveFile(fileId);
      return;
    }
    setFileError(null);
    try {
      const content = await readRepositoryFile({ repositoryId: selectedRepoId, relativePath });
      openFileInEditor({
        id: fileId,
        path: relativePath,
        name: relativePath.split("/").pop() ?? relativePath,
        content,
        language: detectLanguage(relativePath),
      });
    } catch (error) {
      setFileError(String(error));
    }
  };

  const saveActiveFile = async () => {
    if (!activeFile || !activeFileRepositoryId) {
      return;
    }
    setFileError(null);
    setIsSaving(true);
    try {
      await writeRepositoryFile({
        repositoryId: activeFileRepositoryId,
        relativePath: activeFile.path,
        content: activeFile.content,
      });
      markFileSaved(activeFile.id);
      if (activeFileRepositoryId === selectedRepoId) {
        await refetchTree();
      }
    } catch (error) {
      setFileError(String(error));
    } finally {
      setIsSaving(false);
    }
  };

  const openFolder = async () => {
    setFileError(null);
    if (!selectedProductId) {
      setFileError("Select a product before attaching a workspace.");
      return;
    }
    try {
      const selectedPath = await browseForRepositoryPath();
      if (!selectedPath) {
        return;
      }
      const existing = repositories.find((repo) => normalizePath(repo.local_path) === normalizePath(selectedPath));
      if (existing) {
        await attachRepository({
          scopeType: "product",
          scopeId: selectedProductId,
          repositoryId: existing.id,
          isDefault: true,
        });
        await queryClient.invalidateQueries({ queryKey: ["ideScopeRepo"] });
        handleSelectRepository(existing.id);
        return;
      }
      const created = await registerRepository({
        name: selectedPath.split("/").filter(Boolean).pop() ?? "workspace",
        localPath: selectedPath,
        remoteUrl: "",
        defaultBranch: "main",
      });
      await attachRepository({
        scopeType: "product",
        scopeId: selectedProductId,
        repositoryId: created.id,
        isDefault: true,
      });
      await queryClient.invalidateQueries({ queryKey: ["repositories"] });
      await queryClient.invalidateQueries({ queryKey: ["ideScopeRepo"] });
      handleSelectRepository(created.id);
    } catch (error) {
      setFileError(String(error));
    }
  };

  const createWorkspace = async () => {
    setFileError(null);
    if (!selectedProductId) {
      setFileError("Select a product before creating a workspace.");
      return;
    }
    try {
      const provisioned = await createLocalWorkspace({
        productId: selectedProductId,
        productAreaId: activeProductAreaId,
        workItemId: activeWorkItemId,
      });
      await queryClient.invalidateQueries({ queryKey: ["repositories"] });
      await queryClient.invalidateQueries({ queryKey: ["ideScopeRepo"] });
      await queryClient.invalidateQueries({ queryKey: ["ideWorkItemRepo"] });
      handleSelectRepository(provisioned.repository.id);
    } catch (error) {
      setFileError(String(error));
    }
  };

  const onSaveShortcut = React.useEffectEvent((event: KeyboardEvent) => {
    if (!(event.metaKey || event.ctrlKey) || event.key.toLowerCase() !== "s") {
      return;
    }
    event.preventDefault();
    void saveActiveFile();
  });

  useEffect(() => {
    const listener = (event: KeyboardEvent) => onSaveShortcut(event);
    window.addEventListener("keydown", listener);
    return () => window.removeEventListener("keydown", listener);
  }, [onSaveShortcut]);

  return {
    activeCapability,
    activeFile,
    activeFileRepositoryId,
    activeProduct,
    activeProductArea,
    contextBudgetChars,
    copilotDraft,
    copilotError,
    copilotMaxTokens,
    copilotMessages,
    copilotMode,
    copilotModelName,
    copilotProviderId,
    copilotSystemPrompt,
    copilotTemp,
    createWorkspace,
    expandedDirs,
    fileError,
    filteredTree,
    handleSelectProduct,
    includeActiveFileContext,
    isApplyingProposal,
    isCopilotSending,
    isSaving,
    isTreeRefreshing,
    modelOptions,
    openFolder,
    openRepositoryFile,
    patchProposal,
    products,
    providers,
    refetchTree,
    resetCopilot,
    saveActiveFile,
    scopeResolvedRepo,
    selectedProductId,
    selectedRepoId,
    selectedRepository,
    sendCopilot,
    setContextBudgetChars,
    setCopilotDraft,
    setCopilotMaxTokens,
    setCopilotMode,
    setCopilotModelName,
    setCopilotProviderId,
    setCopilotSystemPrompt,
    setCopilotTemp,
    setIncludeActiveFileContext,
    setFileError,
    setTreeFilter,
    treeFilter,
    toggleDirectory,
    updateFileContent,
    workItemResolvedRepo,
    applyPatchProposal,
  };
}
