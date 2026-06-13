import React, { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  createStrategyNode,
  deleteStrategyNode,
  listProductDependencies,
  listProducts,
  listProductStrategyLinks,
  listStrategyNodes,
  updateStrategyNode,
} from "../../../lib/tauri";
import type { Product, ProductDependency, ProductStrategyLink, StrategyNode, StrategyNodeKind } from "../../../lib/types";

type PortfolioTab = "summary" | "manage";
type StrategyTreeNode = StrategyNode & { children: StrategyTreeNode[] };
type StrategyDialogMode = "closed" | "create" | "edit";
type StrategyFormState = {
  parentNodeId: string;
  nodeKind: StrategyNodeKind;
  name: string;
  description: string;
  ownerLabel: string;
};

const emptyStrategyForm: StrategyFormState = {
  parentNodeId: "",
  nodeKind: "strategic_area",
  name: "",
  description: "",
  ownerLabel: "",
};

const styles: Record<string, React.CSSProperties> = {
  page: { display: "flex", flexDirection: "column", gap: 12, height: "100%", minHeight: 0 },
  header: { display: "flex", justifyContent: "space-between", alignItems: "flex-start", gap: 16 },
  title: { margin: 0, fontSize: 22, lineHeight: 1.15, color: "#111827", fontWeight: 800 },
  subtitle: { marginTop: 4, color: "#6b7280", fontSize: 13, lineHeight: 1.4 },
  tabBar: { display: "flex", gap: 8, padding: 4, border: "1px solid #d9e0ea", borderRadius: 10, backgroundColor: "#f8fafc", width: "fit-content" },
  tab: { border: "1px solid transparent", backgroundColor: "transparent", color: "#4b5563", borderRadius: 8, padding: "8px 13px", fontSize: 12, fontWeight: 800, cursor: "pointer" },
  tabActive: { border: "1px solid #2563eb", backgroundColor: "#eff6ff", color: "#1d4ed8", borderRadius: 8, padding: "8px 13px", fontSize: 12, fontWeight: 800, cursor: "pointer" },
  statGrid: { display: "grid", gridTemplateColumns: "repeat(4, minmax(0, 1fr))", gap: 10 },
  statCard: { border: "1px solid #d9e0ea", borderRadius: 10, backgroundColor: "#ffffff", padding: 12 },
  statLabel: { fontSize: 11, color: "#6b7280", textTransform: "uppercase", letterSpacing: "0.06em", fontWeight: 800 },
  statValue: { fontSize: 24, color: "#111827", fontWeight: 800, marginTop: 4 },
  summaryGrid: { display: "grid", gridTemplateColumns: "minmax(0, 1.35fr) minmax(320px, 0.65fr)", gap: 12, flex: 1, minHeight: 0 },
  manageGrid: { display: "grid", gridTemplateColumns: "minmax(0, 1fr) 320px", gap: 12, flex: 1, minHeight: 0 },
  panel: { border: "1px solid #d9e0ea", borderRadius: 12, backgroundColor: "#ffffff", overflow: "hidden", minHeight: 0, display: "flex", flexDirection: "column" },
  panelHeader: { padding: "12px 14px", borderBottom: "1px solid #d9e0ea", display: "flex", justifyContent: "space-between", alignItems: "center", gap: 10 },
  panelTitle: { fontSize: 12, color: "#6b7280", textTransform: "uppercase", letterSpacing: "0.08em", fontWeight: 800 },
  panelBody: { padding: 14, overflow: "auto", minHeight: 0, display: "flex", flexDirection: "column", gap: 12 },
  controlRow: { display: "flex", gap: 8, flexWrap: "wrap", alignItems: "center" },
  button: { border: "1px solid #2563eb", backgroundColor: "#2563eb", color: "#ffffff", borderRadius: 8, padding: "8px 11px", fontSize: 12, fontWeight: 800, cursor: "pointer" },
  ghostButton: { border: "1px solid #cfd8e3", backgroundColor: "#f8fafc", color: "#111827", borderRadius: 8, padding: "7px 10px", fontSize: 12, fontWeight: 800, cursor: "pointer" },
  dangerButton: { border: "1px solid #dc2626", backgroundColor: "#ffffff", color: "#dc2626", borderRadius: 8, padding: "7px 10px", fontSize: 12, fontWeight: 800, cursor: "pointer" },
  section: { border: "1px solid #d9e0ea", borderRadius: 10, padding: 10, backgroundColor: "#f8fafc" },
  sectionTitle: { fontSize: 12, color: "#111827", fontWeight: 800, marginBottom: 8 },
  productGrid: { display: "grid", gridTemplateColumns: "repeat(2, minmax(0, 1fr))", gap: 10 },
  productCard: { border: "1px solid #d9e0ea", borderRadius: 10, backgroundColor: "#ffffff", padding: 12, display: "flex", flexDirection: "column", gap: 8 },
  productTitle: { fontSize: 14, color: "#111827", fontWeight: 800 },
  productText: { fontSize: 12, color: "#4b5563", lineHeight: 1.45 },
  badgeRow: { display: "flex", gap: 6, flexWrap: "wrap" },
  badge: { borderRadius: 999, backgroundColor: "#eef2ff", color: "#3730a3", padding: "3px 8px", fontSize: 11, fontWeight: 700 },
  badgeGreen: { borderRadius: 999, backgroundColor: "#dcfce7", color: "#166534", padding: "3px 8px", fontSize: 11, fontWeight: 700 },
  badgeMuted: { borderRadius: 999, backgroundColor: "#f3f4f6", color: "#4b5563", padding: "3px 8px", fontSize: 11, fontWeight: 700 },
  tree: { display: "flex", flexDirection: "column", gap: 8 },
  treeNode: { border: "1px solid #d9e0ea", borderRadius: 10, backgroundColor: "#ffffff", overflow: "hidden" },
  treeNodeActive: { border: "1px solid #2563eb", borderRadius: 10, backgroundColor: "#eff6ff", overflow: "hidden" },
  treeHeader: { display: "grid", gridTemplateColumns: "30px minmax(0, 1fr) auto", gap: 8, alignItems: "center", padding: "10px 11px" },
  toggle: { width: 24, height: 24, borderRadius: 7, border: "1px solid #cfd8e3", backgroundColor: "#f8fafc", color: "#111827", fontWeight: 800, cursor: "pointer" },
  togglePlaceholder: { width: 24, height: 24 },
  treeTitle: { color: "#111827", fontWeight: 800, fontSize: 13 },
  treeMeta: { color: "#6b7280", fontSize: 11, marginTop: 3, lineHeight: 1.35 },
  treeActions: { display: "flex", gap: 6, alignItems: "center" },
  treeChildren: { marginLeft: 24, padding: "0 0 10px 14px", borderLeft: "1px solid #d9e0ea", display: "flex", flexDirection: "column", gap: 8 },
  empty: { color: "#6b7280", fontSize: 13, padding: 16, textAlign: "center" },
  error: { color: "#dc2626", fontSize: 12 },
  label: { fontSize: 11, color: "#6b7280", fontWeight: 800, marginBottom: 4 },
  input: { width: "100%", boxSizing: "border-box", borderRadius: 8, border: "1px solid #cfd8e3", backgroundColor: "#ffffff", color: "#111827", padding: "8px 10px", fontSize: 12, marginBottom: 10 },
  textarea: { width: "100%", boxSizing: "border-box", borderRadius: 8, border: "1px solid #cfd8e3", backgroundColor: "#ffffff", color: "#111827", padding: "8px 10px", fontSize: 12, minHeight: 84, resize: "vertical", marginBottom: 10 },
  modalBackdrop: { position: "fixed", inset: 0, backgroundColor: "rgba(15, 23, 42, 0.42)", display: "flex", alignItems: "center", justifyContent: "center", padding: 24, zIndex: 50 },
  modal: { width: "min(620px, 100%)", maxHeight: "84vh", overflow: "hidden", borderRadius: 12, backgroundColor: "#ffffff", border: "1px solid #d9e0ea", boxShadow: "0 24px 60px rgba(15, 23, 42, 0.22)" },
  modalHeader: { display: "flex", alignItems: "center", justifyContent: "space-between", gap: 8, padding: "13px 15px", borderBottom: "1px solid #d9e0ea" },
  modalTitle: { fontSize: 14, fontWeight: 800, color: "#111827" },
  modalBody: { padding: 15, maxHeight: "calc(84vh - 58px)", overflow: "auto" },
};

