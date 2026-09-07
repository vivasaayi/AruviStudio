import React, { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ScopeBreadcrumb } from "../../../app/layout/ScopeBreadcrumb";
import { attachRepository, browseForRepositoryPath, detachRepository, listAllRepositoryAttachments, listProducts, listRepositories, registerRepository } from "../../../lib/tauri";
import { useWorkspaceStore } from "../../../state/workspaceStore";

const styles: Record<string, React.CSSProperties> = {
  page: { maxWidth: 1180, margin: "0 auto", color: "#202938", paddingBottom: 36 }, header: { display: "flex", justifyContent: "space-between", alignItems: "start", gap: 18, marginBottom: 18 }, title: { margin: 0, fontSize: 30, letterSpacing: "-.035em" }, muted: { color: "#657085", fontSize: 13, lineHeight: 1.45 }, card: { background: "#fff", border: "1px solid #d9dfeb", borderRadius: 12, padding: 16 }, button: { border: 0, borderRadius: 7, background: "#2864dc", color: "#fff", padding: "9px 13px", cursor: "pointer", fontWeight: 650 }, secondary: { border: "1px solid #c8d1e1", borderRadius: 7, background: "#fff", color: "#27344b", padding: "8px 11px", cursor: "pointer", fontWeight: 600 }, field: { width: "100%", padding: "9px 10px", border: "1px solid #b8c4d7", borderRadius: 7, background: "#fff", color: "#263249", boxSizing: "border-box" }, label: { display: "block", color: "#596579", fontSize: 12, fontWeight: 700, marginBottom: 5 }, group: { marginTop: 24 }, grid: { display: "grid", gridTemplateColumns: "repeat(auto-fill,minmax(300px,1fr))", gap: 12 }, path: { color: "#3978c8", fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace", fontSize: 12, overflowWrap: "anywhere", margin: "7px 0" }, badge: { display: "inline-flex", borderRadius: 99, padding: "3px 8px", background: "#e2f5e9", color: "#147545", fontSize: 11, fontWeight: 700 }, error: { color: "#c23636", fontSize: 13, marginTop: 10 },
};

const folderName = (path: string) => path.replace(/\/+$/, "").split("/").filter(Boolean).pop() ?? "workspace";

export function RepositoryListPage() {
  const client = useQueryClient();
  const activeProductId = useWorkspaceStore((state) => state.activeProductId);
  const setActiveProduct = useWorkspaceStore((state) => state.setActiveProduct);
  const [showCreate, setShowCreate] = useState(false);
  const [productId, setProductId] = useState(activeProductId ?? "");
  const [path, setPath] = useState("");
  const products = useQuery({ queryKey: ["products"], queryFn: listProducts });
  const repositories = useQuery({ queryKey: ["repositories"], queryFn: listRepositories });
  const attachments = useQuery({ queryKey: ["repositoryAttachments"], queryFn: listAllRepositoryAttachments });
  React.useEffect(() => { if (activeProductId) setProductId(activeProductId); else if (products.data?.[0]) setProductId(products.data[0].id); }, [activeProductId, products.data]);

  const refresh = () => { void client.invalidateQueries({ queryKey: ["repositories"] }); void client.invalidateQueries({ queryKey: ["repositoryAttachments"] }); void client.invalidateQueries({ queryKey: ["ideScopeRepo"] }); };
  const create = useMutation({
    mutationFn: async () => {
      if (!productId || !path.trim()) throw new Error("Select a product and workspace folder.");
      const repository = await registerRepository({ name: folderName(path), localPath: path, remoteUrl: "", defaultBranch: "main" });
      const current = (attachments.data ?? []).filter((item) => item.scope_type === "product" && item.scope_id === productId);
      await attachRepository({ scopeType: "product", scopeId: productId, repositoryId: repository.id, isDefault: current.length === 0 });
    },
    onSuccess: () => { refresh(); setPath(""); setShowCreate(false); setActiveProduct(productId); },
  });
  const makeDefault = useMutation({ mutationFn: ({ scopeId, repositoryId }: { scopeId: string; repositoryId: string }) => attachRepository({ scopeType: "product", scopeId, repositoryId, isDefault: true }), onSuccess: refresh });
  const detach = useMutation({ mutationFn: ({ scopeId, repositoryId }: { scopeId: string; repositoryId: string }) => detachRepository({ scopeType: "product", scopeId, repositoryId }), onSuccess: refresh });

  const repositoryById = useMemo(() => new Map((repositories.data ?? []).map((repository) => [repository.id, repository])), [repositories.data]);
  const groups = useMemo(() => (products.data ?? []).map((product) => ({ product, workspaces: (attachments.data ?? []).filter((item) => item.scope_type === "product" && item.scope_id === product.id).map((attachment) => ({ attachment, repository: repositoryById.get(attachment.repository_id) })).filter((entry) => entry.repository) })).filter((group) => group.workspaces.length), [attachments.data, products.data, repositoryById]);
  const currentProduct = (products.data ?? []).find((product) => product.id === activeProductId);

  return <div style={styles.page}>
    <div style={styles.header}><div><h1 style={styles.title}>Workspaces</h1><div style={{ marginTop: 8 }}><ScopeBreadcrumb label="Current Scope" productName={currentProduct?.name} /></div><p style={styles.muted}>Workspaces are repositories attached to products. A repository can be shared by multiple products, and each product has one default.</p></div><button style={styles.button} onClick={() => setShowCreate((value) => !value)}>{showCreate ? "Cancel" : "+ Create / Attach Workspace"}</button></div>
    {showCreate && <section style={{ ...styles.card, marginBottom: 20 }}><strong>Create / Attach Workspace</strong><p style={styles.muted}>Choose a product and folder. An existing registered folder is reused; otherwise its folder name becomes the workspace name.</p><div style={{ display: "grid", gridTemplateColumns: "minmax(220px,1fr) minmax(320px,2fr) auto", gap: 10, alignItems: "end" }}><div><label style={styles.label}>Product</label><select style={styles.field} value={productId} onChange={(event) => setProductId(event.target.value)}><option value="">Select product</option>{(products.data ?? []).map((product) => <option key={product.id} value={product.id}>{product.name}</option>)}</select></div><div><label style={styles.label}>Workspace folder</label><input style={styles.field} value={path || "No folder selected"} readOnly /></div><button style={styles.secondary} onClick={() => void browseForRepositoryPath().then((selected) => { if (selected) setPath(selected); })}>Choose Folder…</button></div>{path && <div style={{ ...styles.muted, marginTop: 9 }}>Workspace name: <strong>{folderName(path)}</strong></div>}<button style={{ ...styles.button, marginTop: 13 }} disabled={!productId || !path || create.isPending} onClick={() => create.mutate()}>{create.isPending ? "Attaching…" : "Create / Attach Workspace"}</button>{create.error && <div style={styles.error}>{String(create.error)}</div>}</section>}
    {!groups.length && !attachments.isLoading && <div style={{ ...styles.card, ...styles.muted }}>No product workspaces yet. Create or attach the first workspace.</div>}
    {groups.map(({ product, workspaces }) => <section key={product.id} style={styles.group}><div style={{ display: "flex", gap: 9, alignItems: "center", marginBottom: 10 }}><h2 style={{ margin: 0, fontSize: 20 }}>{product.name}</h2><span style={{ ...styles.badge, background: "#eef2f8", color: "#526078" }}>{workspaces.length} {workspaces.length === 1 ? "workspace" : "workspaces"}</span></div><div style={styles.grid}>{workspaces.map(({ attachment, repository }) => repository && <article key={attachment.id} style={styles.card}><div style={{ display: "flex", justifyContent: "space-between", gap: 8 }}><strong>{repository.name}</strong>{attachment.is_default && <span style={styles.badge}>Default</span>}</div><div style={styles.path}>{repository.local_path}</div><div style={styles.muted}>Branch: {repository.default_branch}</div><div style={{ display: "flex", gap: 8, marginTop: 13, flexWrap: "wrap" }}>{!attachment.is_default && <button style={styles.button} onClick={() => makeDefault.mutate({ scopeId: product.id, repositoryId: repository.id })}>Make default</button>}<button style={styles.secondary} onClick={() => detach.mutate({ scopeId: product.id, repositoryId: repository.id })}>Detach from product</button></div></article>)}</div></section>)}
  </div>;
}
