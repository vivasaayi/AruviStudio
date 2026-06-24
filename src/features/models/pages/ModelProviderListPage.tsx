import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import {
  browseForLocalModelFile,
  listProviders,
  createProvider,
  createModelDefinition,
  listModelDefinitions,
  setSetting,
  testProviderConnectivity,
  installManagedLocalModel,
  registerLocalRuntimeModel,
  updateProvider,
  deleteProvider,
  updateModelDefinition,
  deleteModelDefinition,
} from "../../../lib/tauri";
import type { ModelDefinition, ModelProvider } from "../../../lib/types";
import {
  LEGACY_WHISPER_PLACEHOLDER_PATH,
  MANAGED_LOCAL_MODELS,
  SPEECH_MODEL_KEY,
  SPEECH_PROVIDER_KEY,
} from "../lib/modelProviderConstants";
import { splitCommaSeparated } from "../lib/modelProviderFormUtils";
import { styles } from "../lib/modelProviderPageStyles";
import { ManagedLocalSpeechModelsPanel } from "../components/ManagedLocalSpeechModelsPanel";
import { ModelProviderGrid } from "../components/ModelProviderGrid";
import { ModelProviderQuickStartPanel } from "../components/ModelProviderQuickStartPanel";
import { RegisteredModelGrid } from "../components/RegisteredModelGrid";

