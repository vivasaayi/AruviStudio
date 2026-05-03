import React, { useEffect } from "react";
import { useQuery } from "@tanstack/react-query";
import { useNavigate } from "react-router-dom";
import {
  exportProductOverviewEpub,
  exportProductOverviewHtml,
  exportProductOverviewPdf,
  getProductTree,
  listProducts,
  listWorkItems,
  revealInFinder,
} from "../../../lib/tauri";
import { useWorkspaceStore } from "../../../state/workspaceStore";
import { useUIStore } from "../../../state/uiStore";
import { ProductOverviewDocument } from "../components/ProductOverviewDocument";
import type { Capability, Module, Product, ProductTree, WorkItem } from "../../../lib/types";
import {
  BOOK_EXPORT_TRIM_PRESETS,
  buildProductOverviewBookBundle,
  getBookExportTrimPreset,
  type BookExportTrimPresetId,
} from "../lib/bookExport";
import { buildProductOverviewHtml } from "../lib/productOverview";

const styles: Record<string, React.CSSProperties> = {
  page: { display: "flex", flexDirection: "column", gap: 18, minHeight: "100%", maxWidth: 1440, margin: "0 auto", width: "100%" },
  header: { display: "flex", justifyContent: "space-between", alignItems: "flex-start", gap: 18, flexWrap: "wrap" as const },
  titleBlock: { display: "flex", flexDirection: "column", gap: 6, minWidth: 0 },
  title: { fontSize: 28, fontWeight: 900, color: "#f3f6fb", margin: 0, lineHeight: 1.05 },
  subtitle: { fontSize: 13, color: "#9aa7bb", lineHeight: 1.65, maxWidth: 760, margin: 0 },
  controlCard: { minWidth: 340, maxWidth: 420, width: "100%", border: "1px solid #2c3644", borderRadius: 18, backgroundColor: "#141b24", padding: 16, boxShadow: "0 18px 40px rgba(0,0,0,0.18)" },
  controlLabel: { fontSize: 11, fontWeight: 800, color: "#8f96a3", letterSpacing: "0.08em", textTransform: "uppercase" as const, marginBottom: 8 },
  select: { width: "100%", padding: "11px 12px", backgroundColor: "#0f151d", border: "1px solid #303b4a", borderRadius: 10, color: "#f0f4fb", fontSize: 13, boxSizing: "border-box" as const },
  helper: { marginTop: 10, fontSize: 12, color: "#98a7bc", lineHeight: 1.55 },
  actionRow: { display: "flex", gap: 10, marginTop: 14, flexWrap: "wrap" as const },
  primaryBtn: { padding: "8px 12px", fontSize: 12, fontWeight: 700, backgroundColor: "#0e639c", color: "#fff", border: "none", borderRadius: 8, cursor: "pointer" },
  ghostBtn: { padding: "8px 12px", fontSize: 12, fontWeight: 700, backgroundColor: "#1a2430", color: "#d8e2f0", border: "1px solid #314255", borderRadius: 8, cursor: "pointer" },
  successText: { marginTop: 10, fontSize: 12, color: "#4ec9b0", lineHeight: 1.55, wordBreak: "break-all" as const },
  errorText: { marginTop: 10, fontSize: 12, color: "#ff8e8e", lineHeight: 1.55 },
  empty: { border: "1px solid #2c3644", borderRadius: 18, backgroundColor: "#141b24", padding: 24, color: "#9aa7bb", fontSize: 14 },
};

