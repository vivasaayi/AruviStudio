import { useEffect, useRef, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import {
  applyRepositoryPatch,
  readRepositoryFile,
  startDesktopCopilotStream,
} from "../../../lib/tauri";
import type { ModelDefinition, ModelProvider } from "../../../lib/types";
import { applyCopilotPatches } from "../lib/applyCopilotPatches";
import {
  parsePatchProposal,
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
  refetchTree: () => Promise<unknown>;
};

export function useIDECopilot({
  providers,
  models,
  activeFile,
  selectedRepoId,
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
    let unlistenChunk: UnlistenFn | null = null;
    let unlistenDone: UnlistenFn | null = null;
    let unlistenError: UnlistenFn | null = null;
    const requestedStreamId = crypto.randomUUID();
    let streamId: string | null = requestedStreamId;

    const cleanup = () => {
      streamId = null;
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

      await startDesktopCopilotStream({
        streamId: requestedStreamId,
        providerId: copilotProviderId,
        model: copilotModelName,
        history: [...copilotMessages, userMessage].map(({ role, content }) => ({ role, content })),
        systemPrompt: copilotSystemPrompt,
        patchMode: copilotMode === "patch",
        file: includeActiveFileContext && activeFile ? { path: activeFile.path, language: activeFile.language, content: activeFile.content } : null,
        contextBudget: maxContextChars,
        temperature: Number.isFinite(Number(copilotTemp)) ? Number(copilotTemp) : 0.2,
        maxTokens: Number.isFinite(Number(copilotMaxTokens)) ? Number(copilotMaxTokens) : 4096,
        sourceId: copilotSessionIdRef.current,
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
      await applyCopilotPatches(selectedRepoId, proposal.patches, { apply: applyRepositoryPatch, read: readRepositoryFile });
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
