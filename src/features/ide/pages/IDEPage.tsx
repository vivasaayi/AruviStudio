import React, { useEffect, useRef, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import Editor from "@monaco-editor/react";
import { TabBar } from "../../../app/layout/TabBar";
import { ScopeBreadcrumb } from "../../../app/layout/ScopeBreadcrumb";
import { useEditorStore } from "../../../state/editorStore";
import { useWorkspaceStore } from "../../../state/workspaceStore";
import { IDECopilotPanel } from "../components/IDECopilotPanel";
import { IDEFileTreeNode } from "../components/IDEFileTreeNode";
import {
  attachRepository,
  applyRepositoryPatch,
  browseForRepositoryPath,
  createLocalWorkspace,
  getCapability,
  listProductAreas,
  listModelDefinitions,
  listProducts,
  listProviders,
  listRepositories,
  listRepositoryTree,
  readRepositoryFile,
  registerRepository,
  revealInFinder,
  resolveRepositoryForScope,
  resolveRepositoryForWorkItem,
  startModelChatStream,
  writeRepositoryFile,
} from "../../../lib/tauri";
import {
  detectLanguage,
  filterTreeNodes,
  normalizePath,
  parsePatchProposal,
  truncateForContext,
  type CopilotMode,
  type CopilotPatchProposal,
  type LocalChatMessage,
} from "../lib/idePageModel";
import { styles } from "../lib/idePageStyles";

export function IDEPage() {
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
  const [copilotError, setCopilotError] = useState<string | null>(null);
  const [copilotProviderId, setCopilotProviderId] = useState("");
  const [copilotModelName, setCopilotModelName] = useState("");
  const [copilotTemp, setCopilotTemp] = useState("0.2");
  const [copilotMaxTokens, setCopilotMaxTokens] = useState("4096");
  const [copilotSystemPrompt, setCopilotSystemPrompt] = useState(
    "You are Aruvi Copilot. Give precise coding guidance and patch-quality outputs.",
  );
  const [copilotDraft, setCopilotDraft] = useState("");
  const [copilotMessages, setCopilotMessages] = useState<LocalChatMessage[]>([]);
  const [isCopilotSending, setIsCopilotSending] = useState(false);
  const [copilotMode, setCopilotMode] = useState<CopilotMode>("chat");
  const [includeActiveFileContext, setIncludeActiveFileContext] = useState(true);
  const [contextBudgetChars, setContextBudgetChars] = useState("12000");
  const [patchProposal, setPatchProposal] = useState<CopilotPatchProposal | null>(null);
  const [isApplyingProposal, setIsApplyingProposal] = useState(false);
  const copilotSessionIdRef = useRef(crypto.randomUUID());

  const { data: repositories = [] } = useQuery({
    queryKey: ["repositories"],
    queryFn: listRepositories,
  });
  const { data: products = [], isLoading: productsLoading } = useQuery({ queryKey: ["products"], queryFn: listProducts });
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

  useEffect(() => {
    if (!copilotProviderId && providers.length > 0) {
      setCopilotProviderId(providers[0].id);
    }
  }, [copilotProviderId, providers]);

  const modelOptions = models.filter((model) => model.provider_id === copilotProviderId && model.enabled);

  useEffect(() => {
    if (!copilotProviderId) {
      return;
    }
    if (!copilotModelName || !modelOptions.some((entry) => entry.name === copilotModelName)) {
      setCopilotModelName(modelOptions[0]?.name ?? "");
    }
  }, [copilotModelName, copilotProviderId, modelOptions]);

  const filteredTree = filterTreeNodes(repositoryTree, deferredTreeFilter);
  const selectedRepository = repositories.find((repo) => repo.id === selectedRepoId) ?? null;
  const rawActiveFile = openFiles.find((entry) => entry.id === activeFileId) ?? null;
  const activeFileRepositoryId = rawActiveFile?.id.split(":")[0] ?? null;
  const activeFile = activeFileRepositoryId === selectedRepoId ? rawActiveFile : null;
  const activeProduct = products.find((product) => product.id === selectedProductId) ?? null;
  const activeProductArea = activeProductAreas.find((productArea) => productArea.id === activeProductAreaId) ?? null;

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
    setCopilotMessages([]);
    setPatchProposal(null);
    setCopilotError(null);
    copilotSessionIdRef.current = crypto.randomUUID();
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

  const sendCopilot = async () => {
    setCopilotError(null);
    const draft = copilotDraft.trim();
    if (!draft) {
      return;
    }
    if (!copilotProviderId || !copilotModelName) {
      setCopilotError("Select provider and model for Aruvi Copilot.");
      return;
    }

    const userMessage: LocalChatMessage = { id: crypto.randomUUID(), role: "user", content: draft };
    const assistantMessageId = crypto.randomUUID();
    const assistantPlaceholder: LocalChatMessage = {
      id: assistantMessageId,
      role: "assistant",
      content: "",
    };
    setCopilotMessages((current) => [...current, userMessage, assistantPlaceholder]);
    setCopilotDraft("");
    setIsCopilotSending(true);
    if (copilotMode === "patch") {
      setPatchProposal(null);
    }

    const contextLimit = Number.parseInt(contextBudgetChars, 10);
    const maxContextChars = Number.isFinite(contextLimit) && contextLimit > 1000 ? contextLimit : 12000;
    const activeFileContext =
      includeActiveFileContext && activeFile
        ? `Active file context\nPath: ${activeFile.path}\nLanguage: ${activeFile.language}\n\n${truncateForContext(
            activeFile.content,
            maxContextChars,
          )}`
        : null;

    const patchSystemInstruction =
      copilotMode === "patch"
        ? `Return ONLY JSON with this schema:
{
  "type": "patch_proposal",
  "summary": "short summary",
  "patches": [
    {
      "path": "relative/path/from/repo/root",
      "base_sha256": null,
      "description": "why this patch exists",
      "patch": "@@ -oldStart,oldCount +newStart,newCount @@\\n context/removals/additions ..."
    }
  ]
}
Rules:
- No markdown, no prose outside JSON.
- Use unified diff hunks in "patch" only.
- Keep patch list minimal and precise.`
        : null;

    let unlistenChunk: UnlistenFn | null = null;
    let unlistenDone: UnlistenFn | null = null;
    let unlistenError: UnlistenFn | null = null;
    let streamId: string | null = null;

    const cleanup = () => {
      if (unlistenChunk) {
        unlistenChunk();
      }
      if (unlistenDone) {
        unlistenDone();
      }
      if (unlistenError) {
        unlistenError();
      }
      unlistenChunk = null;
      unlistenDone = null;
      unlistenError = null;
    };

    try {
      unlistenChunk = await listen<{ stream_id: string; delta: string }>("chat_stream_chunk", (event) => {
        if (!streamId || event.payload.stream_id !== streamId) {
          return;
        }
        setCopilotMessages((current) =>
          current.map((message) =>
            message.id === assistantMessageId
              ? { ...message, content: `${message.content}${event.payload.delta}` }
              : message,
          ),
        );
      });

      unlistenDone = await listen<{ stream_id: string }>("chat_stream_done", (event) => {
        if (!streamId || event.payload.stream_id !== streamId) {
          return;
        }
        if (copilotMode === "patch") {
          setCopilotMessages((current) => {
            const target = current.find((entry) => entry.id === assistantMessageId);
            if (!target) {
              return current;
            }
            const parsed = parsePatchProposal(target.content);
            if (parsed) {
              setPatchProposal(parsed);
            } else {
              setCopilotError("Copilot response was not a valid patch proposal JSON payload.");
            }
            return current;
          });
        }
        setIsCopilotSending(false);
        cleanup();
      });

      unlistenError = await listen<{ stream_id: string; error: string }>("chat_stream_error", (event) => {
        if (!streamId || event.payload.stream_id !== streamId) {
          return;
        }
        setCopilotError(event.payload.error);
        setIsCopilotSending(false);
        cleanup();
      });

      streamId = await startModelChatStream({
        providerId: copilotProviderId,
        model: copilotModelName,
        messages: [
          { role: "system", content: copilotSystemPrompt.trim() || "You are Aruvi Copilot." },
          ...(patchSystemInstruction ? [{ role: "system" as const, content: patchSystemInstruction }] : []),
          ...(activeFileContext ? [{ role: "system" as const, content: activeFileContext }] : []),
          ...[...copilotMessages, userMessage].map(({ role, content }) => ({ role, content })),
        ],
        temperature: Number.isFinite(Number(copilotTemp)) ? Number(copilotTemp) : 0.2,
        maxTokens: Number.isFinite(Number(copilotMaxTokens)) ? Number(copilotMaxTokens) : 4096,
        sourceKind: "desktop_ide",
        sourceId: copilotSessionIdRef.current,
        sourceLabel: "Desktop IDE",
      });

      window.setTimeout(() => {
        if (streamId) {
          setIsCopilotSending(false);
          cleanup();
        }
      }, 180000);
    } catch (error) {
      setCopilotError(String(error));
      setCopilotMessages((current) => current.filter((entry) => entry.id !== assistantMessageId));
      cleanup();
      setIsCopilotSending(false);
    }
  };

  const applyPatchProposal = async (proposal: CopilotPatchProposal) => {
    if (!selectedRepoId) {
      setCopilotError("Select a workspace before applying a patch proposal.");
      return;
    }
    setCopilotError(null);
    setIsApplyingProposal(true);
    try {
      for (const patch of proposal.patches) {
        await applyRepositoryPatch({
          repositoryId: selectedRepoId,
          relativePath: patch.path,
          patch: patch.patch,
          baseSha256: patch.base_sha256 ?? undefined,
        });
        const openedFileId = `${selectedRepoId}:${patch.path}`;
        const existing = openFiles.find((entry) => entry.id === openedFileId);
        if (existing) {
          const refreshed = await readRepositoryFile({
            repositoryId: selectedRepoId,
            relativePath: patch.path,
          });
          replaceFileContent(openedFileId, refreshed);
          markFileSaved(openedFileId);
        }
      }
      await refetchTree();
    } catch (error) {
      setCopilotError(String(error));
    } finally {
      setIsApplyingProposal(false);
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

  return (
    <div style={styles.page}>
      <div style={styles.header}>
        <div style={styles.headerTop}>
          <div>
            <h1 style={styles.title}>IDE Workspace</h1>
            <div style={styles.subtitle}>
              Product-scoped workspace browser, code editor, and Aruvi Copilot.
            </div>
          </div>
          <div style={styles.productPicker}>
            <label style={styles.label}>Product</label>
            <select
              style={styles.select}
              value={selectedProductId ?? ""}
              onChange={(event) => handleSelectProduct(event.target.value)}
            >
              <option value="">Select product</option>
              {products.map((product) => (
                <option key={product.id} value={product.id}>
                  {product.name}
                </option>
              ))}
            </select>
          </div>
        </div>
        <ScopeBreadcrumb
          label="Product Scope"
          productName={activeProduct?.name}
          productAreaName={activeProductArea?.name}
          capabilityName={activeCapability?.name}
        />
      </div>

      <div style={styles.workspace}>
        <div style={styles.panel}>
          <div style={styles.panelHeader}>
            <div style={styles.panelTitle}>Workspace</div>
            <div style={styles.controlRow}>
              <button style={styles.buttonGhost} onClick={() => void refetchTree()} disabled={!selectedRepoId}>
                Refresh
              </button>
              {selectedRepository && (
                <button
                  style={styles.buttonGhost}
                  onClick={() => void revealInFinder(selectedRepository.local_path).catch((error) => setFileError(String(error)))}
                >
                  Reveal in Finder
                </button>
              )}
              <button style={styles.button} onClick={() => void openFolder()} disabled={!selectedProductId}>
                Attach Folder
              </button>
            </div>
          </div>
          <div style={styles.leftBody}>
            <input
              style={styles.input}
              value={treeFilter}
              onChange={(event) => setTreeFilter(event.target.value)}
              placeholder="Filter files..."
              disabled={!selectedRepository}
            />
            {!selectedProductId ? (
              <div style={styles.status}>Select a product before opening an IDE workspace.</div>
            ) : !selectedRepository ? (
              <>
                <div style={styles.status}>
                  {workItemResolvedRepo || scopeResolvedRepo
                    ? "Resolving workspace for the selected product..."
                    : "No workspace is attached to this product yet. Create one here, or attach an existing folder."}
                </div>
                {!workItemResolvedRepo && !scopeResolvedRepo ? (
                  <button style={{ ...styles.button, alignSelf: "flex-start" }} onClick={() => void createWorkspace()}>
                    Create Workspace
                  </button>
                ) : null}
              </>
            ) : (
              <div style={styles.status}>
                <strong>{selectedRepository.name}</strong>
                <br />
                {selectedRepository.local_path}
              </div>
            )}
            {selectedRepository && isTreeRefreshing && <div style={styles.status}>Refreshing file tree...</div>}
            {selectedRepository && filteredTree.length === 0 ? (
              <div style={styles.status}>No files match the current filter.</div>
            ) : selectedRepository ? (
              filteredTree.map((node) => (
                <IDEFileTreeNode
                  key={node.relative_path}
                  node={node}
                  expandedDirs={expandedDirs}
                  activeFilePath={activeFile?.path ?? null}
                  onToggleDirectory={toggleDirectory}
                  onOpenFile={(relativePath) => void openRepositoryFile(relativePath)}
                />
              ))
            ) : (
              null
            )}
            {fileError && <div style={styles.error}>{fileError}</div>}
          </div>
        </div>

        <div style={{ ...styles.panel, ...styles.editorPanel }}>
          <div style={styles.editorHeader}>
            <div style={styles.editorPath}>
              {activeFile ? `${activeFile.path}${activeFile.isDirty ? " • unsaved" : ""}` : "No file selected"}
            </div>
            <div style={styles.controlRow}>
              <button style={styles.buttonGhost} onClick={() => void refetchTree()} disabled={!selectedRepoId}>
                Reload Tree
              </button>
              <button
                style={styles.button}
                onClick={() => void saveActiveFile()}
                disabled={!activeFile || isSaving || !activeFileRepositoryId}
              >
                {isSaving ? "Saving..." : "Save (⌘/Ctrl+S)"}
              </button>
            </div>
          </div>
          <TabBar />
          <div style={styles.editorBody}>
            {activeFile ? (
              <Editor
                path={activeFile.path}
                language={activeFile.language}
                value={activeFile.content}
                theme="vs-light"
                onChange={(value) => updateFileContent(activeFile.id, value ?? "")}
                options={{
                  fontSize: 13,
                  fontFamily: "JetBrains Mono, Menlo, Monaco, Consolas, monospace",
                  minimap: { enabled: false },
                  smoothScrolling: true,
                  automaticLayout: true,
                  tabSize: 2,
                  wordWrap: "on",
                }}
              />
            ) : (
              <div style={styles.placeholder}>
                <div>Open a file from the workspace tree.</div>
                <div>Use Aruvi Copilot on the right to validate prompts and responses while editing.</div>
              </div>
            )}
          </div>
        </div>

        <IDECopilotPanel
          providers={providers}
          modelOptions={modelOptions}
          copilotMode={copilotMode}
          setCopilotMode={setCopilotMode}
          copilotProviderId={copilotProviderId}
          setCopilotProviderId={setCopilotProviderId}
          copilotModelName={copilotModelName}
          setCopilotModelName={setCopilotModelName}
          copilotTemp={copilotTemp}
          setCopilotTemp={setCopilotTemp}
          copilotMaxTokens={copilotMaxTokens}
          setCopilotMaxTokens={setCopilotMaxTokens}
          copilotSystemPrompt={copilotSystemPrompt}
          setCopilotSystemPrompt={setCopilotSystemPrompt}
          includeActiveFileContext={includeActiveFileContext}
          setIncludeActiveFileContext={setIncludeActiveFileContext}
          contextBudgetChars={contextBudgetChars}
          setContextBudgetChars={setContextBudgetChars}
          copilotMessages={copilotMessages}
          patchProposal={patchProposal}
          copilotDraft={copilotDraft}
          setCopilotDraft={setCopilotDraft}
          copilotError={copilotError}
          isCopilotSending={isCopilotSending}
          isApplyingProposal={isApplyingProposal}
          onClear={() => {
            setCopilotMessages([]);
            setPatchProposal(null);
            setCopilotError(null);
            copilotSessionIdRef.current = crypto.randomUUID();
          }}
          onSend={() => void sendCopilot()}
          onApplyPatchProposal={(proposal) => void applyPatchProposal(proposal)}
        />
      </div>
    </div>
  );
}
