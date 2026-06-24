import React from "react";
import { useNavigate } from "react-router-dom";
import { revealInFinder } from "../../../lib/tauri";
import { useWorkspaceStore } from "../../../state/workspaceStore";
import { useUIStore } from "../../../state/uiStore";
import { ProductOverviewDocument } from "../components/ProductOverviewDocument";
import { BOOK_EXPORT_TRIM_PRESETS, type BookExportTrimPresetId } from "../lib/bookExport";
import { useProductOverviewPageController } from "../hooks/useProductOverviewPageController";

const styles: Record<string, React.CSSProperties> = {
  page: { display: "flex", flexDirection: "column", gap: 8, minHeight: "100%", width: "100%" },
  header: { display: "flex", flexDirection: "column", gap: 6 },
  titleBlock: { display: "flex", flexDirection: "column", gap: 3, minWidth: 0 },
  title: { fontSize: 18, fontWeight: 900, color: "#111827", margin: 0, lineHeight: 1.05 },
  subtitle: { fontSize: 12, color: "#64748b", lineHeight: 1.45, margin: 0 },
  controlCard: { display: "grid", gridTemplateColumns: "minmax(220px, 320px) minmax(220px, 300px) minmax(0, 1fr)", gap: 8, alignItems: "end", border: "1px solid #d8dee8", borderRadius: 8, backgroundColor: "#ffffff", padding: 8 },
  controlLabel: { fontSize: 10, fontWeight: 800, color: "#64748b", letterSpacing: "0.08em", textTransform: "uppercase" as const, marginBottom: 4 },
  select: { width: "100%", padding: "6px 9px", backgroundColor: "#ffffff", border: "1px solid #cbd5e1", borderRadius: 8, color: "#111827", fontSize: 12, boxSizing: "border-box" as const },
  helper: { fontSize: 11, color: "#64748b", lineHeight: 1.4 },
  actionRow: { display: "flex", gap: 6, flexWrap: "wrap" as const, justifyContent: "flex-end" as const },
  primaryBtn: { padding: "6px 10px", fontSize: 12, fontWeight: 700, backgroundColor: "#2563eb", color: "#ffffff", border: "1px solid #1d4ed8", borderRadius: 8, cursor: "pointer" },
  ghostBtn: { padding: "6px 10px", fontSize: 12, fontWeight: 700, backgroundColor: "#f8fafc", color: "#1e3a8a", border: "1px solid #93c5fd", borderRadius: 8, cursor: "pointer" },
  successText: { fontSize: 11, color: "#15803d", lineHeight: 1.45, wordBreak: "break-all" as const, gridColumn: "1 / -1" },
  errorText: { fontSize: 11, color: "#b91c1c", lineHeight: 1.45, gridColumn: "1 / -1" },
  empty: { border: "1px solid #d8dee8", borderRadius: 14, backgroundColor: "#ffffff", padding: 20, color: "#64748b", fontSize: 14 },
};

