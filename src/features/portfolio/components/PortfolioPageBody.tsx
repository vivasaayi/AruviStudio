import {
  CompactStrategyList,
  DependencyRow,
  Metric,
  ModalShell,
  ProductCard,
  StrategyNodeAccordion,
  StrategyNodeForm,
} from "./PortfolioPageComponents";
import type { PortfolioPageController } from "../hooks/usePortfolioPageController";
import { getChildKind, strategyKindLabels } from "../lib/portfolioStrategyTree";
import { styles } from "../lib/portfolioPageStyles";

type PortfolioPageBodyProps = {
  controller: PortfolioPageController;
};

export function PortfolioPageBody({ controller }: PortfolioPageBodyProps) {
  const {
    activeTab,
    collapseAll,
    createStrategyMutation,
    deleteAcknowledge,
    deleteCandidate,
    deleteConfirmName,
    deleteStrategyMutation,
    dependencies,
    editingNodeId,
    expandAll,
    expandedNodeIds,
    formError,
    linkDraft,
    linkProductMutation,
    linkableProducts,
    openCreateChildDialog,
    openCreateRootDialog,
    openEditDialog,
    productLabel,
    products,
    productsLoading,
    selectedNodeLinks,
    selectedNodeProductCount,
    selectedProductLinks,
    selectedStrategyNode,
    selectedStrategyNodeId,
    setActiveTab,
    setDeleteAcknowledge,
    setDeleteCandidate,
    setDeleteConfirmName,
    setFormError,
    setLinkDraft,
    setSelectedStrategyNodeId,
    setStrategyForm,
    strategyDialogMode,
    strategyForm,
    strategyLabel,
    strategyLinks,
    strategyLoading,
    strategyNodes,
    strategyTree,
    toggleExpanded,
    unlinkProductMutation,
    unlinkedProducts,
    updateStrategyMutation,
    visibleProducts,
    closeStrategyDialog,
  } = controller;

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
