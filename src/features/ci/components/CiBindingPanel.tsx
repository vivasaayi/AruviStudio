import React, { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { listProducts, listRepositories, listRepositoryAttachments } from "../../../lib/tauri";
import { listCiTargetBindings, saveCiTargetBinding } from "../../../lib/tauri/ci";

const field: React.CSSProperties = { padding: "8px", border: "1px solid #b8c4d7", borderRadius: 6, background: "white", color: "#263249" };
const normalizePath = (value?: string) => (value ?? "").replace(/\/+$/, "");

export function CiBindingPanel({ targets }: { targets: Array<{ id: string; profile: string; repository?: string }> }) {
  const client = useQueryClient();
  const [productId, setProductId] = useState("");
  const [repositoryId, setRepositoryId] = useState("");
  const [targetId, setTargetId] = useState("");
  const products = useQuery({ queryKey: ["products"], queryFn: listProducts });
  const repositories = useQuery({ queryKey: ["repositories"], queryFn: listRepositories });
  const bindings = useQuery({ queryKey: ["ci-target-bindings"], queryFn: listCiTargetBindings });
  const attachments = useQuery({ queryKey: ["repositoryAttachments", "product", productId], queryFn: () => listRepositoryAttachments({ scopeType: "product", scopeId: productId }), enabled: Boolean(productId) });
  const attachedIds = new Set((attachments.data ?? []).map((item) => item.repository_id));
  const selectedRepository = (repositories.data ?? []).find((item) => item.id === repositoryId);
  const matchingTargets = selectedRepository ? targets.filter((target) => normalizePath(target.repository) === normalizePath(selectedRepository.local_path) && target.id.endsWith("-preview")) : [];
  const save = useMutation({ mutationFn: () => saveCiTargetBinding({ product_id: productId, repository_id: repositoryId, target_id: targetId, auto_preview: true }), onSuccess: () => void client.invalidateQueries({ queryKey: ["ci-target-bindings"] }) });

  return <section style={{ background: "#fff", border: "1px solid #d9dfeb", borderRadius: 12, padding: 16, marginBottom: 16 }}>
    <strong>Preview approval</strong><p style={{ color: "#657085", fontSize: 13 }}>Approve an attached product workspace only for an Aruvici target configured for that same local repository.</p>
    <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
      <select aria-label="Product" style={field} value={productId} onChange={(event) => { setProductId(event.target.value); setRepositoryId(""); setTargetId(""); }}><option value="">Select product</option>{(products.data ?? []).map((product) => <option key={product.id} value={product.id}>{product.name}</option>)}</select>
      <select aria-label="Repository" style={field} value={repositoryId} onChange={(event) => { setRepositoryId(event.target.value); setTargetId(""); }} disabled={!productId}><option value="">Select attached workspace</option>{(repositories.data ?? []).filter((repository) => attachedIds.has(repository.id)).map((repository) => <option key={repository.id} value={repository.id}>{repository.name}</option>)}</select>
      <select aria-label="Preview target" style={field} value={targetId} onChange={(event) => setTargetId(event.target.value)} disabled={!repositoryId}><option value="">Select matching Preview target</option>{matchingTargets.map((target) => <option key={target.id} value={target.id}>{target.id}</option>)}</select>
      <button disabled={!productId || !repositoryId || !targetId || save.isPending} onClick={() => save.mutate()} style={{ ...field, background: "#2864dc", color: "white", border: 0, fontWeight: 650, opacity: targetId ? 1 : .45 }}>Approve Preview</button>
    </div>
    {repositoryId && !matchingTargets.length && <div style={{ color: "#9a5a05", fontSize: 13, marginTop: 9 }}>No Preview target is registered in Aruvici for this workspace.</div>}
    <div style={{ marginTop: 12, display: "grid", gap: 6 }}>{(bindings.data ?? []).map((binding) => <div key={binding.id} style={{ fontSize: 12, color: "#485974" }}>{binding.product_id.slice(0, 8)} · {binding.repository_id.slice(0, 8)} → <strong>{binding.target_id}</strong> · automatic</div>)}</div>
    {save.error && <div style={{ color: "#c23636", marginTop: 8 }}>{String(save.error)}</div>}
  </section>;
}
