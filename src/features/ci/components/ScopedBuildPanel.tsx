import React, { useEffect, useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { listProducts, listRepositories } from "../../../lib/tauri";
import {
  listCiRepositoryBranches,
  listCiRepositoryCommits,
  listCiTargetBindings,
  queueLocalPreviewReference,
  queueLocalReleaseReference,
} from "../../../lib/tauri/ci";
import { useWorkspaceStore } from "../../../state/workspaceStore";

type SelectionMode = "branch" | "commit";
const field: React.CSSProperties = { minWidth: 180, flex: "1 1 210px", padding: "9px", border: "1px solid #b8c4d7", borderRadius: 7, background: "#fff", color: "#263249" };
const button: React.CSSProperties = { border: 0, borderRadius: 7, background: "#2864dc", color: "#fff", padding: "9px 12px", cursor: "pointer", fontWeight: 650 };

/** Product-scoped, immutable build handoff. Branches are resolved locally to a SHA. */
export function ScopedBuildPanel({ online, targets }: { online: boolean; targets: Array<{ id: string }> }) {
  const client = useQueryClient();
  const activeProductId = useWorkspaceStore((state) => state.activeProductId);
  const setActiveProduct = useWorkspaceStore((state) => state.setActiveProduct);
  const [productId, setProductId] = useState(activeProductId ?? "");
  const [repositoryId, setRepositoryId] = useState("");
  const [mode, setMode] = useState<SelectionMode>("branch");
  const [branch, setBranch] = useState("");
  const [commit, setCommit] = useState("");
  const [releaseTarget, setReleaseTarget] = useState<"aruvi-studio" | "aruvi-studio-intel">("aruvi-studio");
  const products = useQuery({ queryKey: ["products"], queryFn: listProducts });
  const repositories = useQuery({ queryKey: ["repositories"], queryFn: listRepositories });
  const bindings = useQuery({ queryKey: ["ci-target-bindings"], queryFn: listCiTargetBindings });
  const branches = useQuery({ queryKey: ["ci-repository-branches", repositoryId], queryFn: () => listCiRepositoryBranches(repositoryId), enabled: Boolean(repositoryId) });
  const commits = useQuery({ queryKey: ["ci-repository-commits", repositoryId], queryFn: () => listCiRepositoryCommits(repositoryId), enabled: Boolean(repositoryId) });

  const scopedBindings = useMemo(() => (bindings.data ?? []).filter((binding) => binding.product_id === productId), [bindings.data, productId]);
  const scopedRepositoryIds = useMemo(() => new Set(scopedBindings.map((binding) => binding.repository_id)), [scopedBindings]);
  const scopedRepositories = useMemo(() => (repositories.data ?? []).filter((repository) => scopedRepositoryIds.has(repository.id)), [repositories.data, scopedRepositoryIds]);
  const selectedProduct = (products.data ?? []).find((product) => product.id === productId);
  const previewConfigured = scopedBindings.some((binding) => binding.repository_id === repositoryId && binding.target_id === "aruvi-studio-preview") && targets.some((target) => target.id === "aruvi-studio-preview");
  const releaseConfigured = targets.some((target) => target.id === releaseTarget);
  const reference = mode === "branch" ? { branch } : { commit };
  const canQueue = Boolean(productId && repositoryId && (mode === "branch" ? branch : commit) && online);

  useEffect(() => { if (activeProductId) setProductId(activeProductId); }, [activeProductId]);
  useEffect(() => {
    if (!scopedRepositories.some((repository) => repository.id === repositoryId)) setRepositoryId(scopedRepositories[0]?.id ?? "");
  }, [repositoryId, scopedRepositories]);
  useEffect(() => { if (mode === "branch" && branches.data?.length && !branches.data.some((item) => item.name === branch)) setBranch(branches.data[0].name); }, [branches.data, branch, mode]);
  useEffect(() => { if (mode === "commit" && commits.data?.length && !commits.data.some((item) => item.sha === commit)) setCommit(commits.data[0].sha); }, [commits.data, commit, mode]);

  const preview = useMutation({
    mutationFn: () => queueLocalPreviewReference({ repositoryId, productId, ...reference }),
    onSuccess: () => void client.invalidateQueries({ queryKey: ["local-ci-runs"] }),
  });
  const release = useMutation({
    mutationFn: () => queueLocalReleaseReference({ target: releaseTarget, repositoryId, productId, ...reference }),
    onSuccess: () => void client.invalidateQueries({ queryKey: ["local-ci-runs"] }),
  });
  const selectedRevision = mode === "branch" ? branches.data?.find((item) => item.name === branch)?.commit : commits.data?.find((item) => item.sha === commit);

  return <section style={{ background: "#fff", border: "1px solid #d9dfeb", borderRadius: 12, padding: 16, marginBottom: 16 }}>
    <div style={{ color: "#657085", fontSize: 11, fontWeight: 750, letterSpacing: ".08em", textTransform: "uppercase", marginBottom: 8 }}>Current product scope</div>
    <div style={{ display: "flex", gap: 8, flexWrap: "wrap", alignItems: "center" }}>
      <select aria-label="CI product scope" style={field} value={productId} onChange={(event) => { setProductId(event.target.value); setActiveProduct(event.target.value || null); }}>
        <option value="">Select a product</option>{(products.data ?? []).map((product) => <option key={product.id} value={product.id}>{product.name}</option>)}
      </select>
      <select aria-label="CI scoped repository" style={field} value={repositoryId} onChange={(event) => setRepositoryId(event.target.value)} disabled={!productId}>
        <option value="">{productId ? "Select an approved repository" : "Select a product first"}</option>{scopedRepositories.map((repository) => <option key={repository.id} value={repository.id}>{repository.name}</option>)}
      </select>
      {selectedProduct && <span style={{ color: "#4268ad", fontWeight: 650 }}>{selectedProduct.name}</span>}
    </div>
    {productId && !scopedRepositories.length && <p style={{ color: "#9a5a05", fontSize: 13 }}>This product has no approved CI repository binding yet. Add one in Operations wallboard.</p>}
    <hr style={{ border: 0, borderTop: "1px solid #e4e8f0", margin: "16px 0" }} />
    <strong>Choose the revision to build</strong><p style={{ color: "#657085", fontSize: 13, marginTop: 5 }}>A branch is resolved to its current immutable commit when you queue it; choosing history builds that exact commit.</p>
    <div style={{ display: "flex", gap: 8, flexWrap: "wrap", alignItems: "center" }}>
      <select aria-label="Revision selection method" style={{ ...field, flex: "0 1 185px" }} value={mode} onChange={(event) => setMode(event.target.value as SelectionMode)} disabled={!repositoryId}><option value="branch">Latest on branch</option><option value="commit">Specific commit</option></select>
      {mode === "branch" ? <select aria-label="Git branch" style={field} value={branch} onChange={(event) => setBranch(event.target.value)} disabled={!repositoryId || branches.isLoading}><option value="">Select a branch</option>{(branches.data ?? []).map((item) => <option key={item.name} value={item.name}>{item.name} · {item.commit.sha.slice(0, 12)}</option>)}</select> : <select aria-label="Git commit" style={{ ...field, flex: "2 1 420px" }} value={commit} onChange={(event) => setCommit(event.target.value)} disabled={!repositoryId || commits.isLoading}><option value="">Select a commit from local history</option>{(commits.data ?? []).map((item) => <option key={item.sha} value={item.sha}>{item.sha.slice(0, 12)} · {item.summary}</option>)}</select>}
    </div>
    {selectedRevision && <div style={{ marginTop: 9, color: "#485974", fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace", fontSize: 12 }}>Will build {selectedRevision.sha} · {selectedRevision.summary}</div>}
    <div style={{ display: "flex", gap: 8, flexWrap: "wrap", marginTop: 16, alignItems: "center" }}>
      <button style={button} disabled={!canQueue || !previewConfigured || preview.isPending} onClick={() => preview.mutate()}>{preview.isPending ? "Queueing…" : "Build Preview"}</button>
      <select aria-label="Production architecture" style={{ ...field, flex: "0 1 220px" }} value={releaseTarget} onChange={(event) => setReleaseTarget(event.target.value as typeof releaseTarget)}><option value="aruvi-studio">Apple Silicon artifact</option><option value="aruvi-studio-intel">Intel artifact</option></select>
      <button style={{ ...button, background: "#34425c" }} disabled={!canQueue || !releaseConfigured || release.isPending} onClick={() => release.mutate()}>{release.isPending ? "Queueing…" : "Build production artifact"}</button>
    </div>
    <p style={{ color: "#657085", fontSize: 12, marginBottom: 0 }}>Production build creates a signed, verified ZIP only. It cannot install, replace, or launch the production app.</p>
    {(preview.error || release.error) && <div style={{ color: "#c23636", marginTop: 8, fontSize: 13 }}>{String(preview.error ?? release.error)}</div>}
  </section>;
}
