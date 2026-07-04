import { useEffect, useRef, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import {
  applyRepositoryPatch,
  readRepositoryFile,
  startModelChatStream,
} from "../../../lib/tauri";
import type { ModelDefinition, ModelProvider } from "../../../lib/types";
import {
  parsePatchProposal,
  truncateForContext,
  type CopilotMode,
  type CopilotPatchProposal,
  type LocalChatMessage,
} from "../lib/idePageModel";

type IDEOpenFile = {
  id: string;
  path: string;
  content: string;
  language: string;
};

type UseIDECopilotInput = {
  providers: ModelProvider[];
  models: ModelDefinition[];
  activeFile: IDEOpenFile | null;
  selectedRepoId: string;
  openFiles: IDEOpenFile[];
  replaceFileContent: (id: string, content: string) => void;
  markFileSaved: (id: string) => void;
  refetchTree: () => Promise<unknown>;
};

export function useIDECopilot({
  providers,
  models,
  activeFile,
  selectedRepoId,
  openFiles,
  replaceFileContent,
  markFileSaved,
  refetchTree,
}: UseIDECopilotInput) {
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

  const resetCopilot = () => {
    setCopilotMessages([]);
    setPatchProposal(null);
    setCopilotError(null);
    copilotSessionIdRef.current = crypto.randomUUID();
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

  return {
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
  };
}
