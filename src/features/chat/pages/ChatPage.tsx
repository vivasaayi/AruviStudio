import React, { useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { listModelDefinitions, listProviders, startModelChatStream } from "../../../lib/tauri";
import type { ChatMessagePayload } from "../../../lib/types";

const styles: Record<string, React.CSSProperties> = {
  page: { display: "flex", flexDirection: "column", gap: 12, height: "100%" },
  header: { display: "flex", justifyContent: "space-between", alignItems: "center", gap: 10 },
  title: { fontSize: 20, fontWeight: 700, color: "#e0e0e0", margin: 0 },
  subtitle: { fontSize: 12, color: "#8f96a3" },
  panel: { backgroundColor: "#252526", border: "1px solid #333", borderRadius: 12, padding: 14 },
  row: { display: "grid", gridTemplateColumns: "1fr 1fr auto auto", gap: 10, alignItems: "end" },
  label: { fontSize: 12, color: "#9aa0a6", marginBottom: 6, display: "block" },
  input: { width: "100%", padding: "9px 12px", backgroundColor: "#1e1e1e", border: "1px solid #444", borderRadius: 8, color: "#e0e0e0", fontSize: 13, boxSizing: "border-box" as const },
  chatBox: { flex: 1, minHeight: 0, backgroundColor: "#212327", border: "1px solid #32353d", borderRadius: 12, padding: 12, overflowY: "auto" as const, display: "flex", flexDirection: "column", gap: 10 },
  bubbleUser: { alignSelf: "flex-end", maxWidth: "78%", backgroundColor: "#0e639c", borderRadius: 10, padding: "10px 12px", color: "#fff", whiteSpace: "pre-wrap" as const },
  bubbleAssistant: { alignSelf: "flex-start", maxWidth: "85%", backgroundColor: "#2b2f37", borderRadius: 10, padding: "10px 12px", color: "#e9edf6", whiteSpace: "pre-wrap" as const },
  composer: { display: "flex", gap: 10 },
  textarea: { flex: 1, minHeight: 80, padding: "10px 12px", backgroundColor: "#181a1f", border: "1px solid #3c4048", borderRadius: 10, color: "#e0e0e0", resize: "vertical" as const, fontSize: 13 },
  btn: { padding: "8px 14px", fontSize: 13, backgroundColor: "#0e639c", color: "#fff", border: "none", borderRadius: 8, cursor: "pointer", fontWeight: 700 },
  btnGhost: { padding: "8px 14px", fontSize: 13, backgroundColor: "#2c3139", color: "#e0e0e0", border: "1px solid #3c4048", borderRadius: 8, cursor: "pointer", fontWeight: 700 },
  error: { color: "#f44747", fontSize: 12 },
  info: { color: "#8f96a3", fontSize: 12 },
  empty: { color: "#777", textAlign: "center" as const, padding: 24 },
};

type LocalChatMessage = ChatMessagePayload & { id: string };

export function ChatPage() {
  const [systemPrompt, setSystemPrompt] = useState(
    "Always reply in English unless the user explicitly asks for another language.",
  );
  const [providerId, setProviderId] = useState("");
  const [modelName, setModelName] = useState("");
  const [temperature, setTemperature] = useState("0.7");
  const [maxTokens, setMaxTokens] = useState("2048");
  const [draft, setDraft] = useState("");
  const [messages, setMessages] = useState<LocalChatMessage[]>([]);
  const [isSending, setIsSending] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const { data: providers = [] } = useQuery({ queryKey: ["providers"], queryFn: listProviders });
  const { data: models = [] } = useQuery({ queryKey: ["model-definitions"], queryFn: listModelDefinitions });

  const modelOptions = useMemo(
    () => models.filter((model) => model.provider_id === providerId && model.enabled),
    [models, providerId],
  );

  React.useEffect(() => {
    if (!providerId && providers.length > 0) {
      setProviderId(providers[0].id);
    }
  }, [providerId, providers]);

  React.useEffect(() => {
    if (!providerId) {
      return;
    }
    if (!modelName || !modelOptions.some((entry) => entry.name === modelName)) {
      setModelName(modelOptions[0]?.name ?? "");
    }
  }, [providerId, modelName, modelOptions]);

  const send = async () => {
    setError(null);
    const content = draft.trim();
    if (!content) {
      return;
    }
    if (!providerId || !modelName) {
      setError("Select a provider and model first.");
      return;
    }

    const userMessage: LocalChatMessage = { id: crypto.randomUUID(), role: "user", content };
    const assistantMessageId = crypto.randomUUID();
    const assistantPlaceholder: LocalChatMessage = { id: assistantMessageId, role: "assistant", content: "" };
    const nextMessages = [...messages, userMessage, assistantPlaceholder];
    setMessages(nextMessages);
    setDraft("");
    setIsSending(true);

    let unlistenChunk: UnlistenFn | null = null;
    let unlistenDone: UnlistenFn | null = null;
    let unlistenError: UnlistenFn | null = null;
    let activeStreamId: string | null = null;

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
        if (!activeStreamId || event.payload.stream_id !== activeStreamId) {
          return;
        }
        setMessages((current) =>
          current.map((entry) =>
            entry.id === assistantMessageId
              ? { ...entry, content: `${entry.content}${event.payload.delta}` }
              : entry,
          ),
        );
      });

      unlistenDone = await listen<{ stream_id: string }>("chat_stream_done", (event) => {
        if (!activeStreamId || event.payload.stream_id !== activeStreamId) {
          return;
        }
        setIsSending(false);
        cleanup();
      });

      unlistenError = await listen<{ stream_id: string; error: string }>("chat_stream_error", (event) => {
        if (!activeStreamId || event.payload.stream_id !== activeStreamId) {
          return;
        }
        setError(event.payload.error);
        setIsSending(false);
        cleanup();
      });

      activeStreamId = await startModelChatStream({
        providerId,
        model: modelName,
        messages: [
          {
            role: "system",
            content:
              systemPrompt.trim() ||
              "Always reply in English unless the user explicitly asks for another language.",
          },
          ...[...messages, userMessage].map(({ role, content: text }) => ({
            role,
            content: text,
          })),
        ],
        temperature: Number.isFinite(Number(temperature)) ? Number(temperature) : 0.7,
        maxTokens: Number.isFinite(Number(maxTokens)) ? Number(maxTokens) : 2048,
      });

      // Safety timeout to avoid permanent pending state if stream never closes.
      window.setTimeout(() => {
        if (activeStreamId) {
          setIsSending(false);
          cleanup();
        }
      }, 120000);
    } catch (sendError) {
      setError(String(sendError));
      setMessages((current) => current.filter((entry) => entry.id !== assistantMessageId));
      cleanup();
      setIsSending(false);
    }
  };

  return (
    <div style={styles.page}>
      <div style={styles.header}>
        <div>
          <h1 style={styles.title}>Direct Chat</h1>
          <div style={styles.subtitle}>Ad-hoc conversation with your selected provider/model.</div>
        </div>
      </div>

      <div style={styles.panel}>
        <div style={styles.row}>
          <div>
            <label style={styles.label}>Provider</label>
            <select style={styles.input} value={providerId} onChange={(e) => setProviderId(e.target.value)}>
              <option value="">Select provider</option>
              {providers.map((provider) => (
                <option key={provider.id} value={provider.id}>
                  {provider.name}
                </option>
              ))}
            </select>
          </div>
          <div>
            <label style={styles.label}>Model</label>
            <select style={styles.input} value={modelName} onChange={(e) => setModelName(e.target.value)}>
              <option value="">Select model</option>
              {modelOptions.map((model) => (
                <option key={model.id} value={model.name}>
                  {model.name}
                </option>
              ))}
            </select>
          </div>
          <div>
            <label style={styles.label}>Temp</label>
            <input style={styles.input} value={temperature} onChange={(e) => setTemperature(e.target.value)} />
          </div>
          <div>
            <label style={styles.label}>Max Tokens</label>
            <input style={styles.input} value={maxTokens} onChange={(e) => setMaxTokens(e.target.value)} />
          </div>
        </div>
        {providerId && modelOptions.length === 0 && (
          <div style={styles.info}>No models registered for this provider. Add one in the Models tab.</div>
        )}
        <div style={{ marginTop: 10 }}>
          <label style={styles.label}>System Prompt</label>
          <textarea
            style={{ ...styles.textarea, minHeight: 64 }}
            value={systemPrompt}
            onChange={(e) => setSystemPrompt(e.target.value)}
            placeholder="Instruction injected before user messages"
          />
        </div>
      </div>

      <div style={styles.chatBox}>
        {messages.length === 0 ? (
          <div style={styles.empty}>Start a conversation.</div>
        ) : (
          messages.map((message) => (
            <div key={message.id} style={message.role === "user" ? styles.bubbleUser : styles.bubbleAssistant}>
              {message.content}
            </div>
          ))
        )}
      </div>

      <div style={styles.composer}>
        <textarea
          style={styles.textarea}
          placeholder="Ask anything..."
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={(e) => {
            if ((e.metaKey || e.ctrlKey) && e.key === "Enter" && !isSending) {
              e.preventDefault();
              void send();
            }
          }}
        />
        <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          <button style={styles.btn} onClick={() => void send()} disabled={isSending}>
            {isSending ? "Sending..." : "Send"}
          </button>
          <button style={styles.btnGhost} onClick={() => setMessages([])} disabled={isSending}>
            Clear
          </button>
        </div>
      </div>
      {error && <div style={styles.error}>{error}</div>}
    </div>
  );
}
