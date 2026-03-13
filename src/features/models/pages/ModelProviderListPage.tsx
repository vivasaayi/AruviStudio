import React, { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import {
  listProviders,
  createProvider,
  createModelDefinition,
  listModelDefinitions,
  testProviderConnectivity,
  updateProvider,
  deleteProvider,
  updateModelDefinition,
  deleteModelDefinition,
} from "../../../lib/tauri";
import type { ModelDefinition, ModelProvider } from "../../../lib/types";

const styles: Record<string, React.CSSProperties> = {
  page: { maxWidth: 960, margin: "0 auto" },
  header: { display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 24 },
  title: { fontSize: 20, fontWeight: 600, color: "#e0e0e0" },
  btn: { padding: "6px 16px", fontSize: 13, backgroundColor: "#0e639c", color: "#fff", border: "none", borderRadius: 4, cursor: "pointer" },
  btnTest: { padding: "4px 10px", fontSize: 12, backgroundColor: "#2d6a3f", color: "#fff", border: "none", borderRadius: 4, cursor: "pointer" },
  grid: { display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(340px, 1fr))", gap: 16 },
  card: { backgroundColor: "#252526", borderRadius: 8, padding: 16, border: "1px solid #333" },
  name: { fontSize: 15, fontWeight: 600, color: "#e0e0e0", marginBottom: 4 },
  type: { fontSize: 13, color: "#569cd6", marginBottom: 8 },
  url: { fontSize: 12, color: "#888", fontFamily: "monospace", marginBottom: 12, wordBreak: "break-all" as const },
  cardFooter: { display: "flex", justifyContent: "space-between", alignItems: "center" },
  badge: { fontSize: 11, padding: "2px 8px", borderRadius: 10, display: "inline-block" },
  empty: { textAlign: "center" as const, color: "#666", padding: 40, fontSize: 14 },
  form: { backgroundColor: "#252526", padding: 20, borderRadius: 8, marginBottom: 24, border: "1px solid #333" },
  sectionTitle: { fontSize: 16, fontWeight: 600, color: "#e0e0e0", marginBottom: 12 },
  input: { width: "100%", padding: "8px 12px", backgroundColor: "#1e1e1e", border: "1px solid #444", borderRadius: 4, color: "#e0e0e0", fontSize: 13, marginBottom: 12, boxSizing: "border-box" as const },
  select: { width: "100%", padding: "8px 12px", backgroundColor: "#1e1e1e", border: "1px solid #444", borderRadius: 4, color: "#e0e0e0", fontSize: 13, marginBottom: 12 },
  label: { fontSize: 12, color: "#999", display: "block", marginBottom: 4 },
  testResult: { fontSize: 12, marginTop: 8, padding: 8, borderRadius: 4 },
  subGrid: { display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(280px, 1fr))", gap: 12 },
  modelCard: { backgroundColor: "#1f2228", borderRadius: 8, padding: 14, border: "1px solid #333" },
  modelName: { fontSize: 14, fontWeight: 600, color: "#e0e0e0", marginBottom: 4 },
  modelMeta: { fontSize: 12, color: "#999", marginBottom: 6 },
  feedbackSuccess: { color: "#4ec9b0", fontSize: 12, marginTop: 8 },
  feedbackError: { color: "#f44747", fontSize: 12, marginTop: 8 },
  modalBackdrop: { position: "fixed", inset: 0, backgroundColor: "rgba(8, 10, 14, 0.72)", display: "flex", alignItems: "center", justifyContent: "center", padding: 24, zIndex: 50 },
  modal: { width: "min(860px, 100%)", maxHeight: "86vh", backgroundColor: "#252526", border: "1px solid #333", borderRadius: 12, overflow: "hidden", boxShadow: "0 24px 60px rgba(0,0,0,0.45)" },
  modalHeader: { display: "flex", justifyContent: "space-between", alignItems: "center", padding: "12px 16px", borderBottom: "1px solid #333" },
  modalTitle: { fontSize: 16, fontWeight: 600, color: "#e0e0e0" },
  modalBody: { padding: 16, overflow: "auto", maxHeight: "calc(86vh - 56px)" },
};

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
      <div style={{ ...styles.form, marginBottom: 18 }}>
        <div style={{ ...styles.title, fontSize: 16, marginBottom: 10 }}>Quick Start</div>
        <div style={{ color: "#9aa0a6", fontSize: 13, lineHeight: 1.5, marginBottom: 14 }}>
          Use DeepSeek hosted first if you want a quick end-to-end path. The provider is OpenAI-compatible, so it fits the current orchestration layer without waiting on local runtime work.
        </div>
        <div style={{ display: "flex", gap: 10, flexWrap: "wrap" }}>
          <button style={styles.btn} onClick={() => applyPreset("deepseek")}>Use DeepSeek Hosted</button>
          <button style={{ ...styles.btn, backgroundColor: "#2c3139" }} onClick={() => applyPreset("lm_studio")}>Use LM Studio Local</button>
        </div>
      </div>
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
          <label style={styles.label}>Base URL</label><input style={styles.input} value={form.baseUrl} onChange={(e) => setForm({ ...form, baseUrl: e.target.value })} />
          <div style={{ fontSize: 12, color: "#8f96a3", marginBottom: 8 }}>
            You can use <code>http://localhost:1234</code> or <code>http://localhost:1234/v1</code>. Aruvi normalizes both.
          </div>
          <label style={styles.label}>API Key / Secret Ref (optional)</label><input style={styles.input} value={form.authSecretRef} onChange={(e) => setForm({ ...form, authSecretRef: e.target.value })} placeholder="Paste API key (stored in Keychain) or ref:provider:... value" />
          <div style={{ fontSize: 12, color: "#8f96a3", marginBottom: 12 }}>
            Keys are stored in macOS Keychain. If Keychain access is blocked during development, runtime falls back to <code>~/.aruvistudio/llm-config.json</code>.
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
      {isLoading ? (<div style={styles.empty}>Loading providers...</div>) : providers && providers.length > 0 ? (
        <>
          <div style={styles.grid}>
            {providers.map((p: ModelProvider) => (
              <div key={p.id} style={styles.card}>
                <div style={styles.name}>{p.name}</div><div style={styles.type}>{p.provider_type}</div><div style={styles.url}>{p.base_url}</div>
                <div style={styles.cardFooter}>
                  <span style={{ ...styles.badge, backgroundColor: p.enabled ? "#1b3a2d" : "#3a1b1b", color: p.enabled ? "#4ec9b0" : "#f44747" }}>{p.enabled ? "Enabled" : "Disabled"}</span>
                  <div style={{ display: "flex", gap: 8 }}>
                    <button style={{ ...styles.btnTest, backgroundColor: "#2c3139" }} onClick={() => {
                      setEditingProvider(p);
                      setProviderEditForm({
                        name: p.name,
                        providerType: p.provider_type,
                        baseUrl: p.base_url,
                        authSecretRef: "",
                        enabled: p.enabled,
                      });
                    }}>Edit</button>
                    <button style={styles.btnTest} onClick={() => testConnectivity(p.id)}>Test Connection</button>
                  </div>
                </div>
                {testResults[p.id] && <div style={{ ...styles.testResult, backgroundColor: testResults[p.id].status === "success" ? "#1b3a2d" : testResults[p.id].status === "error" ? "#3a1b1b" : "#2a2a2a", color: testResults[p.id].status === "success" ? "#4ec9b0" : testResults[p.id].status === "error" ? "#f44747" : "#888" }}>{testResults[p.id].message}</div>}
              </div>
            ))}
          </div>
          <div style={{ ...styles.form, marginTop: 18 }}>
            <div style={styles.sectionTitle}>Registered Models</div>
            {modelDefinitions && modelDefinitions.length > 0 ? (
              <div style={styles.subGrid}>
                {modelDefinitions.map((model: ModelDefinition) => {
                  const provider = providers.find((entry) => entry.id === model.provider_id);
                  return (
                    <div key={model.id} style={styles.modelCard}>
                      <div style={styles.modelName}>{model.name}</div>
                      <div style={styles.modelMeta}>{provider?.name ?? "Unknown provider"}</div>
                      <div style={styles.modelMeta}>Context: {model.context_window ?? "not set"}</div>
                      <div style={styles.modelMeta}>Tags: {model.capability_tags.length > 0 ? model.capability_tags.join(", ") : "none"}</div>
                      {model.notes ? <div style={styles.modelMeta}>{model.notes}</div> : null}
                      <span style={{ ...styles.badge, backgroundColor: model.enabled ? "#1b3a2d" : "#3a1b1b", color: model.enabled ? "#4ec9b0" : "#f44747" }}>
                        {model.enabled ? "Enabled" : "Disabled"}
                      </span>
                      <div style={{ marginTop: 8 }}>
                        <button
                          style={{ ...styles.btnTest, backgroundColor: "#2c3139" }}
                          onClick={() => {
                            setEditingModel(model);
                            setModelEditForm({
                              providerId: model.provider_id,
                              name: model.name,
                              contextWindow: model.context_window ? String(model.context_window) : "",
                              capabilityTags: model.capability_tags.join(", "),
                              notes: model.notes,
                              enabled: model.enabled,
                            });
                          }}
                        >
                          Edit
                        </button>
                      </div>
                    </div>
                  );
                })}
              </div>
            ) : (
              <div style={styles.empty}>No model definitions yet. Add one so agents can be bound to a concrete model.</div>
            )}
          </div>
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

function splitCommaSeparated(value: string): string[] {
  return value
    .split(",")
    .map((entry) => entry.trim())
    .filter(Boolean);
}
