import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  createStrategyNode,
  deleteStrategyNode,
  linkProductToStrategy,
  listProductDependencies,
  listProducts,
  listProductStrategyLinks,
  listStrategyNodes,
  unlinkProductFromStrategy,
  updateStrategyNode,
} from "../../../lib/tauri";
import type { Product, StrategyNode } from "../../../lib/types";
import {
  CompactStrategyList,
  DependencyRow,
  Metric,
  ModalShell,
  ProductCard,
  StrategyNodeAccordion,
  StrategyNodeForm,
} from "../components/PortfolioPageComponents";
import {
  buildStrategyTree,
  collectStrategySubtreeIds,
  countProductsForStrategy,
  findTreeNode,
  formatPortfolioError,
  getChildKind,
  strategyKindLabels,
} from "../lib/portfolioStrategyTree";
import { emptyStrategyForm, type PortfolioTab, type StrategyDialogMode, type StrategyFormState } from "../lib/portfolioPageState";
import { styles } from "../lib/portfolioPageStyles";

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
  const [linkDraft, setLinkDraft] = useState({ productId: "", isPrimary: true });
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
  const selectedNodeLinks = selectedStrategyNode
    ? strategyLinks.filter((link) => link.strategy_node_id === selectedStrategyNode.id)
    : [];
  const selectedNodeLinkedProductIds = new Set(selectedNodeLinks.map((link) => link.product_id));
  const linkableProducts = products.filter((product) => !selectedNodeLinkedProductIds.has(product.id));

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
    onError: (error) => setFormError(formatPortfolioError(error)),
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
    onError: (error) => setFormError(formatPortfolioError(error)),
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
    onError: (error) => setFormError(formatPortfolioError(error)),
  });

  const linkProductMutation = useMutation({
    mutationFn: () => linkProductToStrategy({
      productId: linkDraft.productId,
      strategyNodeId: selectedStrategyNodeId!,
      isPrimary: linkDraft.isPrimary,
    }),
    onSuccess: async () => {
      setLinkDraft({ productId: "", isPrimary: true });
      setFormError(null);
      await invalidateStrategy();
    },
    onError: (error) => setFormError(formatPortfolioError(error)),
  });

  const unlinkProductMutation = useMutation({
    mutationFn: (productId: string) => unlinkProductFromStrategy({
      productId,
      strategyNodeId: selectedStrategyNodeId!,
    }),
    onSuccess: async () => {
      setFormError(null);
      await invalidateStrategy();
    },
    onError: (error) => setFormError(formatPortfolioError(error)),
  });

  const productLabel = (productId: string) => products.find((product) => product.id === productId)?.name ?? "Unknown product";
  const strategyLabel = (strategyNodeId: string) => strategyNodeById.get(strategyNodeId)?.name ?? "Unknown strategy";
  const selectedProductLinks = (product: Product) => strategyLinks.filter((link) => link.product_id === product.id);

  const openCreateRootDialog = () => {
    setStrategyForm({ ...emptyStrategyForm, nodeKind: "strategic_product_area" });
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
        <Metric label="Strategic Product Areas" value={strategyNodes.filter((node) => node.node_kind === "strategic_product_area").length} />
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
                  <div style={styles.empty}>No strategic product areas yet. Use Manage to create the first strategic product area.</div>
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
                <div style={styles.subtitle}>Accordion map for Strategic Product Area / Domain / Sub Domain.</div>
              </div>
              <div style={styles.controlRow}>
                <button style={styles.ghostButton} onClick={expandAll}>Expand All</button>
                <button style={styles.ghostButton} onClick={collapseAll}>Collapse All</button>
                <button style={styles.button} onClick={openCreateRootDialog}>Add Strategic Product Area</button>
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
                <div style={styles.empty}>No strategy hierarchy yet. Add the first strategic product area.</div>
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
                  <div style={styles.section}>
                    <div style={styles.sectionTitle}>Product Placement</div>
                    <div style={styles.label}>Link product to this node</div>
                    <select
                      aria-label="Product to link"
                      style={styles.input}
                      value={linkDraft.productId}
                      onChange={(event) => setLinkDraft((draft) => ({ ...draft, productId: event.target.value }))}
                    >
                      <option value="">Select product</option>
                      {linkableProducts.map((product) => <option key={product.id} value={product.id}>{product.name}</option>)}
                    </select>
                    <label style={{ display: "flex", alignItems: "center", gap: 8, color: "#374151", fontSize: 12, fontWeight: 700, marginBottom: 10 }}>
                      <input
                        type="checkbox"
                        checked={linkDraft.isPrimary}
                        onChange={(event) => setLinkDraft((draft) => ({ ...draft, isPrimary: event.target.checked }))}
                      />
                      Primary placement
                    </label>
                    <button
                      style={styles.button}
                      onClick={() => linkProductMutation.mutate()}
                      disabled={!selectedStrategyNodeId || !linkDraft.productId || linkProductMutation.isPending}
                    >
                      {linkProductMutation.isPending ? "Linking..." : "Link Product"}
                    </button>
                  </div>
                  <div style={styles.section}>
                    <div style={styles.sectionTitle}>Linked Products</div>
                    {selectedNodeLinks.length > 0 ? (
                      selectedNodeLinks.map((link) => (
                        <div key={link.id} style={{ borderTop: "1px solid #d9e0ea", paddingTop: 8, marginTop: 8 }}>
                          <div style={styles.productTitle}>{productLabel(link.product_id)}</div>
                          <div style={styles.badgeRow}>
                            {link.is_primary ? <span style={styles.badge}>Primary</span> : <span style={styles.badgeMuted}>Secondary</span>}
                            <button
                              style={styles.dangerButton}
                              onClick={() => unlinkProductMutation.mutate(link.product_id)}
                              disabled={unlinkProductMutation.isPending}
                            >
                              Unlink
                            </button>
                          </div>
                        </div>
                      ))
                    ) : (
                      <div style={styles.empty}>No products are linked directly to this strategy node.</div>
                    )}
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