export function ModelProviderListPage() {
  const queryClient = useQueryClient();
  const [showForm, setShowForm] = useState(false);
  const [showModelForm, setShowModelForm] = useState(false);
  const [form, setForm] = useState({ name: "", providerType: "openai_compatible", baseUrl: "http://localhost:1234/v1", authSecretRef: "" });
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
  const refreshModelQueries = async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ["providers"] }),
      queryClient.invalidateQueries({ queryKey: ["model-definitions"] }),
    ]);
  };

  const installLocalModelMutation = useMutation({
    mutationFn: (entry: (typeof MANAGED_LOCAL_MODELS)[number]) =>
      installManagedLocalModel({
        providerName: entry.providerName,
        modelName: entry.modelName,
        downloadUrl: entry.downloadUrl,
        fileName: entry.fileName,
        capabilityTags: ["speech_to_text", "transcription", "audio", "local_runtime"],
        notes: entry.notes,
      }),
    onSuccess: async (result, entry) => {
      await refreshModelQueries();
      await applySpeechSettings(result.provider.id, result.model_definition.name);
      setProviderSuccess(`${entry.displayName} is ready at ${result.file_path} and set as the active speech model.`);
      setProviderError(null);
    },
    onError: (error) => {
      setProviderError(String(error));
      setProviderSuccess(null);
    },
  });

  const registerLocalModelMutation = useMutation({
    mutationFn: async (entry: (typeof MANAGED_LOCAL_MODELS)[number]) => {
      const selectedPath = await browseForLocalModelFile();
      if (!selectedPath) {
        throw new Error("Model registration cancelled.");
      }
      return registerLocalRuntimeModel({
        providerName: entry.providerName,
        modelName: entry.modelName,
        modelPath: selectedPath,
        capabilityTags: ["speech_to_text", "transcription", "audio", "local_runtime"],
        notes: entry.notes,
      });
    },
    onSuccess: async (result, entry) => {
      await refreshModelQueries();
      await applySpeechSettings(result.provider.id, result.model_definition.name);
      setProviderSuccess(`${entry.displayName} registered from ${result.file_path} and set as the active speech model.`);
      setProviderError(null);
    },
    onError: (error) => {
      if (String(error).includes("cancelled")) {
        return;
      }
      setProviderError(String(error));
      setProviderSuccess(null);
    },
  });

  const createMutation = useMutation({
    mutationFn: () => createProvider(form),
    onSuccess: async (provider) => {
      await queryClient.invalidateQueries({ queryKey: ["providers"] });
      setProviderSuccess(`Provider "${provider.name}" added.`);
      setProviderError(null);
      setForm({ name: "", providerType: "openai_compatible", baseUrl: "http://localhost:1234/v1", authSecretRef: "" });
      setShowForm(false);
    },
    onError: (error) => {
      setProviderError(String(error));
      setProviderSuccess(null);
    },
  });

  const createModelMutation = useMutation({
    mutationFn: () =>
      createModelDefinition({
        providerId: modelForm.providerId,
        name: modelForm.name,
        contextWindow: modelForm.contextWindow ? Number(modelForm.contextWindow) : undefined,
        capabilityTags: splitCommaSeparated(modelForm.capabilityTags),
        notes: modelForm.notes.trim() || undefined,
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["model-definitions"] });
      setModelSuccess(`Model "${modelForm.name}" added.`);
      setModelError(null);
      setModelForm({ providerId: "", name: "", contextWindow: "", capabilityTags: "", notes: "" });
      setShowModelForm(false);
    },
    onError: (error) => {
      setModelError(String(error));
      setModelSuccess(null);
    },
  });

  const updateProviderMutation = useMutation({
    mutationFn: () =>
      updateProvider({
        id: editingProvider!.id,
        name: providerEditForm.name,
        providerType: providerEditForm.providerType,
        baseUrl: providerEditForm.baseUrl,
        authSecretRef: providerEditForm.authSecretRef.trim() ? providerEditForm.authSecretRef.trim() : undefined,
        enabled: providerEditForm.enabled,
      }),
    onSuccess: async (provider) => {
      await queryClient.invalidateQueries({ queryKey: ["providers"] });
      setProviderSuccess(`Provider "${provider.name}" updated.`);
      setProviderError(null);
      setEditingProvider(null);
    },
    onError: (error) => {
      setProviderError(String(error));
      setProviderSuccess(null);
    },
  });

  const deleteProviderMutation = useMutation({
    mutationFn: (id: string) => deleteProvider(id),
    onSuccess: async () => {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["providers"] }),
        queryClient.invalidateQueries({ queryKey: ["model-definitions"] }),
      ]);
      setProviderSuccess("Provider deleted.");
      setProviderError(null);
    },
    onError: (error) => {
      setProviderError(String(error));
      setProviderSuccess(null);
    },
  });

  const updateModelMutation = useMutation({
    mutationFn: () =>
      updateModelDefinition({
        id: editingModel!.id,
        providerId: modelEditForm.providerId,
        name: modelEditForm.name,
        contextWindow: modelEditForm.contextWindow ? Number(modelEditForm.contextWindow) : undefined,
        capabilityTags: splitCommaSeparated(modelEditForm.capabilityTags),
        notes: modelEditForm.notes.trim() || undefined,
        enabled: modelEditForm.enabled,
      }),
    onSuccess: async (model) => {
      await queryClient.invalidateQueries({ queryKey: ["model-definitions"] });
      setModelSuccess(`Model "${model.name}" updated.`);
      setModelError(null);
      setEditingModel(null);
    },
    onError: (error) => {
      setModelError(String(error));
      setModelSuccess(null);
    },
  });

  const deleteModelMutation = useMutation({
    mutationFn: (id: string) => deleteModelDefinition(id),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["model-definitions"] });
      setModelSuccess("Model deleted.");
      setModelError(null);
    },
    onError: (error) => {
      setModelError(String(error));
      setModelSuccess(null);
    },
  });

  const testConnectivity = async (id: string) => {
    setTestResults((prev) => ({ ...prev, [id]: { status: "testing", message: "Testing..." } }));
    try {
      const result = await testProviderConnectivity(id);
      setTestResults((prev) => ({ ...prev, [id]: { status: "success", message: result } }));
    } catch (err: unknown) {
      setTestResults((prev) => ({ ...prev, [id]: { status: "error", message: String(err) } }));
    }
  };

  const applyPreset = (preset: "deepseek" | "lm_studio") => {
    if (preset === "deepseek") {
      setForm({
        name: "DeepSeek (Hosted)",
        providerType: "openai_compatible",
        baseUrl: "https://api.deepseek.com/v1",
        authSecretRef: "",
      });
      setProviderError(null);
      setProviderSuccess(null);
      setShowForm(true);
      return;
    }

    setForm({
      name: "LM Studio (Local)",
      providerType: "openai_compatible",
      baseUrl: "http://localhost:1234/v1",
      authSecretRef: "",
    });
    setProviderError(null);
    setProviderSuccess(null);
    setShowForm(true);
  };

  const applySpeechSettings = async (providerId: string, modelName: string) => {
    await Promise.all([
      setSetting(SPEECH_PROVIDER_KEY, providerId),
      setSetting(SPEECH_MODEL_KEY, modelName),
    ]);
    setProviderSuccess(`Speech settings now use ${modelName}.`);
    setProviderError(null);
  };

  const visibleProviders = (providers ?? []).filter(
    (provider) => !(provider.provider_type === "local_runtime" && provider.base_url === LEGACY_WHISPER_PLACEHOLDER_PATH),
  );

  const startEditingProvider = (provider: ModelProvider) => {
    setEditingProvider(provider);
    setProviderEditForm({
      name: provider.name,
      providerType: provider.provider_type,
      baseUrl: provider.base_url,
      authSecretRef: "",
      enabled: provider.enabled,
    });
  };

  const startEditingModel = (model: ModelDefinition) => {
    setEditingModel(model);
    setModelEditForm({
      providerId: model.provider_id,
      name: model.name,
      contextWindow: model.context_window ? String(model.context_window) : "",
      capabilityTags: model.capability_tags.join(", "),
      notes: model.notes,
      enabled: model.enabled,
    });
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
              <button style={{ ...styles.btn, backgroundColor: "#2c3139" }} onClick={() => setShowForm(false)}>Close</button>
            </div>
            <div style={styles.modalBody}>
        <div style={styles.form}>
          <label style={styles.label}>Provider Name</label><input style={styles.input} value={form.name} onChange={(e) => setForm({ ...form, name: e.target.value })} placeholder="e.g. DeepSeek (Hosted)" />
          <label style={styles.label}>Provider Type</label><select style={styles.select} value={form.providerType} onChange={(e) => setForm({ ...form, providerType: e.target.value })}><option value="openai_compatible">OpenAI Compatible</option><option value="local_runtime">Local Runtime</option></select>
          <label style={styles.label}>{form.providerType === "local_runtime" ? "Local Model Path" : "Base URL"}</label><input style={styles.input} value={form.baseUrl} onChange={(e) => setForm({ ...form, baseUrl: e.target.value })} />
          <div style={{ fontSize: 12, color: "#8f96a3", marginBottom: 8 }}>
            {form.providerType === "local_runtime"
              ? <>For Whisper.cpp local speech, use an absolute path like <code>/Users/you/models/ggml-base.en.bin</code>.</>
              : <>You can use <code>http://localhost:1234</code> or <code>http://localhost:1234/v1</code>. Aruvi normalizes both.</>}
          </div>
          <label style={styles.label}>API Key / Secret Ref (optional)</label><input style={styles.input} value={form.authSecretRef} onChange={(e) => setForm({ ...form, authSecretRef: e.target.value })} placeholder={form.providerType === "local_runtime" ? "Leave empty for local runtime" : "Paste API key (stored in Keychain) or ref:provider:... value"} />
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
          <select style={styles.select} value={modelForm.providerId} onChange={(e) => setModelForm({ ...modelForm, providerId: e.target.value })}>
            <option value="">Select a provider</option>
            {(providers ?? []).map((provider) => (
              <option key={provider.id} value={provider.id}>
                {provider.name}
              </option>
            ))}
          </select>
          <label style={styles.label}>Model Name</label>
          <input style={styles.input} value={modelForm.name} onChange={(e) => setModelForm({ ...modelForm, name: e.target.value })} placeholder="e.g. deepseek-chat or deepseek-coder" />
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
                  name: "deepseek-chat",
                  contextWindow: "64000",
                  capabilityTags: "planning,analysis,review",
                  notes: "Preferred for planning, analysis, and review-oriented stages.",
                });
              }}
            >
              DeepSeek Chat Preset
            </button>
            <button
              style={{ ...styles.btn, backgroundColor: "#2c3139" }}
              onClick={() => {
                const deepSeekProvider = (providers ?? []).find((provider) => provider.base_url.includes("api.deepseek.com") || provider.name.toLowerCase().includes("deepseek"));
                setModelForm({
                  providerId: deepSeekProvider?.id ?? modelForm.providerId,
                  name: "deepseek-coder",
                  contextWindow: "64000",
                  capabilityTags: "coding,implementation,testing",
                  notes: "Preferred for implementation and test-generation stages.",
                });
              }}
            >
              DeepSeek Coder Preset
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
      ) : (<div style={styles.empty}>No providers configured. Add DeepSeek (hosted) for a fast end-to-end path or LM Studio for local runs.</div>)}

      {editingProvider && (
        <div style={styles.modalBackdrop}>
          <div style={styles.modal}>
            <div style={styles.modalHeader}>
              <div style={styles.modalTitle}>Edit Provider</div>
              <button style={{ ...styles.btn, backgroundColor: "#2c3139" }} onClick={() => setEditingProvider(null)}>Close</button>
            </div>
            <div style={styles.modalBody}>
              <div style={styles.form}>
                <label style={styles.label}>Provider Name</label>
                <input style={styles.input} value={providerEditForm.name} onChange={(e) => setProviderEditForm({ ...providerEditForm, name: e.target.value })} />
                <label style={styles.label}>Provider Type</label>
                <select style={styles.select} value={providerEditForm.providerType} onChange={(e) => setProviderEditForm({ ...providerEditForm, providerType: e.target.value })}>
                  <option value="openai_compatible">OpenAI Compatible</option>
                  <option value="local_runtime">Local Runtime</option>
                </select>
                <label style={styles.label}>Base URL</label>
                <input style={styles.input} value={providerEditForm.baseUrl} onChange={(e) => setProviderEditForm({ ...providerEditForm, baseUrl: e.target.value })} />
                <label style={styles.label}>Rotate API Key (optional)</label>
                <input style={styles.input} value={providerEditForm.authSecretRef} onChange={(e) => setProviderEditForm({ ...providerEditForm, authSecretRef: e.target.value })} placeholder="Leave empty to keep current key" />
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
