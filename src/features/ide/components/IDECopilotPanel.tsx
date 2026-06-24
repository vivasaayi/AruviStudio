import type { Dispatch, SetStateAction } from "react";

import type { ModelDefinition, ModelProvider } from "../../../lib/types";
import {
  truncateForContext,
  type CopilotMode,
  type CopilotPatchProposal,
  type LocalChatMessage,
} from "../lib/idePageModel";
import { styles } from "../lib/idePageStyles";

type IDECopilotPanelProps = {
  providers: ModelProvider[];
  modelOptions: ModelDefinition[];
  copilotMode: CopilotMode;
  setCopilotMode: Dispatch<SetStateAction<CopilotMode>>;
  copilotProviderId: string;
  setCopilotProviderId: Dispatch<SetStateAction<string>>;
  copilotModelName: string;
  setCopilotModelName: Dispatch<SetStateAction<string>>;
  copilotTemp: string;
  setCopilotTemp: Dispatch<SetStateAction<string>>;
  copilotMaxTokens: string;
  setCopilotMaxTokens: Dispatch<SetStateAction<string>>;
  copilotSystemPrompt: string;
  setCopilotSystemPrompt: Dispatch<SetStateAction<string>>;
  includeActiveFileContext: boolean;
  setIncludeActiveFileContext: Dispatch<SetStateAction<boolean>>;
  contextBudgetChars: string;
  setContextBudgetChars: Dispatch<SetStateAction<string>>;
  copilotMessages: LocalChatMessage[];
  patchProposal: CopilotPatchProposal | null;
  copilotDraft: string;
  setCopilotDraft: Dispatch<SetStateAction<string>>;
  copilotError: string | null;
  isCopilotSending: boolean;
  isApplyingProposal: boolean;
  onClear: () => void;
  onSend: () => void;
  onApplyPatchProposal: (proposal: CopilotPatchProposal) => void;
};

export function IDECopilotPanel({
  providers,
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
  onClear,
  onSend,
  onApplyPatchProposal,
}: IDECopilotPanelProps) {
  return (
    <div style={styles.panel}>
      <div style={styles.panelHeader}>
        <div style={styles.panelTitle}>Aruvi Copilot</div>
        <button
          style={styles.buttonGhost}
          onClick={onClear}
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
            onClick={() => onApplyPatchProposal(patchProposal)}
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
              onSend();
            }
          }}
        />
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: 8 }}>
          <div style={styles.status}>
            {isCopilotSending ? "Streaming response..." : "Cmd/Ctrl + Enter to send"}
          </div>
          <button style={styles.button} onClick={onSend} disabled={isCopilotSending}>
            {isCopilotSending ? "Sending..." : copilotMode === "patch" ? "Generate Proposal" : "Send"}
          </button>
        </div>
        {copilotError && <div style={styles.error}>{copilotError}</div>}
      </div>
    </div>
  );
}
