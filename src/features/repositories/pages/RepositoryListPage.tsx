import React, { useMemo, useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { ScopeBreadcrumb } from "../../../app/layout/ScopeBreadcrumb";
import { useWorkspaceStore } from "../../../state/workspaceStore";
import { browseForRepositoryPath, deleteRepository, getProductTree, listProducts, listRepositories, registerRepository } from "../../../lib/tauri";
import type { Repository } from "../../../lib/types";

const styles: Record<string, React.CSSProperties> = {
  page: { maxWidth: 960, margin: "0 auto" },
  header: { display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 24 },
  title: { fontSize: 20, fontWeight: 600, color: "#e0e0e0" },
  btn: { padding: "6px 16px", fontSize: 13, backgroundColor: "#0e639c", color: "#fff", border: "none", borderRadius: 4, cursor: "pointer" },
  btnDanger: { padding: "4px 10px", fontSize: 12, backgroundColor: "#6c2020", color: "#fff", border: "none", borderRadius: 4, cursor: "pointer" },
  grid: { display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(300px, 1fr))", gap: 16 },
  card: { backgroundColor: "#252526", borderRadius: 8, padding: 16, border: "1px solid #333" },
  cardName: { fontSize: 15, fontWeight: 600, marginBottom: 8, color: "#e0e0e0" },
  cardPath: { fontSize: 12, color: "#569cd6", marginBottom: 4, fontFamily: "monospace" },
  cardRemote: { fontSize: 11, color: "#888", marginBottom: 4, wordBreak: "break-all" as const },
  cardBranch: { fontSize: 12, color: "#4ec9b0", marginBottom: 12 },
  empty: { textAlign: "center" as const, color: "#666", padding: 40, fontSize: 14 },
  form: { backgroundColor: "#252526", padding: 20, borderRadius: 8, marginBottom: 24, border: "1px solid #333" },
  input: { width: "100%", padding: "8px 12px", backgroundColor: "#1e1e1e", border: "1px solid #444", borderRadius: 4, color: "#e0e0e0", fontSize: 13, marginBottom: 12, boxSizing: "border-box" as const },
  label: { fontSize: 12, color: "#999", display: "block", marginBottom: 4 },
  inputRow: { display: "grid", gridTemplateColumns: "1fr auto", gap: 8, alignItems: "start", marginBottom: 12 },
  inputNoMargin: { width: "100%", padding: "8px 12px", backgroundColor: "#1e1e1e", border: "1px solid #444", borderRadius: 4, color: "#e0e0e0", fontSize: 13, boxSizing: "border-box" as const },
  helperText: { fontSize: 11, color: "#888", marginTop: 4 },
  errorText: { fontSize: 12, color: "#ff7b72", marginBottom: 12 },
};

export function RepositoryListPage() {
  const queryClient = useQueryClient();
  const { activeProductId, activeModuleId, activeCapabilityId } = useWorkspaceStore();
  const [showForm, setShowForm] = useState(false);
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [form, setForm] = useState({ name: "", localPath: "", remoteUrl: "", defaultBranch: "main" });
  const [formError, setFormError] = useState<string | null>(null);

  const { data: repos, isLoading } = useQuery({ queryKey: ["repositories"], queryFn: listRepositories });
  const { data: products = [] } = useQuery({ queryKey: ["products"], queryFn: listProducts });
  const { data: activeProductTree } = useQuery({
    queryKey: ["repositoryProductTree", activeProductId],
    queryFn: () => getProductTree(activeProductId!),
    enabled: !!activeProductId,
  });
  const activeProduct = products.find((product) => product.id === activeProductId) ?? null;
  const activeModule = activeProductTree?.modules.find((moduleTree) => moduleTree.module.id === activeModuleId)?.module ?? null;
  const activeCapability = useMemo(() => {
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

  const createMutation = useMutation({
    mutationFn: () => registerRepository(form),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["repositories"] });
      setForm({ name: "", localPath: "", remoteUrl: "", defaultBranch: "main" });
      setShowForm(false);
      setFormError(null);
    },
    onError: (error) => setFormError(String(error)),
  });

  const deleteMutation = useMutation({
    mutationFn: (id: string) => deleteRepository(id),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["repositories"] }),
  });

  return (
    <div style={styles.page}>
      <div style={styles.header}>
        <div>
          <h1 style={styles.title}>Workspaces</h1>
          <ScopeBreadcrumb
            label="Current Scope"
            productName={activeProduct?.name}
            moduleName={activeModule?.name}
            capabilityName={activeCapability?.name}
          />
        </div>
        <button style={styles.btn} onClick={() => setShowForm(!showForm)}>{showForm ? "Cancel" : "+ Add Workspace"}</button>
      </div>
      {showForm && (
        <div style={styles.form}>
          {formError && <div style={styles.errorText}>{formError}</div>}
          <label style={styles.label}>Workspace Name</label><input style={styles.input} value={form.name} onChange={(e) => setForm({ ...form, name: e.target.value })} placeholder="Workspace name" />
          <label style={styles.label}>Local Path</label>
          <div style={styles.inputRow}>
            <div>
              <input style={styles.inputNoMargin} value={form.localPath} onChange={(e) => setForm({ ...form, localPath: e.target.value })} placeholder="/path/to/workspace" />
              <div style={styles.helperText}>Browse for a local workspace folder or paste the path manually.</div>
            </div>
            <button
              style={styles.btn}
              onClick={async () => {
                setFormError(null);
                try {
                  const selectedPath = await browseForRepositoryPath();
                  if (selectedPath) {
                    setForm((current) => ({ ...current, localPath: selectedPath }));
                  }
                } catch (error) {
                  setFormError(String(error));
                }
              }}
            >
              Browse…
            </button>
          </div>
          <button
            style={{ ...styles.btn, backgroundColor: "#2c3139", marginBottom: 12 }}
            onClick={() => setShowAdvanced((current) => !current)}
          >
            {showAdvanced ? "Hide Advanced" : "Show Advanced"}
          </button>
          {showAdvanced && (
            <>
              <label style={styles.label}>Remote URL</label><input style={styles.input} value={form.remoteUrl} onChange={(e) => setForm({ ...form, remoteUrl: e.target.value })} placeholder="https://github.com/..." />
              <label style={styles.label}>Default Branch</label><input style={styles.input} value={form.defaultBranch} onChange={(e) => setForm({ ...form, defaultBranch: e.target.value })} />
            </>
          )}
          <button style={styles.btn} onClick={() => createMutation.mutate()} disabled={!form.name || !form.localPath}>{createMutation.isPending ? "Adding..." : "Add Workspace"}</button>
        </div>
      )}
      {isLoading ? (<div style={styles.empty}>Loading workspaces...</div>) : repos && repos.length > 0 ? (
        <div style={styles.grid}>
          {repos.map((r: Repository) => (
            <div key={r.id} style={styles.card}>
              <div style={styles.cardName}>{r.name}</div><div style={styles.cardPath}>{r.local_path}</div>
              <div style={styles.cardBranch}>Version history enabled</div>
              {showAdvanced && (
                <>
                  <div style={styles.cardRemote}>{r.remote_url || "No remote configured"}</div>
                  <div style={styles.cardBranch}>Branch: {r.default_branch}</div>
                </>
              )}
              <button style={styles.btnDanger} onClick={() => deleteMutation.mutate(r.id)}>Remove</button>
            </div>
          ))}
        </div>
      ) : (<div style={styles.empty}>No workspaces added yet. Add a workspace to get started.</div>)}
    </div>
  );
}