export function ProductOverviewPage() {
  const navigate = useNavigate();
  const { activeProductId, setActiveProduct, setActiveModule, setActiveCapability, setActiveWorkItem } = useWorkspaceStore();
  const {
    setActiveView,
    setProductWorkspaceTab,
    setWorkItemWorkspaceTab,
    openProductDialog,
    openModuleDialog,
    openCapabilityDialog,
  } = useUIStore();
  const [exportPath, setExportPath] = React.useState<string | null>(null);
  const [exportError, setExportError] = React.useState<string | null>(null);
  const [isExporting, setIsExporting] = React.useState(false);
  const [bookTrimPresetId, setBookTrimPresetId] = React.useState<BookExportTrimPresetId>(BOOK_EXPORT_TRIM_PRESETS[0].id);

  const { data: products = [], isLoading: productsLoading } = useQuery<Product[]>({
    queryKey: ["products"],
    queryFn: listProducts,
  });

  const visibleActiveProductId = products.some((product) => product.id === activeProductId)
    ? activeProductId
    : null;
  const selectedProductId = visibleActiveProductId ?? products[0]?.id ?? null;
  const selectedProduct = products.find((product) => product.id === selectedProductId) ?? null;

  useEffect(() => {
    if (productsLoading) {
      return;
    }
    if (activeProductId !== selectedProductId) {
      setActiveProduct(selectedProductId);
    }
  }, [activeProductId, productsLoading, selectedProductId, setActiveProduct]);

  const { data: tree, isLoading: treeLoading } = useQuery({
    queryKey: ["productOverviewTree", selectedProductId],
    queryFn: () => getProductTree(selectedProductId!),
    enabled: !!selectedProduct,
  });

  const { data: workItems = [], isLoading: workItemsLoading } = useQuery<WorkItem[]>({
    queryKey: ["productOverviewPageWorkItems", selectedProductId],
    queryFn: () => listWorkItems({ productId: selectedProductId ?? undefined }),
    enabled: !!selectedProduct,
  });

  useEffect(() => {
    if (treeLoading || workItemsLoading) {
      return;
    }
    if (typeof window === "undefined") {
      return;
    }
    const hash = window.location.hash.replace(/^#/, "");
    if (!hash) {
      return;
    }
    requestAnimationFrame(() => {
      document.getElementById(hash)?.scrollIntoView({ block: "start" });
    });
  }, [selectedProductId, treeLoading, workItemsLoading, tree, workItems.length]);

  const goToProductWorkspace = () => {
    setActiveView("products");
    navigate("/products");
  };

  const exportHtml = async () => {
    await runExport("overview", buildProductOverviewHtml);
  };

  const runExport = async (
    variant: "overview" | "book",
    builder: (input: { product: Product; tree?: ProductTree; workItems?: WorkItem[] }) => string,
  ) => {
    if (!selectedProduct) {
      return;
    }

    try {
      setIsExporting(true);
      setExportError(null);
      const html = builder({
        product: selectedProduct,
        tree,
        workItems,
      });
      const path = await exportProductOverviewHtml({
        fileName: `${slugify(selectedProduct.name)}-${variant}.html`,
        html,
      });
      setExportPath(path);
    } catch (error) {
      setExportPath(null);
      setExportError(String(error));
    } finally {
      setIsExporting(false);
    }
  };

  const runBookArtifactExport = async (format: "html" | "pdf" | "epub") => {
    if (!selectedProduct) {
      return;
    }

    const trimPreset = getBookExportTrimPreset(bookTrimPresetId);

    try {
      setIsExporting(true);
      setExportError(null);
      const bundle = buildProductOverviewBookBundle(
        {
          product: selectedProduct,
          tree,
          workItems,
        },
        {
          trimPreset,
          renderMode: format === "html" ? "web" : format === "pdf" ? "print" : "epub",
        },
      );

      let path: string;
      if (format === "html") {
        path = await exportProductOverviewHtml({
          fileName: `${slugify(selectedProduct.name)}-book.html`,
          html: bundle.html,
        });
      } else if (format === "pdf") {
        path = await exportProductOverviewPdf({
          fileName: `${slugify(selectedProduct.name)}-book.pdf`,
          html: bundle.html,
          pageWidth: trimPreset.pageWidth,
          pageHeight: trimPreset.pageHeight,
          marginTop: trimPreset.marginTop,
          marginRight: trimPreset.marginRight,
          marginBottom: trimPreset.marginBottom,
          marginLeft: trimPreset.marginLeft,
          headerTitle: selectedProduct.name,
          headerRight: trimPreset.label,
        });
      } else {
        path = await exportProductOverviewEpub({
          fileName: `${slugify(selectedProduct.name)}-book.epub`,
          title: selectedProduct.name,
          html: bundle.html,
          tocItems: bundle.tocItems,
          author: "Aruvi Studio",
          language: "en",
        });
      }

      setExportPath(path);
    } catch (error) {
      setExportPath(null);
      setExportError(String(error));
    } finally {
      setIsExporting(false);
    }
  };

  const editProduct = () => {
    if (!selectedProductId) {
      return;
    }
    setActiveProduct(selectedProductId);
    setActiveView("products");
    navigate(`/products/${selectedProductId}`);
    openProductDialog("edit");
  };

  const editModule = (module: Module) => {
    if (!selectedProductId) {
      return;
    }
    setActiveProduct(selectedProductId);
    setActiveModule(module.id);
    setActiveCapability(null);
    setProductWorkspaceTab("structure");
    setActiveView("products");
    navigate(`/products/${selectedProductId}`);
    openModuleDialog("edit");
  };

  const editCapability = (capability: Capability) => {
    if (!selectedProductId) {
      return;
    }
    setActiveProduct(selectedProductId);
    setActiveModule(capability.module_id);
    setActiveCapability(capability.id);
    setProductWorkspaceTab("structure");
    setActiveView("products");
    navigate(`/products/${selectedProductId}`);
    openCapabilityDialog("edit");
  };

  const openWorkItem = (workItem: WorkItem) => {
    setActiveProduct(workItem.product_id);
    setActiveModule(workItem.module_id ?? null);
    setActiveCapability(workItem.capability_id ?? null);
    setActiveWorkItem(workItem.id);
    setWorkItemWorkspaceTab("detail");
    setActiveView("work-items");
    navigate(`/work-items/${workItem.id}`);
  };

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
          <h1 style={styles.title}>Product Overview</h1>
          <p style={styles.subtitle}>
            Read the product like documentation instead of a CRUD screen. This page is optimized for review, correction, and export.
          </p>
        </div>
        <div style={styles.controlCard}>
          <div style={styles.controlLabel}>Product</div>
          <select
            style={styles.select}
            value={selectedProductId ?? ""}
            onChange={(event) => setActiveProduct(event.target.value || null)}
            disabled={products.length === 0}
          >
            {products.map((product) => (
              <option key={product.id} value={product.id}>
                {product.name}
              </option>
            ))}
          </select>
          <div style={styles.helper}>
            This route is product-wide. Export writes HTML, EPUB, and print-ready PDF files to `~/Documents/AruviStudio/exports/`.
          </div>
          <div style={{ ...styles.controlLabel, marginTop: 14 }}>Book Trim Preset</div>
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
          <div style={styles.helper}>
            {getBookExportTrimPreset(bookTrimPresetId).description}
          </div>
          <div style={styles.actionRow}>
            <button
              style={styles.primaryBtn}
              onClick={exportHtml}
              disabled={!selectedProduct || treeLoading || workItemsLoading || isExporting}
            >
              {isExporting ? "Exporting…" : "Export Docs HTML"}
            </button>
            <button
              style={styles.ghostBtn}
              onClick={() => runBookArtifactExport("html")}
              disabled={!selectedProduct || treeLoading || workItemsLoading || isExporting}
            >
              Export Book HTML
            </button>
            <button
              style={styles.ghostBtn}
              onClick={() => runBookArtifactExport("epub")}
              disabled={!selectedProduct || treeLoading || workItemsLoading || isExporting}
            >
              Export EPUB
            </button>
            <button
              style={styles.ghostBtn}
              onClick={() => runBookArtifactExport("pdf")}
              disabled={!selectedProduct || treeLoading || workItemsLoading || isExporting}
            >
              Export Book PDF
            </button>
            {exportPath ? (
              <button style={styles.ghostBtn} onClick={() => revealInFinder(exportPath)}>
                Reveal Export
              </button>
            ) : (
              <button style={styles.ghostBtn} onClick={goToProductWorkspace}>
                Open Products
              </button>
            )}
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
          isLoading={treeLoading || workItemsLoading}
          onEditProduct={editProduct}
          onEditModule={editModule}
          onEditCapability={editCapability}
          onOpenWorkItem={openWorkItem}
        />
      ) : (
        <div style={styles.empty}>Loading product overview…</div>
      )}
    </div>
  );
}

function slugify(value: string) {
  return value
    .toLowerCase()
    .trim()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    || "product";
}
