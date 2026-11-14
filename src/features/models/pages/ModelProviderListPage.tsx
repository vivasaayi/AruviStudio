import { useRef, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import {
  listProviders,
  listModelDefinitions,
  discoverProviderModels,
  type DiscoveredModel,
} from "../../../lib/tauri";
import type { ModelDefinition, ModelProvider } from "../../../lib/types";
import { styles } from "../lib/modelProviderPageStyles";
import { ManagedLocalSpeechModelsPanel } from "../components/ManagedLocalSpeechModelsPanel";
import { ModelProviderGrid } from "../components/ModelProviderGrid";
import { ModelProviderQuickStartPanel } from "../components/ModelProviderQuickStartPanel";
import { RegisteredModelGrid } from "../components/RegisteredModelGrid";
import { useModelProviderPageActions } from "../hooks/useModelProviderPageActions";

export function ModelProviderListPage() {
  const queryClient = useQueryClient();
  const [showForm, setShowForm] = useState(false);
  const [showModelForm, setShowModelForm] = useState(false);
  const [form, setForm] = useState({ name: "", providerType: "openai_compatible", baseUrl: "http://localhost:1234/v1", authSecretRef: "" });
  const [discoveredModels, setDiscoveredModels] = useState<DiscoveredModel[]>([]);
  const [discoveryCursor, setDiscoveryCursor] = useState<string | null>(null);
  const [discovering, setDiscovering] = useState(false);
  const discoverySequence = useRef(0);
  const [modelForm, setModelForm] = useState({ providerId: "", name: "", contextWindow: "", capabilityTags: "", notes: "" });
  const [testResults, setTestResults] = useState<Record<string, { status: string; message: string }>>({});
  const [providerError, setProviderError] = useState<string | null>(null);
  const [providerSuccess, setProviderSuccess] = useState<string | null>(null);
  const [modelError, setModelError] = useState<string | null>(null);
  const [modelSuccess, setModelSuccess] = useState<string | null>(null);
  const [editingProvider, setEditingProvider] = useState<ModelProvider | null>(null);
  const [providerEditForm, setProviderEditForm] = useState({ name: "", providerType: "openai_compatible", baseUrl: "", authSecretRef: "", enabled: true });
  const [editingModel, setEditingModel] = useState<ModelDefinition | null>(null);
  const [modelEditForm, setModelEditForm] = useState({ providerId: "", name: "", contextWindow: "", capabilityTags: "", notes: "", enabled: true });

  const { data: providers, isLoading } = useQuery({ queryKey: ["providers"], queryFn: listProviders });
  const { data: modelDefinitions } = useQuery({ queryKey: ["model-definitions"], queryFn: listModelDefinitions });
  const {
    applyPreset,
    applySpeechSettings,
    createModelMutation,
    createMutation,
    deleteModelMutation,
    deleteProviderMutation,
    installLocalModelMutation,
    registerLocalModelMutation,
    startEditingModel,
    startEditingProvider,
    testConnectivity,
    updateModelMutation,
    updateProviderMutation,
    visibleProviders,
  } = useModelProviderPageActions({
    queryClient,
    providers: providers ?? [],
    form,
    setForm,
    modelForm,
    setModelForm,
    setShowForm,
    setShowModelForm,
    setTestResults,
    editingProvider,
    setEditingProvider,
    providerEditForm,
    setProviderEditForm,
    editingModel,
    setEditingModel,
    modelEditForm,
    setModelEditForm,
    setProviderError,
    setProviderSuccess,
    setModelError,
    setModelSuccess,
  });

  const discoverModels = async (nextPage: boolean) => {
    if (!modelForm.providerId || discovering) return;
    const sequence = ++discoverySequence.current;
    if (!nextPage) { setDiscoveredModels([]); setDiscoveryCursor(null); }
    setDiscovering(true);
    setModelError(null);
    try {
      const result = await discoverProviderModels(modelForm.providerId, nextPage ? discoveryCursor ?? undefined : undefined);
      if (sequence !== discoverySequence.current) return;
      setDiscoveredModels((previous) => nextPage ? [...previous, ...result.models] : result.models);
      setDiscoveryCursor(result.next_cursor);
    } catch (error) {
      if (sequence === discoverySequence.current) setModelError(String(error));
    } finally {
      if (sequence === discoverySequence.current) setDiscovering(false);
    }
  };

  return (
    <div style={styles.page}>
      <div style={styles.header}>
        <h1 style={styles.title}>Model Providers</h1>
        <div style={{ display: "flex", gap: 10 }}>
          <button
            style={{ ...styles.btn, backgroundColor: "#2c3139" }}
            onClick={() => {
              setModelError(null);
              setModelSuccess(null);
              setShowModelForm(true);
            }}
          >
            + Add Model
          </button>
          <button
            style={styles.btn}
            onClick={() => {
              setProviderError(null);
              setProviderSuccess(null);
              setShowForm(true);
            }}
          >
            + Add Provider
          </button>
        </div>
      </div>
      {providerSuccess && <div style={styles.feedbackSuccess}>{providerSuccess}</div>}
      {providerError && <div style={styles.feedbackError}>{providerError}</div>}
      {modelSuccess && <div style={styles.feedbackSuccess}>{modelSuccess}</div>}
      {modelError && <div style={styles.feedbackError}>{modelError}</div>}
      <ModelProviderQuickStartPanel onApplyPreset={applyPreset} />
      <ManagedLocalSpeechModelsPanel
        providers={providers ?? []}
        modelDefinitions={modelDefinitions ?? []}
        isBusy={installLocalModelMutation.isPending || registerLocalModelMutation.isPending}
        isInstallPending={installLocalModelMutation.isPending}
        onInstall={(entry) => installLocalModelMutation.mutate(entry)}
        onRegister={(entry) => registerLocalModelMutation.mutate(entry)}
        onUseForSpeech={(providerId, modelName) => void applySpeechSettings(providerId, modelName)}
      />
      {showForm && (
        <div style={styles.modalBackdrop}>
          <div style={styles.modal}>
            <div style={styles.modalHeader}>
              <div style={styles.modalTitle}>Add Provider</div>
              <button style={{ ...styles.btn, backgroundColor: "#2c3139" }} onClick={() => { setForm((current) => ({ ...current, authSecretRef: "" })); setShowForm(false); }}>Close</button>
            </div>
            <div style={styles.modalBody}>
        <div style={styles.form}>
          <label style={styles.label}>Provider Name</label><input aria-label="Provider name" style={styles.input} value={form.name} onChange={(e) => setForm({ ...form, name: e.target.value })} placeholder="e.g. DeepSeek (Hosted)" />
          <label style={styles.label}>Provider Type</label><select aria-label="Provider type" style={styles.select} value={form.providerType} onChange={(e) => setForm({ ...form, providerType: e.target.value })}><option value="openai_compatible">OpenAI Compatible</option><option value="openai_reasoning">OpenAI reasoning models</option><option value="anthropic">Anthropic Messages</option><option value="local_runtime">Local Runtime</option></select>
          <label style={styles.label}>{form.providerType === "local_runtime" ? "Local Model Path" : "Base URL"}</label><input aria-label="Provider base URL" placeholder="http://host:port/v1" style={styles.input} value={form.baseUrl} onChange={(e) => setForm({ ...form, baseUrl: e.target.value })} />
          <div style={{ fontSize: 12, color: "#8f96a3", marginBottom: 8 }}>
            {form.providerType === "local_runtime"
              ? <>For Whisper.cpp local speech, use an absolute path like <code>/Users/you/models/ggml-base.en.bin</code>.</>
              : form.providerType === "anthropic" ? <>Use <code>https://api.anthropic.com</code> and an Anthropic API key.</>
              : form.providerType === "openai_reasoning" ? <>Use this for models that accept <code>max_completion_tokens</code> and do not support temperature. Temperature is omitted.</>
              : <>For Aruvi Model Host, enter its running endpoint and API token; ports can change. For LM Studio, you can use <code>http://localhost:1234/v1</code>.</>}
          </div>
          <label style={styles.label}>API Key / Secret Ref (optional)</label><input aria-label="API Key / Secret Ref (optional)" type="password" autoComplete="new-password" style={styles.input} value={form.authSecretRef} onChange={(e) => setForm({ ...form, authSecretRef: e.target.value })} placeholder={form.providerType === "local_runtime" ? "Leave empty for local runtime" : "Paste API key (stored in Keychain) or ref:provider:... value"} />
          <div style={{ fontSize: 12, color: "#8f96a3", marginBottom: 12 }}>
            {form.providerType === "local_runtime"
              ? <>Local runtime providers do not require an API key. The path above should point directly to a downloaded Whisper model file.</>
              : <>Keys are stored in macOS Keychain. If Keychain access is blocked during development, runtime falls back to <code>~/.aruvistudio/llm-config.json</code>.</>}
          </div>
          {providerError && <div style={styles.feedbackError}>{providerError}</div>}
          <button style={styles.btn} onClick={() => createMutation.mutate()} disabled={!form.name || createMutation.isPending}>{createMutation.isPending ? "Adding..." : "Add Provider"}</button>
        </div>
            </div>
          </div>
        </div>
      )}
      {showModelForm && (
        <div style={styles.modalBackdrop}>
          <div style={styles.modal}>
            <div style={styles.modalHeader}>
              <div style={styles.modalTitle}>Add Model</div>
              <button style={{ ...styles.btn, backgroundColor: "#2c3139" }} onClick={() => setShowModelForm(false)}>Close</button>
            </div>
            <div style={styles.modalBody}>
        <div style={styles.form}>
          <div style={styles.sectionTitle}>Register Model Definition</div>
          <label style={styles.label}>Provider</label>
          <select aria-label="Provider for model" style={styles.select} value={modelForm.providerId} onChange={(e) => { discoverySequence.current++; setDiscoveredModels([]); setDiscoveryCursor(null); setDiscovering(false); setModelForm({ ...modelForm, providerId: e.target.value }); }}>
            <option value="">Select a provider</option>
            {(providers ?? []).map((provider) => (
              <option key={provider.id} value={provider.id}>
                {provider.name}
              </option>
            ))}
          </select>
          <button type="button" style={styles.btn} disabled={!modelForm.providerId || discovering} onClick={() => void discoverModels(false)}>{discovering ? "Discovering..." : "Discover available models"}</button>
          {discoveryCursor && <button type="button" style={styles.btn} disabled={discovering} onClick={() => void discoverModels(true)}>Load next page</button>}
          {discoveredModels.length > 0 && <><label style={styles.label}>Available model</label><select aria-label="Available model" style={styles.select} value="" onChange={(e) => { if (e.target.value) setModelForm({ ...modelForm, name: e.target.value }); }}><option value="">Select a discovered model</option>{discoveredModels.map((model) => {
            const task = typeof model.metadata?.task === "string" ? model.metadata.task.trim().slice(0, 80) : "";
            return <option key={model.id} value={model.id}>{model.display_name ? `${model.display_name} (${model.id})` : model.id}{task ? ` — provider task: ${task}` : ""}</option>;
          })}</select></>}
          <label style={styles.label}>Model Name</label>
          <input aria-label="Model name" style={styles.input} value={modelForm.name} onChange={(e) => setModelForm({ ...modelForm, name: e.target.value })} placeholder="e.g. deepseek-flash or deepseek-v4-pro" />
          <label style={styles.label}>Context Window (optional)</label>
          <input style={styles.input} value={modelForm.contextWindow} onChange={(e) => setModelForm({ ...modelForm, contextWindow: e.target.value })} placeholder="e.g. 64000" />
          <label style={styles.label}>Capability Tags (comma-separated)</label>
          <input style={styles.input} value={modelForm.capabilityTags} onChange={(e) => setModelForm({ ...modelForm, capabilityTags: e.target.value })} placeholder="coding, testing, planning, analysis" />
          <label style={styles.label}>Notes</label>
          <textarea style={{ ...styles.input, minHeight: 88, resize: "vertical" }} value={modelForm.notes} onChange={(e) => setModelForm({ ...modelForm, notes: e.target.value })} placeholder="Describe where this model should be preferred." />
          <div style={{ display: "flex", gap: 10, flexWrap: "wrap", marginBottom: 12 }}>
            <button
              style={{ ...styles.btn, backgroundColor: "#2c3139" }}
              onClick={() => {
                const deepSeekProvider = (providers ?? []).find((provider) => provider.base_url.includes("api.deepseek.com") || provider.name.toLowerCase().includes("deepseek"));
                setModelForm({
                  providerId: deepSeekProvider?.id ?? modelForm.providerId,
                  name: "deepseek-flash",
                  contextWindow: "",
                  capabilityTags: "coding,planning,analysis",
                  notes: "DeepSeek Flash; verify current availability and pricing with the provider.",
                });
              }}
            >
              DeepSeek Flash Preset
            </button>
            <button
              style={{ ...styles.btn, backgroundColor: "#2c3139" }}
              onClick={() => {
                const deepSeekProvider = (providers ?? []).find((provider) => provider.base_url.includes("api.deepseek.com") || provider.name.toLowerCase().includes("deepseek"));
                setModelForm({
                  providerId: deepSeekProvider?.id ?? modelForm.providerId,
                  name: "deepseek-v4-pro",
                  contextWindow: "",
                  capabilityTags: "coding,planning,analysis",
                  notes: "DeepSeek V4 Pro; verify current availability and pricing with the provider.",
                });
              }}
            >
              DeepSeek V4 Pro Preset
            </button>
            <button
              style={{ ...styles.btn, backgroundColor: "#355c2b" }}
              onClick={() => {
                const whisperProvider = (providers ?? []).find((provider) => provider.provider_type === "local_runtime" || provider.name.toLowerCase().includes("whisper"));
                setModelForm({
                  providerId: whisperProvider?.id ?? modelForm.providerId,
                  name: "whisper-base.en",
                  contextWindow: "",
                  capabilityTags: "speech_to_text,transcription,audio",
                  notes: "Local Whisper runtime for desktop voice transcription through whisper-rs/whisper.cpp.",
                });
              }}
            >
              Local Whisper Preset
            </button>
          </div>
          {modelError && <div style={styles.feedbackError}>{modelError}</div>}
          <button style={styles.btn} onClick={() => createModelMutation.mutate()} disabled={!modelForm.providerId || !modelForm.name || createModelMutation.isPending}>
            {createModelMutation.isPending ? "Adding..." : "Add Model"}
          </button>
        </div>
            </div>
          </div>
        </div>
      )}
      {isLoading ? (<div style={styles.empty}>Loading providers...</div>) : visibleProviders.length > 0 ? (
        <>
          <ModelProviderGrid
            providers={visibleProviders}
            testResults={testResults}
            onEditProvider={startEditingProvider}
            onTestProvider={(providerId) => void testConnectivity(providerId)}
          />
          <RegisteredModelGrid
            models={modelDefinitions ?? []}
            visibleProviders={visibleProviders}
            providers={providers ?? []}
            onEditModel={startEditingModel}
          />
        </>
      ) : (<div style={styles.empty}>No providers configured. Connect Aruvi Model Host for local coding models, or add another compatible provider.</div>)}

      {editingProvider && (
        <div style={styles.modalBackdrop}>
          <div style={styles.modal}>
            <div style={styles.modalHeader}>
              <div style={styles.modalTitle}>Edit Provider</div>
              <button style={{ ...styles.btn, backgroundColor: "#2c3139" }} onClick={() => { setProviderEditForm((current) => ({ ...current, authSecretRef: "" })); setEditingProvider(null); }}>Close</button>
            </div>
            <div style={styles.modalBody}>
              <div style={styles.form}>
                <label style={styles.label}>Provider Name</label>
                <input style={styles.input} value={providerEditForm.name} onChange={(e) => setProviderEditForm({ ...providerEditForm, name: e.target.value })} />
                <label style={styles.label}>Provider Type</label>
                <select style={styles.select} value={providerEditForm.providerType} onChange={(e) => setProviderEditForm({ ...providerEditForm, providerType: e.target.value })}>
                  <option value="openai_compatible">OpenAI Compatible</option>
                  <option value="openai_reasoning">OpenAI reasoning models</option>
                  <option value="local_runtime">Local Runtime</option>
                  <option value="anthropic">Anthropic Messages</option>
                </select>
                {providerEditForm.providerType === "openai_reasoning" && <div style={{ fontSize: 12, color: "#8f96a3" }}>Uses max_completion_tokens and omits temperature. Select only for models that support these settings.</div>}
                <label style={styles.label}>Base URL</label>
                <input style={styles.input} value={providerEditForm.baseUrl} onChange={(e) => setProviderEditForm({ ...providerEditForm, baseUrl: e.target.value })} />
                <label style={styles.label}>Rotate API Key (optional)</label>
                <input aria-label="Rotate API Key (optional)" type="password" autoComplete="new-password" style={styles.input} value={providerEditForm.authSecretRef} onChange={(e) => setProviderEditForm({ ...providerEditForm, authSecretRef: e.target.value })} placeholder="Leave empty to keep current key" />
                <label style={{ ...styles.label, display: "flex", gap: 8, alignItems: "center" }}>
                  <input type="checkbox" checked={providerEditForm.enabled} onChange={(e) => setProviderEditForm({ ...providerEditForm, enabled: e.target.checked })} />
                  Enabled
                </label>
                <div style={{ display: "flex", justifyContent: "space-between", gap: 10, marginTop: 8 }}>
                  <button style={{ ...styles.btn, backgroundColor: "#6c2020" }} onClick={() => deleteProviderMutation.mutate(editingProvider.id)}>
                    {deleteProviderMutation.isPending ? "Deleting..." : "Delete Provider"}
                  </button>
                  <button style={styles.btn} onClick={() => updateProviderMutation.mutate()} disabled={updateProviderMutation.isPending || !providerEditForm.name}>
                    {updateProviderMutation.isPending ? "Saving..." : "Save Changes"}
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>
      )}

      {editingModel && (
        <div style={styles.modalBackdrop}>
          <div style={styles.modal}>
            <div style={styles.modalHeader}>
              <div style={styles.modalTitle}>Edit Model</div>
              <button style={{ ...styles.btn, backgroundColor: "#2c3139" }} onClick={() => setEditingModel(null)}>Close</button>
            </div>
            <div style={styles.modalBody}>
              <div style={styles.form}>
                <label style={styles.label}>Provider</label>
                <select style={styles.select} value={modelEditForm.providerId} onChange={(e) => setModelEditForm({ ...modelEditForm, providerId: e.target.value })}>
                  {(providers ?? []).map((provider) => (
                    <option key={provider.id} value={provider.id}>{provider.name}</option>
                  ))}
                </select>
                <label style={styles.label}>Model Name</label>
                <input style={styles.input} value={modelEditForm.name} onChange={(e) => setModelEditForm({ ...modelEditForm, name: e.target.value })} />
                <label style={styles.label}>Context Window</label>
                <input style={styles.input} value={modelEditForm.contextWindow} onChange={(e) => setModelEditForm({ ...modelEditForm, contextWindow: e.target.value })} />
                <label style={styles.label}>Capability Tags</label>
                <input style={styles.input} value={modelEditForm.capabilityTags} onChange={(e) => setModelEditForm({ ...modelEditForm, capabilityTags: e.target.value })} placeholder="coding, testing, planning, analysis" />
                <label style={styles.label}>Notes</label>
                <textarea style={{ ...styles.input, minHeight: 88, resize: "vertical" }} value={modelEditForm.notes} onChange={(e) => setModelEditForm({ ...modelEditForm, notes: e.target.value })} />
                <label style={{ ...styles.label, display: "flex", gap: 8, alignItems: "center" }}>
                  <input type="checkbox" checked={modelEditForm.enabled} onChange={(e) => setModelEditForm({ ...modelEditForm, enabled: e.target.checked })} />
                  Enabled
                </label>
                <div style={{ display: "flex", justifyContent: "space-between", gap: 10, marginTop: 8 }}>
                  <button style={{ ...styles.btn, backgroundColor: "#6c2020" }} onClick={() => deleteModelMutation.mutate(editingModel.id)}>
                    {deleteModelMutation.isPending ? "Deleting..." : "Delete Model"}
                  </button>
                  <button style={styles.btn} onClick={() => updateModelMutation.mutate()} disabled={updateModelMutation.isPending || !modelEditForm.name || !modelEditForm.providerId}>
                    {updateModelMutation.isPending ? "Saving..." : "Save Changes"}
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