const strategyKindLabels: Record<StrategyNodeKind, string> = {
  strategic_area: "Strategic Area",
  domain: "Domain",
  subdomain: "Subdomain",
};

export function PortfolioPage() {
  const queryClient = useQueryClient();
  const { data: products = [], isLoading: productsLoading } = useQuery({ queryKey: ["products"], queryFn: listProducts });
  const { data: strategyNodes = [], isLoading: strategyLoading } = useQuery({ queryKey: ["strategy-nodes"], queryFn: listStrategyNodes });
  const { data: strategyLinks = [] } = useQuery({ queryKey: ["product-strategy-links"], queryFn: listProductStrategyLinks });
  const { data: dependencies = [] } = useQuery({ queryKey: ["product-dependencies"], queryFn: listProductDependencies });

  const [activeTab, setActiveTab] = useState<PortfolioTab>("summary");
  const [selectedStrategyNodeId, setSelectedStrategyNodeId] = useState<string | null>(null);
  const [expandedNodeIds, setExpandedNodeIds] = useState<Set<string>>(() => new Set());
  const [strategyDialogMode, setStrategyDialogMode] = useState<StrategyDialogMode>("closed");
  const [strategyForm, setStrategyForm] = useState<StrategyFormState>(emptyStrategyForm);
  const [editingNodeId, setEditingNodeId] = useState<string | null>(null);
  const [deleteCandidate, setDeleteCandidate] = useState<StrategyNode | null>(null);
  const [deleteConfirmName, setDeleteConfirmName] = useState("");
  const [deleteAcknowledge, setDeleteAcknowledge] = useState(false);
  const [formError, setFormError] = useState<string | null>(null);

  const strategyTree = useMemo(() => buildStrategyTree(strategyNodes), [strategyNodes]);
  const selectedStrategyNode = strategyNodes.find((node) => node.id === selectedStrategyNodeId) ?? null;
  const strategyNodeById = useMemo(() => new Map(strategyNodes.map((node) => [node.id, node])), [strategyNodes]);
  const unlinkedProducts = products.filter((product) => !strategyLinks.some((link) => link.product_id === product.id));
  const visibleProductIds = useMemo(() => {
    if (!selectedStrategyNodeId) {
      return null;
    }
    const descendantIds = new Set(collectStrategySubtreeIds(strategyTree, selectedStrategyNodeId));
    return new Set(strategyLinks.filter((link) => descendantIds.has(link.strategy_node_id)).map((link) => link.product_id));
  }, [selectedStrategyNodeId, strategyLinks, strategyTree]);
  const visibleProducts = visibleProductIds ? products.filter((product) => visibleProductIds.has(product.id)) : products;
  const selectedNodeProductCount = selectedStrategyNode
    ? countProductsForStrategy(findTreeNode(strategyTree, selectedStrategyNode.id), strategyLinks)
    : products.length;

  const invalidateStrategy = async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ["strategy-nodes"] }),
      queryClient.invalidateQueries({ queryKey: ["product-strategy-links"] }),
    ]);
  };

  const createStrategyMutation = useMutation({
    mutationFn: () => createStrategyNode({
      parentNodeId: strategyForm.parentNodeId || null,
      nodeKind: strategyForm.nodeKind,
      name: strategyForm.name.trim(),
      description: strategyForm.description.trim(),
      ownerLabel: strategyForm.ownerLabel.trim(),
    }),
    onSuccess: async (created) => {
      setSelectedStrategyNodeId(created.id);
      setExpandedNodeIds((current) => new Set([...current, created.parent_node_id ?? created.id]));
      closeStrategyDialog();
      await invalidateStrategy();
    },
    onError: (error) => setFormError(formatError(error)),
  });

  const updateStrategyMutation = useMutation({
    mutationFn: () => updateStrategyNode({
      id: editingNodeId!,
      parentNodeId: strategyForm.parentNodeId || null,
      clearParent: !strategyForm.parentNodeId,
      nodeKind: strategyForm.nodeKind,
      name: strategyForm.name.trim(),
      description: strategyForm.description.trim(),
      ownerLabel: strategyForm.ownerLabel.trim(),
    }),
    onSuccess: async (updated) => {
      setSelectedStrategyNodeId(updated.id);
      closeStrategyDialog();
      await invalidateStrategy();
    },
    onError: (error) => setFormError(formatError(error)),
  });

  const deleteStrategyMutation = useMutation({
    mutationFn: (id: string) => deleteStrategyNode(id),
    onSuccess: async (_, deletedId) => {
      if (selectedStrategyNodeId === deletedId) {
        setSelectedStrategyNodeId(null);
      }
      setDeleteCandidate(null);
      setDeleteConfirmName("");
      setDeleteAcknowledge(false);
      setFormError(null);
      await invalidateStrategy();
    },
    onError: (error) => setFormError(formatError(error)),
  });

  const productLabel = (productId: string) => products.find((product) => product.id === productId)?.name ?? "Unknown product";
  const strategyLabel = (strategyNodeId: string) => strategyNodeById.get(strategyNodeId)?.name ?? "Unknown strategy";
  const selectedProductLinks = (product: Product) => strategyLinks.filter((link) => link.product_id === product.id);

  const openCreateRootDialog = () => {
    setStrategyForm({ ...emptyStrategyForm, nodeKind: "strategic_area" });
    setEditingNodeId(null);
    setFormError(null);
    setStrategyDialogMode("create");
  };

  const openCreateChildDialog = (parent: StrategyNode) => {
    const childKind = getChildKind(parent.node_kind);
    if (!childKind) {
      return;
    }
    setStrategyForm({
      ...emptyStrategyForm,
      parentNodeId: parent.id,
      nodeKind: childKind,
    });
    setEditingNodeId(null);
    setFormError(null);
    setStrategyDialogMode("create");
    setExpandedNodeIds((current) => new Set([...current, parent.id]));
  };

  const openEditDialog = (node: StrategyNode) => {
    setStrategyForm({
      parentNodeId: node.parent_node_id ?? "",
      nodeKind: node.node_kind,
      name: node.name,
      description: node.description,
      ownerLabel: node.owner_label,
    });
    setEditingNodeId(node.id);
    setFormError(null);
    setStrategyDialogMode("edit");
  };

  const closeStrategyDialog = () => {
    setStrategyDialogMode("closed");
    setStrategyForm(emptyStrategyForm);
    setEditingNodeId(null);
    setFormError(null);
  };

  const toggleExpanded = (nodeId: string) => {
    setExpandedNodeIds((current) => {
      const next = new Set(current);
      if (next.has(nodeId)) {
        next.delete(nodeId);
      } else {
        next.add(nodeId);
      }
      return next;
    });
  };

  const expandAll = () => setExpandedNodeIds(new Set(strategyNodes.map((node) => node.id)));
  const collapseAll = () => setExpandedNodeIds(new Set());

  return (
    <div style={styles.page}>
      <div style={styles.header}>
        <div>
          <h1 style={styles.title}>Portfolio</h1>
          <div style={styles.subtitle}>CEO/head lens for strategy, product placement, and cross-product dependencies.</div>
        </div>
        <div style={styles.tabBar}>
          <button style={activeTab === "summary" ? styles.tabActive : styles.tab} onClick={() => setActiveTab("summary")}>Summary</button>
          <button style={activeTab === "manage" ? styles.tabActive : styles.tab} onClick={() => setActiveTab("manage")}>Manage</button>
        </div>
      </div>

      <div style={styles.statGrid}>
        <Metric label="Strategic Areas" value={strategyNodes.filter((node) => node.node_kind === "strategic_area").length} />
        <Metric label="Domains" value={strategyNodes.filter((node) => node.node_kind === "domain").length} />
        <Metric label="Products" value={products.length} />
        <Metric label="Unlinked Products" value={unlinkedProducts.length} />
      </div>

      {activeTab === "summary" ? (
        <div style={styles.summaryGrid}>
          <div style={styles.panel}>
            <div style={styles.panelHeader}>
              <div>
                <div style={styles.panelTitle}>Products</div>
                <div style={styles.subtitle}>{selectedStrategyNode ? `${strategyKindLabels[selectedStrategyNode.node_kind]} / ${selectedStrategyNode.name}` : "All products"}</div>
              </div>
              <button style={styles.ghostButton} onClick={() => setSelectedStrategyNodeId(null)}>All Products</button>
            </div>
            <div style={styles.panelBody}>
              {productsLoading ? (
                <div style={styles.empty}>Loading products...</div>
              ) : visibleProducts.length > 0 ? (
                <div style={styles.productGrid}>
                  {visibleProducts.map((product) => (
                    <ProductCard
                      key={product.id}
                      product={product}
                      links={selectedProductLinks(product)}
                      strategyLabel={strategyLabel}
                    />
                  ))}
                </div>
              ) : (
                <div style={styles.empty}>No products linked under this strategy scope yet.</div>
              )}
            </div>
          </div>

          <div style={styles.panel}>
            <div style={styles.panelHeader}>
              <div>
                <div style={styles.panelTitle}>Summary</div>
                <div style={styles.subtitle}>{selectedNodeProductCount} products in selected scope</div>
              </div>
            </div>
            <div style={styles.panelBody}>
              <div style={styles.section}>
                <div style={styles.sectionTitle}>Strategy Map</div>
                {strategyLoading ? (
                  <div style={styles.empty}>Loading strategy...</div>
                ) : strategyTree.length > 0 ? (
                  <CompactStrategyList nodes={strategyTree} selectedId={selectedStrategyNodeId} onSelect={setSelectedStrategyNodeId} strategyLinks={strategyLinks} />
                ) : (
                  <div style={styles.empty}>No strategy areas yet. Use Manage to create the first strategic area.</div>
                )}
              </div>
              <div style={styles.section}>
                <div style={styles.sectionTitle}>Unlinked Products</div>
                {unlinkedProducts.length > 0 ? (
                  <div style={styles.badgeRow}>
                    {unlinkedProducts.map((product) => <span key={product.id} style={styles.badgeMuted}>{product.name}</span>)}
                  </div>
                ) : (
                  <div style={styles.productText}>All products are placed in strategy.</div>
                )}
              </div>
              <div style={styles.section}>
                <div style={styles.sectionTitle}>Dependencies</div>
                {dependencies.length > 0 ? dependencies.map((dependency) => (
                  <DependencyRow key={dependency.id} dependency={dependency} productLabel={productLabel} />
                )) : (
                  <div style={styles.empty}>No cross-product dependencies captured yet.</div>
                )}
              </div>
            </div>
          </div>
        </div>
      ) : (
        <div style={styles.manageGrid}>
          <div style={styles.panel}>
            <div style={styles.panelHeader}>
              <div>
                <div style={styles.panelTitle}>Strategy Hierarchy</div>
                <div style={styles.subtitle}>Accordion map for Strategic Area / Domain / Subdomain.</div>
              </div>
              <div style={styles.controlRow}>
                <button style={styles.ghostButton} onClick={expandAll}>Expand All</button>
                <button style={styles.ghostButton} onClick={collapseAll}>Collapse All</button>
                <button style={styles.button} onClick={openCreateRootDialog}>Add Strategic Area</button>
              </div>
            </div>
            <div style={styles.panelBody}>
              {strategyLoading ? (
                <div style={styles.empty}>Loading strategy...</div>
              ) : strategyTree.length > 0 ? (
                <div style={styles.tree}>
                  {strategyTree.map((node) => (
                    <StrategyNodeAccordion
                      key={node.id}
                      node={node}
                      selectedId={selectedStrategyNodeId}
                      expandedNodeIds={expandedNodeIds}
                      strategyLinks={strategyLinks}
                      onSelect={setSelectedStrategyNodeId}
                      onToggle={toggleExpanded}
                      onAddChild={openCreateChildDialog}
                      onEdit={openEditDialog}
                      onDelete={(candidate) => {
                        setDeleteCandidate(candidate);
                        setDeleteConfirmName("");
                        setDeleteAcknowledge(false);
                        setFormError(null);
                      }}
                    />
                  ))}
                </div>
              ) : (
                <div style={styles.empty}>No strategy hierarchy yet. Add the first strategic area.</div>
              )}
            </div>
          </div>

          <div style={styles.panel}>
            <div style={styles.panelHeader}>
              <div style={styles.panelTitle}>Selected Node</div>
            </div>
            <div style={styles.panelBody}>
              {selectedStrategyNode ? (
                <>
                  <div style={styles.section}>
                    <div style={styles.productTitle}>{selectedStrategyNode.name}</div>
                    <div style={styles.productText}>{strategyKindLabels[selectedStrategyNode.node_kind]}{selectedStrategyNode.owner_label ? ` / ${selectedStrategyNode.owner_label}` : ""}</div>
                    {selectedStrategyNode.description ? <div style={styles.productText}>{selectedStrategyNode.description}</div> : null}
                    <div style={styles.badgeRow}>
                      <span style={styles.badge}>{selectedNodeProductCount} products</span>
                    </div>
                  </div>
                  <div style={styles.controlRow}>
                    {getChildKind(selectedStrategyNode.node_kind) ? <button style={styles.button} onClick={() => openCreateChildDialog(selectedStrategyNode)}>Add Child</button> : null}
                    <button style={styles.ghostButton} onClick={() => openEditDialog(selectedStrategyNode)}>Edit</button>
                    <button style={styles.dangerButton} onClick={() => setDeleteCandidate(selectedStrategyNode)}>Delete</button>
                  </div>
                </>
              ) : (
                <div style={styles.empty}>Select a strategy node to inspect or manage it.</div>
              )}
              {formError && <div style={styles.error}>{formError}</div>}
            </div>
          </div>
        </div>
      )}

      {strategyDialogMode !== "closed" ? (
        <ModalShell title={strategyDialogMode === "create" ? "Add Strategy Node" : "Edit Strategy Node"} onClose={closeStrategyDialog}>
          <StrategyNodeForm
            form={strategyForm}
            nodes={strategyNodes}
            editingNodeId={editingNodeId}
            onChange={setStrategyForm}
          />
          {formError && <div style={styles.error}>{formError}</div>}
          <div style={{ display: "flex", justifyContent: "flex-end", gap: 8 }}>
            <button style={styles.ghostButton} onClick={closeStrategyDialog}>Cancel</button>
            <button
              style={styles.button}
              onClick={() => strategyDialogMode === "create" ? createStrategyMutation.mutate() : updateStrategyMutation.mutate()}
              disabled={!strategyForm.name.trim() || createStrategyMutation.isPending || updateStrategyMutation.isPending}
            >
              {createStrategyMutation.isPending || updateStrategyMutation.isPending ? "Saving..." : "Save Node"}
            </button>
          </div>
        </ModalShell>
      ) : null}

      {deleteCandidate ? (
        <ModalShell title="Delete Strategy Node" onClose={() => setDeleteCandidate(null)}>
          <div style={styles.productText}>
            This deletes "{deleteCandidate.name}" and any child strategy nodes. Product records stay intact, but strategy placements under this branch are removed.
          </div>
          <label style={{ display: "flex", alignItems: "center", gap: 8, marginTop: 12, marginBottom: 10, color: "#374151", fontSize: 12, fontWeight: 700 }}>
            <input type="checkbox" checked={deleteAcknowledge} onChange={(event) => setDeleteAcknowledge(event.target.checked)} />
            I understand this removes the selected strategy branch.
          </label>
          <div style={styles.label}>Type the node name to confirm</div>
          <input style={styles.input} value={deleteConfirmName} onChange={(event) => setDeleteConfirmName(event.target.value)} />
          {formError && <div style={styles.error}>{formError}</div>}
          <div style={{ display: "flex", justifyContent: "flex-end", gap: 8 }}>
            <button style={styles.ghostButton} onClick={() => setDeleteCandidate(null)}>Cancel</button>
            <button
              style={styles.dangerButton}
              onClick={() => deleteStrategyMutation.mutate(deleteCandidate.id)}
              disabled={!deleteAcknowledge || deleteConfirmName !== deleteCandidate.name || deleteStrategyMutation.isPending}
            >
              {deleteStrategyMutation.isPending ? "Deleting..." : "Delete Node"}
            </button>
          </div>
        </ModalShell>
      ) : null}
    </div>
  );
}