export function ProductOverviewPage() {
  const navigate = useNavigate();
  const { activeProductId, setActiveProduct, setActiveProductArea, setActiveCapability, setActiveWorkItem } = useWorkspaceStore();
  const {
    setActiveView,
    setProductWorkspaceTab,
    setWorkItemWorkspaceTab,
    openProductDialog,
    openProductAreaDialog,
    openCapabilityDialog,
  } = useUIStore();
  const {
    bookTrimPresetId,
    editCapability,
    editProduct,
    editProductArea,
    exportError,
    exportHtml,
    exportPath,
    goToProductWorkspace,
    isExporting,
    loadProductAreaTree,
    openWorkItem,
    overviewMetrics,
    planFromItem,
    productReferences,
    products,
    productsLoading,
    referencesLoading,
    runBookArtifactExport,
    selectedProduct,
    selectedProductId,
    selectedProductWorkItemSummary,
    selectProduct,
    setBookTrimPresetId,
    summariesLoading,
    tree,
    treeLoading,
    treeSummary,
    workItems,
  } = useProductOverviewPageController({
    activeProductId,
    setActiveProduct,
    setActiveProductArea,
    setActiveCapability,
    setActiveWorkItem,
    setActiveView,
    setProductWorkspaceTab,
    setWorkItemWorkspaceTab,
    openProductDialog,
    openProductAreaDialog,
    openCapabilityDialog,
    navigate,
  });

  if (!productsLoading && products.length === 0) {
    return (
      <div style={styles.page}>
        <div style={styles.empty}>
          No visible products yet. Create one in the product workspace first.
          <div>
            <button style={styles.primaryBtn} onClick={goToProductWorkspace}>Open Products</button>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div style={styles.page}>
      <div style={styles.header}>
        <div style={styles.titleBlock}>
          <h1 style={styles.title}>Product Book</h1>
          <p style={styles.subtitle}>
            Read, revise, and export the product book before sending its stories and tasks to delivery.
          </p>
        </div>
        <div style={styles.controlCard}>
          <div>
            <div style={styles.controlLabel}>Product</div>
            <select
              style={styles.select}
              value={selectedProductId ?? ""}
              onChange={(event) => selectProduct(event.target.value || null)}
              disabled={products.length === 0}
            >
              {products.map((product) => (
                <option key={product.id} value={product.id}>
                  {product.name}
                </option>
              ))}
            </select>
          </div>
          <div>
            <div style={styles.controlLabel}>Book Trim Preset</div>
            <select
              style={styles.select}
              value={bookTrimPresetId}
              onChange={(event) => setBookTrimPresetId(event.target.value as BookExportTrimPresetId)}
              disabled={isExporting}
            >
              {BOOK_EXPORT_TRIM_PRESETS.map((preset) => (
                <option key={preset.id} value={preset.id}>
                  {preset.label}
                </option>
              ))}
            </select>
          </div>
          <div>
            <div style={styles.actionRow}>
              <button
                style={styles.primaryBtn}
                onClick={exportHtml}
                disabled={!selectedProduct || treeLoading || referencesLoading || isExporting}
              >
                {isExporting ? "Exporting..." : "Docs HTML"}
              </button>
              <button
                style={styles.ghostBtn}
                onClick={() => runBookArtifactExport("html")}
                disabled={!selectedProduct || treeLoading || referencesLoading || isExporting}
              >
                Book HTML
              </button>
              <button
                style={styles.ghostBtn}
                onClick={() => runBookArtifactExport("epub")}
                disabled={!selectedProduct || treeLoading || referencesLoading || isExporting}
              >
                EPUB
              </button>
              <button
                style={styles.ghostBtn}
                onClick={() => runBookArtifactExport("pdf")}
                disabled={!selectedProduct || treeLoading || referencesLoading || isExporting}
              >
                PDF
              </button>
              {exportPath ? (
                <button style={styles.ghostBtn} onClick={() => revealInFinder(exportPath)}>
                  Reveal
                </button>
              ) : (
                <button style={styles.ghostBtn} onClick={goToProductWorkspace}>
                  Products
                </button>
              )}
            </div>
          </div>
          {exportPath ? <div style={styles.successText}>Exported to {exportPath}</div> : null}
          {exportError ? <div style={styles.errorText}>Export failed: {exportError}</div> : null}
        </div>
      </div>

      {selectedProduct ? (
        <ProductOverviewDocument
          product={selectedProduct}
          tree={tree}
          workItems={workItems}
          metricsOverride={overviewMetrics}
          nodeCountsOverride={treeSummary ? {
            productAreaCount: treeSummary.product_area_count,
            totalNodeCount: treeSummary.total_node_count,
            leafNodeCount: treeSummary.leaf_node_count,
          } : undefined}
          activeWorkItemCountOverride={selectedProductWorkItemSummary?.active_count ?? 0}
          references={productReferences}
          isLoading={treeLoading || summariesLoading || referencesLoading}
          onEditProduct={editProduct}
          onEditProductArea={editProductArea}
          onEditCapability={editCapability}
          onLoadProductAreaTree={loadProductAreaTree}
          onOpenWorkItem={openWorkItem}
          onPlanFromItem={planFromItem}
        />
      ) : (
        <div style={styles.empty}>Loading product overview…</div>
      )}
    </div>
  );
}
