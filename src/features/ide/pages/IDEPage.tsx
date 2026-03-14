import React, { useEffect, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import Editor from "@monaco-editor/react";
import { TabBar } from "../../../app/layout/TabBar";
import { ScopeBreadcrumb } from "../../../app/layout/ScopeBreadcrumb";
import { useEditorStore } from "../../../state/editorStore";
import { useWorkspaceStore } from "../../../state/workspaceStore";
import {
  applyRepositoryPatch,
  browseForRepositoryPath,
  createLocalWorkspace,
  getProductTree,
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
import type { ChatMessagePayload, RepositoryTreeNode } from "../../../lib/types";

type LocalChatMessage = ChatMessagePayload & { id: string };
type CopilotMode = "chat" | "patch";

interface PatchProposalItem {
  path: string;
  patch: string;
  base_sha256?: string | null;
  description?: string;
}

interface CopilotPatchProposal {
  summary: string;
  patches: PatchProposalItem[];
  raw: string;
}

const styles: Record<string, React.CSSProperties> = {
  page: { display: "flex", flexDirection: "column", height: "100%", margin: -12 },
  header: { padding: "10px 12px 8px", borderBottom: "1px solid #2d3138" },
  title: { margin: 0, fontSize: 20, fontWeight: 750, color: "#f1f3f7" },
  subtitle: { marginTop: 4, fontSize: 12, color: "#8f96a3" },
  workspace: { flex: 1, minHeight: 0, display: "grid", gridTemplateColumns: "280px 1fr 360px", gap: 10, padding: 10 },
  panel: { border: "1px solid #2f343d", borderRadius: 12, backgroundColor: "#1f2329", minHeight: 0, display: "flex", flexDirection: "column", overflow: "hidden" },
  panelHeader: { padding: "10px 12px", borderBottom: "1px solid #2b3038", display: "flex", alignItems: "center", justifyContent: "space-between", gap: 8 },
  panelTitle: { fontSize: 12, fontWeight: 800, textTransform: "uppercase" as const, letterSpacing: 1, color: "#a7afbf" },
  controlRow: { display: "flex", gap: 8, alignItems: "center" },
  select: { width: "100%", padding: "8px 10px", backgroundColor: "#121620", border: "1px solid #3a404a", color: "#edf1f8", borderRadius: 8, fontSize: 13 },
  input: { width: "100%", padding: "8px 10px", backgroundColor: "#121620", border: "1px solid #3a404a", color: "#edf1f8", borderRadius: 8, fontSize: 13, boxSizing: "border-box" as const },
  button: { border: "none", backgroundColor: "#0e639c", color: "#ffffff", borderRadius: 8, padding: "7px 10px", fontSize: 12, fontWeight: 700, cursor: "pointer" },
  buttonGhost: { border: "1px solid #434a55", backgroundColor: "#2c3139", color: "#e5e7eb", borderRadius: 8, padding: "7px 10px", fontSize: 12, fontWeight: 700, cursor: "pointer" },
  leftBody: { padding: 10, overflowY: "auto" as const, display: "flex", flexDirection: "column", gap: 8, minHeight: 0 },
  treeNode: { border: "1px solid transparent", borderRadius: 8, padding: "6px 8px", cursor: "pointer", backgroundColor: "#1c2027", color: "#d2d7e0", fontSize: 12 },
  treeNodeActive: { border: "1px solid #0e639c", borderRadius: 8, padding: "6px 8px", cursor: "pointer", backgroundColor: "#1f2a35", color: "#ffffff", fontSize: 12 },
  treeDirRow: { display: "flex", alignItems: "center", gap: 6, fontWeight: 700 },
  treeFileRow: { display: "flex", alignItems: "center", gap: 6 },
  treeMeta: { fontSize: 10, color: "#8f96a3", marginTop: 2 },
  treeChildren: { marginLeft: 14, marginTop: 6, display: "flex", flexDirection: "column", gap: 6, borderLeft: "1px solid #2a2f37", paddingLeft: 8 },
  editorPanel: { display: "flex", flexDirection: "column", minHeight: 0, overflow: "hidden" },
  editorHeader: { padding: "8px 12px", borderBottom: "1px solid #2b3038", display: "flex", alignItems: "center", justifyContent: "space-between", gap: 10, backgroundColor: "#20252d" },
  editorPath: { fontSize: 12, color: "#b9c0cf", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" as const },
  editorBody: { flex: 1, minHeight: 0, backgroundColor: "#1e1e1e" },
  placeholder: { height: "100%", display: "flex", alignItems: "center", justifyContent: "center", flexDirection: "column", color: "#7f8796", fontSize: 13, gap: 6 },
  label: { fontSize: 11, color: "#99a1b1", fontWeight: 600 },
  section: { padding: "10px 12px", borderBottom: "1px solid #2b3038", display: "flex", flexDirection: "column", gap: 8 },
  chatBody: { flex: 1, minHeight: 0, overflowY: "auto" as const, padding: 10, display: "flex", flexDirection: "column", gap: 8, backgroundColor: "#1d2128" },
  bubbleUser: { alignSelf: "flex-end", maxWidth: "85%", backgroundColor: "#0e639c", color: "#fff", borderRadius: 10, padding: "8px 10px", whiteSpace: "pre-wrap" as const, fontSize: 13 },
  bubbleAssistant: { alignSelf: "flex-start", maxWidth: "88%", backgroundColor: "#2b313b", color: "#edf1f8", borderRadius: 10, padding: "8px 10px", whiteSpace: "pre-wrap" as const, fontSize: 13 },
  chatComposer: { borderTop: "1px solid #2b3038", padding: 10, display: "flex", flexDirection: "column", gap: 8 },
  textarea: { width: "100%", minHeight: 90, padding: "9px 10px", backgroundColor: "#121620", border: "1px solid #3a404a", color: "#edf1f8", borderRadius: 8, resize: "vertical" as const, fontSize: 13, boxSizing: "border-box" as const },
  status: { fontSize: 11, color: "#9ea6b6" },
  error: { fontSize: 12, color: "#ff7b72" },
  segmented: { display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 },
  modeButton: { border: "1px solid #3f4550", backgroundColor: "#272c34", color: "#d7dbe3", borderRadius: 8, padding: "6px 8px", fontSize: 12, fontWeight: 700, cursor: "pointer" },
  modeButtonActive: { border: "1px solid #0e639c", backgroundColor: "#18456a", color: "#ffffff", borderRadius: 8, padding: "6px 8px", fontSize: 12, fontWeight: 700, cursor: "pointer" },
  patchPanel: { border: "1px solid #39404a", borderRadius: 8, padding: 8, backgroundColor: "#20252d", display: "flex", flexDirection: "column", gap: 6 },
  patchPath: { fontSize: 12, fontWeight: 700, color: "#eef1f6" },
  patchMeta: { fontSize: 11, color: "#9aa3b4" },
  patchSnippet: { fontSize: 11, fontFamily: "JetBrains Mono, Menlo, Monaco, monospace", backgroundColor: "#171b22", border: "1px solid #2f3540", borderRadius: 6, padding: 6, whiteSpace: "pre-wrap" as const, maxHeight: 110, overflow: "auto" as const },
};

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
  const { activeProductId, activeModuleId, activeCapabilityId, activeWorkItemId, activeRepoId, setActiveRepo } = useWorkspaceStore();
  const activeFile = openFiles.find((entry) => entry.id === activeFileId) ?? null;

  const [selectedRepoId, setSelectedRepoId] = useState<string>(activeRepoId ?? "");
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

  const { data: repositories = [] } = useQuery({
    queryKey: ["repositories"],
    queryFn: listRepositories,
  });
  const { data: products = [] } = useQuery({ queryKey: ["products"], queryFn: listProducts });
  const { data: activeProductTree } = useQuery({
    queryKey: ["ideProductTree", activeProductId],
    queryFn: () => getProductTree(activeProductId!),
    enabled: !!activeProductId,
  });
  const { data: providers = [] } = useQuery({ queryKey: ["providers"], queryFn: listProviders });
  const { data: models = [] } = useQuery({ queryKey: ["model-definitions"], queryFn: listModelDefinitions });
  const { data: scopeResolvedRepo } = useQuery({
    queryKey: ["ideScopeRepo", activeProductId, activeModuleId],
    queryFn: () => resolveRepositoryForScope({ productId: activeProductId, moduleId: activeModuleId }),
    enabled: !!activeProductId || !!activeModuleId,
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
    if (!repositories.length) {
      if (selectedRepoId) {
        setSelectedRepoId("");
      }
      return;
    }
    const hasCurrent = repositories.some((repo) => repo.id === selectedRepoId);
    if (!hasCurrent) {
      const nextId = repositories.some((repo) => repo.id === activeRepoId)
        ? (activeRepoId as string)
        : repositories[0].id;
      React.startTransition(() => {
        setSelectedRepoId(nextId);
        setActiveRepo(nextId);
      });
    }
  }, [activeRepoId, repositories, selectedRepoId, setActiveRepo]);

  useEffect(() => {
    const preferredRepoId =
      workItemResolvedRepo?.id ??
      scopeResolvedRepo?.id ??
      activeRepoId ??
      null;

    if (!preferredRepoId) {
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
  }, [activeRepoId, repositories, scopeResolvedRepo?.id, selectedRepoId, setActiveRepo, workItemResolvedRepo?.id]);

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
  const activeFileRepositoryId = activeFile?.id.split(":")[0] ?? null;
  const activeProduct = products.find((product) => product.id === activeProductId) ?? null;
  const activeModule = activeProductTree?.modules.find((moduleTree) => moduleTree.module.id === activeModuleId)?.module ?? null;
  const activeCapability = React.useMemo(() => {
    if (!activeCapabilityId || !activeProductTree) {
      return null;
    }
    const stack = [...activeProductTree.modules.flatMap((moduleTree) => moduleTree.features)];
    while (stack.length > 0) {
      const current = stack.pop();
      if (!current) continue;
      if (current.capability.id === activeCapabilityId) {
        return current.capability;
      }
      stack.push(...current.children);
    }
    return null;
  }, [activeCapabilityId, activeProductTree]);

  const handleSelectRepository = (repositoryId: string) => {
    setSelectedRepoId(repositoryId);
    setActiveRepo(repositoryId || null);
    setExpandedDirs({});
    setFileError(null);
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
    try {
      const selectedPath = await browseForRepositoryPath();
      if (!selectedPath) {
        return;
      }
      const existing = repositories.find((repo) => normalizePath(repo.local_path) === normalizePath(selectedPath));
      if (existing) {
        handleSelectRepository(existing.id);
        return;
      }
      const created = await registerRepository({
        name: selectedPath.split("/").filter(Boolean).pop() ?? "workspace",
        localPath: selectedPath,
        remoteUrl: "",
        defaultBranch: "main",
      });
      await queryClient.invalidateQueries({ queryKey: ["repositories"] });
      handleSelectRepository(created.id);
    } catch (error) {
      setFileError(String(error));
    }
  };

  const createWorkspace = async () => {
    setFileError(null);
    try {
      const provisioned = await createLocalWorkspace({
        productId: activeProductId,
        moduleId: activeModuleId,
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
        <h1 style={styles.title}>IDE Workspace</h1>
        <div style={styles.subtitle}>
          Lightweight workspace browser + code editor + Aruvi Copilot, backed by the same model stack used in automation.
        </div>
        <ScopeBreadcrumb
          label="Current Scope"
          productName={activeProduct?.name}
          moduleName={activeModule?.name}
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
              <button style={styles.button} onClick={() => void openFolder()}>
                Open Folder
              </button>
            </div>
          </div>
          <div style={styles.leftBody}>
            <select style={styles.select} value={selectedRepoId} onChange={(event) => handleSelectRepository(event.target.value)}>
              <option value="">Select workspace</option>
              {repositories.map((repository) => (
                <option key={repository.id} value={repository.id}>
                  {repository.name}
                </option>
              ))}
            </select>
            <input
              style={styles.input}
              value={treeFilter}
              onChange={(event) => setTreeFilter(event.target.value)}
              placeholder="Filter files..."
            />
            {!selectedRepository ? (
              <>
                <div style={styles.status}>
                  {workItemResolvedRepo || scopeResolvedRepo
                    ? "Resolving workspace for the current scope..."
                    : "No workspace is attached to the current scope yet. Create one here, or open an existing folder."}
                </div>
                {!workItemResolvedRepo && !scopeResolvedRepo ? (
                  <button style={{ ...styles.button, alignSelf: "flex-start" }} onClick={() => void createWorkspace()}>
                    Create Workspace
                  </button>
                ) : null}
              </>
            ) : (
              <div style={styles.status}>{selectedRepository.local_path}</div>
            )}
            {isTreeRefreshing && <div style={styles.status}>Refreshing file tree...</div>}
            {filteredTree.length === 0 ? (
              <div style={styles.status}>No files to display for the current filter.</div>
            ) : (
              filteredTree.map((node) => (
                <TreeNode
                  key={node.relative_path}
                  node={node}
                  expandedDirs={expandedDirs}
                  activeFilePath={activeFile?.path ?? null}
                  onToggleDirectory={toggleDirectory}
                  onOpenFile={(relativePath) => void openRepositoryFile(relativePath)}
                />
              ))
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
                theme="vs-dark"
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

        <div style={styles.panel}>
          <div style={styles.panelHeader}>
            <div style={styles.panelTitle}>Aruvi Copilot</div>
            <button
              style={styles.buttonGhost}
              onClick={() => {
                setCopilotMessages([]);
                setPatchProposal(null);
                setCopilotError(null);
              }}
              disabled={isCopilotSending}
            >
              Clear
            </button>
          </div>
          <div style={styles.section}>
            <div style={styles.segmented}>
              <button
                style={copilotMode === "chat" ? styles.modeButtonActive : styles.modeButton}
                onClick={() => setCopilotMode("chat")}
                disabled={isCopilotSending}
              >
                Chat
              </button>
              <button
                style={copilotMode === "patch" ? styles.modeButtonActive : styles.modeButton}
                onClick={() => setCopilotMode("patch")}
                disabled={isCopilotSending}
              >
                Propose Patch
              </button>
            </div>
            <label style={styles.label}>Provider</label>
            <select style={styles.select} value={copilotProviderId} onChange={(event) => setCopilotProviderId(event.target.value)}>
              <option value="">Select provider</option>
              {providers.map((provider) => (
                <option key={provider.id} value={provider.id}>
                  {provider.name}
                </option>
              ))}
            </select>
            <label style={styles.label}>Model</label>
            <select style={styles.select} value={copilotModelName} onChange={(event) => setCopilotModelName(event.target.value)}>
              <option value="">Select model</option>
              {modelOptions.map((model) => (
                <option key={model.id} value={model.name}>
                  {model.name}
                </option>
              ))}
            </select>
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 }}>
              <div>
                <label style={styles.label}>Temperature</label>
                <input style={styles.input} value={copilotTemp} onChange={(event) => setCopilotTemp(event.target.value)} />
              </div>
              <div>
                <label style={styles.label}>Max Tokens</label>
                <input style={styles.input} value={copilotMaxTokens} onChange={(event) => setCopilotMaxTokens(event.target.value)} />
              </div>
            </div>
            <label style={styles.label}>System Prompt</label>
            <textarea
              style={{ ...styles.textarea, minHeight: 72 }}
              value={copilotSystemPrompt}
              onChange={(event) => setCopilotSystemPrompt(event.target.value)}
            />
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <input
                id="include-file-context"
                type="checkbox"
                checked={includeActiveFileContext}
                onChange={(event) => setIncludeActiveFileContext(event.target.checked)}
              />
              <label style={styles.label} htmlFor="include-file-context">
                Include active file context
              </label>
            </div>
            <label style={styles.label}>Context Budget (chars)</label>
            <input
              style={styles.input}
              value={contextBudgetChars}
              onChange={(event) => setContextBudgetChars(event.target.value)}
            />
          </div>
          <div style={styles.chatBody}>
            {copilotMessages.length === 0 ? (
              <div style={styles.status}>Ask Aruvi Copilot for implementation help, refactors, or review notes.</div>
            ) : (
              copilotMessages.map((message) => (
                <div
                  key={message.id}
                  style={message.role === "user" ? styles.bubbleUser : styles.bubbleAssistant}
                >
                  {message.content}
                </div>
              ))
            )}
          </div>
          {patchProposal && (
            <div style={styles.section}>
              <div style={styles.label}>Patch Proposal</div>
              <div style={styles.status}>{patchProposal.summary}</div>
              {patchProposal.patches.map((patch, index) => (
                <div key={`${patch.path}:${index}`} style={styles.patchPanel}>
                  <div style={styles.patchPath}>{patch.path}</div>
                  <div style={styles.patchMeta}>
                    {patch.description || "No description"}{patch.base_sha256 ? " · base hash guarded" : ""}
                  </div>
                  <div style={styles.patchSnippet}>{truncateForContext(patch.patch, 420)}</div>
                </div>
              ))}
              <button
                style={styles.button}
                onClick={() => void applyPatchProposal(patchProposal)}
                disabled={isApplyingProposal}
              >
                {isApplyingProposal ? "Applying..." : "Apply Proposal"}
              </button>
            </div>
          )}
          <div style={styles.chatComposer}>
            <textarea
              style={styles.textarea}
              value={copilotDraft}
              placeholder={copilotMode === "patch" ? "Describe the exact change; Copilot will return a structured patch proposal..." : "Ask about the selected file or broader repo changes..."}
              onChange={(event) => setCopilotDraft(event.target.value)}
              onKeyDown={(event) => {
                if ((event.metaKey || event.ctrlKey) && event.key === "Enter" && !isCopilotSending) {
                  event.preventDefault();
                  void sendCopilot();
                }
              }}
            />
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: 8 }}>
              <div style={styles.status}>
                {isCopilotSending ? "Streaming response..." : "Cmd/Ctrl + Enter to send"}
              </div>
              <button style={styles.button} onClick={() => void sendCopilot()} disabled={isCopilotSending}>
                {isCopilotSending ? "Sending..." : copilotMode === "patch" ? "Generate Proposal" : "Send"}
              </button>
            </div>
            {copilotError && <div style={styles.error}>{copilotError}</div>}
          </div>
        </div>
      </div>
    </div>
  );
}

function TreeNode(props: {
  node: RepositoryTreeNode;
  expandedDirs: Record<string, boolean>;
  activeFilePath: string | null;
  onToggleDirectory: (relativePath: string) => void;
  onOpenFile: (relativePath: string) => void;
}) {
  const { node, expandedDirs, activeFilePath, onToggleDirectory, onOpenFile } = props;
  const isDirectory = node.node_type === "directory";
  const isExpanded = expandedDirs[node.relative_path] ?? false;
  const isActiveFile = !isDirectory && activeFilePath === node.relative_path;
  const rowStyle = isActiveFile ? styles.treeNodeActive : styles.treeNode;

  return (
    <div>
      <div
        style={rowStyle}
        onClick={() => {
          if (isDirectory) {
            onToggleDirectory(node.relative_path);
            return;
          }
          onOpenFile(node.relative_path);
        }}
      >
        {isDirectory ? (
          <div style={styles.treeDirRow}>
            <span>{isExpanded ? "▾" : "▸"}</span>
            <span>{node.name}</span>
          </div>
        ) : (
          <div style={styles.treeFileRow}>
            <span>•</span>
            <span>{node.name}</span>
          </div>
        )}
        {!isDirectory && node.size_bytes != null && (
          <div style={styles.treeMeta}>{formatBytes(node.size_bytes)}</div>
        )}
      </div>
      {isDirectory && isExpanded && node.children.length > 0 && (
        <div style={styles.treeChildren}>
          {node.children.map((child) => (
            <TreeNode
              key={child.relative_path}
              node={child}
              expandedDirs={expandedDirs}
              activeFilePath={activeFilePath}
              onToggleDirectory={onToggleDirectory}
              onOpenFile={onOpenFile}
            />
          ))}
        </div>
      )}
    </div>
  );
}

function filterTreeNodes(nodes: RepositoryTreeNode[], rawFilter: string): RepositoryTreeNode[] {
  const filter = rawFilter.trim().toLowerCase();
  if (!filter) {
    return nodes;
  }

  const filtered: RepositoryTreeNode[] = [];
  for (const node of nodes) {
    if (node.node_type === "file") {
      if (
        node.name.toLowerCase().includes(filter) ||
        node.relative_path.toLowerCase().includes(filter)
      ) {
        filtered.push(node);
      }
      continue;
    }

    const children = filterTreeNodes(node.children, filter);
    if (
      node.name.toLowerCase().includes(filter) ||
      node.relative_path.toLowerCase().includes(filter) ||
      children.length > 0
    ) {
      filtered.push({
        ...node,
        children,
      });
    }
  }
  return filtered;
}

function normalizePath(path: string): string {
  return path.replace(/\/+$/, "");
}

function truncateForContext(content: string, maxChars: number): string {
  if (content.length <= maxChars) {
    return content;
  }
  const headLength = Math.floor(maxChars * 0.7);
  const tailLength = Math.max(0, maxChars - headLength);
  return `${content.slice(0, headLength)}\n\n...<truncated for context budget>...\n\n${content.slice(
    Math.max(content.length - tailLength, 0),
  )}`;
}

function formatBytes(size: number): string {
  if (size < 1024) {
    return `${size} B`;
  }
  if (size < 1024 * 1024) {
    return `${(size / 1024).toFixed(1)} KB`;
  }
  return `${(size / (1024 * 1024)).toFixed(1)} MB`;
}

function detectLanguage(path: string): string {
  const extension = path.split(".").pop()?.toLowerCase() ?? "";
  const map: Record<string, string> = {
    ts: "typescript",
    tsx: "typescript",
    js: "javascript",
    jsx: "javascript",
    json: "json",
    rs: "rust",
    py: "python",
    md: "markdown",
    html: "html",
    css: "css",
    scss: "scss",
    yml: "yaml",
    yaml: "yaml",
    toml: "toml",
    sql: "sql",
    sh: "shell",
    zsh: "shell",
    go: "go",
    java: "java",
    kt: "kotlin",
    swift: "swift",
  };
  return map[extension] ?? "plaintext";
}

function parsePatchProposal(text: string): CopilotPatchProposal | null {
  const payload = extractJsonObject(text);
  if (!payload) {
    return null;
  }
  try {
    const parsed = JSON.parse(payload) as {
      type?: string;
      summary?: string;
      patches?: Array<{ path?: string; patch?: string; base_sha256?: string | null; baseSha256?: string | null; description?: string }>;
    };
    if (parsed.type !== "patch_proposal" || !Array.isArray(parsed.patches) || parsed.patches.length === 0) {
      return null;
    }
    const patches = parsed.patches
      .map((entry) => ({
        path: (entry.path ?? "").trim(),
        patch: entry.patch ?? "",
        base_sha256: entry.base_sha256 ?? entry.baseSha256 ?? null,
        description: entry.description,
      }))
      .filter((entry) => entry.path.length > 0 && entry.patch.trim().length > 0);
    if (patches.length === 0) {
      return null;
    }
    return {
      summary: parsed.summary?.trim() || "Patch proposal generated.",
      patches,
      raw: payload,
    };
  } catch {
    return null;
  }
}

function extractJsonObject(text: string): string | null {
  const fencedMatch = text.match(/```json\s*([\s\S]*?)```/i);
  if (fencedMatch?.[1]) {
    return fencedMatch[1].trim();
  }
  const trimmed = text.trim();
  if (trimmed.startsWith("{") && trimmed.endsWith("}")) {
    return trimmed;
  }
  const start = trimmed.indexOf("{");
  const end = trimmed.lastIndexOf("}");
  if (start >= 0 && end > start) {
    return trimmed.slice(start, end + 1);
  }
  return null;
}