function Metric({ label, value }: { label: string; value: number }) {
  return (
    <div style={styles.statCard}>
      <div style={styles.statLabel}>{label}</div>
      <div style={styles.statValue}>{value}</div>
    </div>
  );
}

function ProductCard({ product, links, strategyLabel }: { product: Product; links: ProductStrategyLink[]; strategyLabel: (strategyNodeId: string) => string }) {
  return (
    <div style={styles.productCard}>
      <div style={styles.productTitle}>{product.name}</div>
      <div style={styles.productText}>{product.description || product.vision || "No product description yet."}</div>
      <div style={styles.badgeRow}>
        <span style={styles.badgeGreen}>{product.status}</span>
        <span style={styles.badgeMuted}>{product.lifecycle}</span>
        <span style={styles.badgeMuted}>{product.health}</span>
        {links.map((link) => (
          <span key={link.id} style={styles.badge}>{link.is_primary ? "Primary: " : ""}{strategyLabel(link.strategy_node_id)}</span>
        ))}
      </div>
    </div>
  );
}

function CompactStrategyList(props: {
  nodes: StrategyTreeNode[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  strategyLinks: ProductStrategyLink[];
}) {
  return (
    <div style={styles.tree}>
      {props.nodes.map((node) => (
        <button
          key={node.id}
          style={props.selectedId === node.id ? styles.treeNodeActive : styles.treeNode}
          onClick={() => props.onSelect(node.id)}
        >
          <div style={styles.treeTitle}>{node.name}</div>
          <div style={styles.treeMeta}>{strategyKindLabels[node.node_kind]} / {countProductsForStrategy(node, props.strategyLinks)} products</div>
        </button>
      ))}
    </div>
  );
}

function StrategyNodeAccordion(props: {
  node: StrategyTreeNode;
  selectedId: string | null;
  expandedNodeIds: Set<string>;
  strategyLinks: ProductStrategyLink[];
  onSelect: (id: string) => void;
  onToggle: (id: string) => void;
  onAddChild: (node: StrategyNode) => void;
  onEdit: (node: StrategyNode) => void;
  onDelete: (node: StrategyNode) => void;
}) {
  const { node, selectedId, expandedNodeIds, strategyLinks, onSelect, onToggle, onAddChild, onEdit, onDelete } = props;
  const isExpanded = expandedNodeIds.has(node.id);
  const productCount = countProductsForStrategy(node, strategyLinks);
  const canAddChild = Boolean(getChildKind(node.node_kind));
  return (
    <div>
      <div style={selectedId === node.id ? styles.treeNodeActive : styles.treeNode}>
        <div style={styles.treeHeader}>
          {node.children.length > 0 ? (
            <button style={styles.toggle} aria-label={`${isExpanded ? "Collapse" : "Expand"} ${node.name}`} onClick={() => onToggle(node.id)}>{isExpanded ? "-" : "+"}</button>
          ) : (
            <div style={styles.togglePlaceholder} />
          )}
          <div onClick={() => onSelect(node.id)} style={{ cursor: "pointer", minWidth: 0 }}>
            <div style={styles.treeTitle}>{node.name}</div>
            <div style={styles.treeMeta}>
              {strategyKindLabels[node.node_kind]}{node.owner_label ? ` / ${node.owner_label}` : ""} / {productCount} products
            </div>
          </div>
          <div style={styles.treeActions}>
            {canAddChild ? <button style={styles.ghostButton} aria-label={`Add child to ${node.name}`} onClick={() => onAddChild(node)}>Add</button> : null}
            <button style={styles.ghostButton} aria-label={`Edit ${node.name}`} onClick={() => onEdit(node)}>Edit</button>
            <button style={styles.dangerButton} aria-label={`Delete ${node.name}`} onClick={() => onDelete(node)}>Delete</button>
          </div>
        </div>
      </div>
      {isExpanded && node.children.length > 0 ? (
        <div style={styles.treeChildren}>
          {node.children.map((child) => (
            <StrategyNodeAccordion key={child.id} {...props} node={child} />
          ))}
        </div>
      ) : null}
    </div>
  );
}

function StrategyNodeForm(props: {
  form: StrategyFormState;
  nodes: StrategyNode[];
  editingNodeId: string | null;
  onChange: (form: StrategyFormState) => void;
}) {
  const { form, nodes, editingNodeId, onChange } = props;
  const parentNode = nodes.find((node) => node.id === form.parentNodeId) ?? null;
  const allowedKind = parentNode ? getChildKind(parentNode.node_kind) : "strategic_area";
  const blockedParentIds = new Set(editingNodeId ? [editingNodeId, ...collectDescendantIds(nodes, editingNodeId)] : []);
  const parentOptions = nodes.filter((node) => !blockedParentIds.has(node.id) && Boolean(getChildKind(node.node_kind)));
  const setParent = (parentNodeId: string) => {
    const nextParent = nodes.find((node) => node.id === parentNodeId) ?? null;
    onChange({
      ...form,
      parentNodeId,
      nodeKind: nextParent ? getChildKind(nextParent.node_kind) ?? form.nodeKind : "strategic_area",
    });
  };
  return (
    <>
      <div style={styles.label}>Parent</div>
      <select style={styles.input} value={form.parentNodeId} onChange={(event) => setParent(event.target.value)}>
        <option value="">No parent / Strategic Area</option>
        {parentOptions.map((node) => <option key={node.id} value={node.id}>{strategyKindLabels[node.node_kind]} / {node.name}</option>)}
      </select>
      <div style={styles.label}>Kind</div>
      <select
        style={styles.input}
        value={form.nodeKind}
        onChange={(event) => onChange({ ...form, nodeKind: event.target.value as StrategyNodeKind })}
        disabled
      >
        <option value={allowedKind ?? form.nodeKind}>{strategyKindLabels[allowedKind ?? form.nodeKind]}</option>
      </select>
      <div style={styles.label}>Name</div>
      <input aria-label="Strategy node name" style={styles.input} value={form.name} onChange={(event) => onChange({ ...form, name: event.target.value })} />
      <div style={styles.label}>Owner / Hat</div>
      <input aria-label="Owner or hat" style={styles.input} value={form.ownerLabel} onChange={(event) => onChange({ ...form, ownerLabel: event.target.value })} placeholder="CEO, Head of Devices, Founder" />
      <div style={styles.label}>Description</div>
      <textarea aria-label="Strategy node description" style={styles.textarea} value={form.description} onChange={(event) => onChange({ ...form, description: event.target.value })} />
    </>
  );
}

function DependencyRow({ dependency, productLabel }: { dependency: ProductDependency; productLabel: (productId: string) => string }) {
  return (
    <div style={{ borderTop: "1px solid #d9e0ea", paddingTop: 8, marginTop: 8 }}>
      <div style={styles.productTitle}>{productLabel(dependency.product_id)}</div>
      <div style={styles.productText}>depends on {productLabel(dependency.depends_on_product_id)}</div>
      <div style={styles.badgeRow}>
        <span style={styles.badge}>{dependency.dependency_kind}</span>
        <span style={styles.badgeGreen}>{dependency.status}</span>
      </div>
      {dependency.description ? <div style={styles.productText}>{dependency.description}</div> : null}
    </div>
  );
}

function ModalShell({ title, onClose, children }: { title: string; onClose: () => void; children: React.ReactNode }) {
  return (
    <div style={styles.modalBackdrop} onClick={onClose}>
      <div style={styles.modal} onClick={(event) => event.stopPropagation()}>
        <div style={styles.modalHeader}>
          <div style={styles.modalTitle}>{title}</div>
          <button style={styles.ghostButton} onClick={onClose}>Close</button>
        </div>
        <div style={styles.modalBody}>{children}</div>
      </div>
    </div>
  );
}

function buildStrategyTree(nodes: StrategyNode[]): StrategyTreeNode[] {
  const byId = new Map<string, StrategyTreeNode>();
  nodes.forEach((node) => byId.set(node.id, { ...node, children: [] }));
  const roots: StrategyTreeNode[] = [];
  byId.forEach((node) => {
    if (node.parent_node_id && byId.has(node.parent_node_id)) {
      byId.get(node.parent_node_id)!.children.push(node);
      return;
    }
    roots.push(node);
  });
  const sortNodes = (items: StrategyTreeNode[]) => {
    items.sort((left, right) => left.sort_order - right.sort_order || left.name.localeCompare(right.name));
    items.forEach((item) => sortNodes(item.children));
  };
  sortNodes(roots);
  return roots;
}

function findTreeNode(nodes: StrategyTreeNode[], id: string): StrategyTreeNode | null {
  for (const node of nodes) {
    if (node.id === id) {
      return node;
    }
    const found = findTreeNode(node.children, id);
    if (found) {
      return found;
    }
  }
  return null;
}

function collectStrategySubtreeIds(nodes: StrategyTreeNode[], targetId: string): string[] {
  const target = findTreeNode(nodes, targetId);
  return target ? collectIds(target) : [];
}

function collectIds(node: StrategyTreeNode): string[] {
  return [node.id, ...node.children.flatMap(collectIds)];
}

function collectDescendantIds(nodes: StrategyNode[], nodeId: string): string[] {
  const children = nodes.filter((node) => node.parent_node_id === nodeId);
  return children.flatMap((child) => [child.id, ...collectDescendantIds(nodes, child.id)]);
}

function countProductsForStrategy(node: StrategyTreeNode | null, links: ProductStrategyLink[]): number {
  if (!node) {
    return 0;
  }
  const ids = new Set(collectIds(node));
  return links.filter((link) => ids.has(link.strategy_node_id)).length;
}

function getChildKind(kind: StrategyNodeKind): StrategyNodeKind | null {
  switch (kind) {
    case "strategic_area":
      return "domain";
    case "domain":
      return "subdomain";
    case "subdomain":
      return null;
  }
}

function formatError(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}
