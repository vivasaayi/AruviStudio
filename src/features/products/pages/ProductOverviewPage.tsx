import React, { useEffect } from "react";
import { useQuery } from "@tanstack/react-query";
import { useNavigate } from "react-router-dom";
import {
  exportProductOverviewEpub,
  exportProductOverviewHtml,
  exportProductOverviewPdf,
  getProductTree,
  listProductReferences,
  listProducts,
  revealInFinder,
  summarizeWorkItemsByProduct,
} from "../../../lib/tauri";
import { useWorkspaceStore } from "../../../state/workspaceStore";
import { useUIStore } from "../../../state/uiStore";
import { ProductOverviewDocument, type ProductOverviewPlannerAction } from "../components/ProductOverviewDocument";
import type { Capability, ProductArea, Product, ProductReference, ProductTree, ProductWorkItemSummary, WorkItem } from "../../../lib/types";
import {
  BOOK_EXPORT_TRIM_PRESETS,
  buildProductOverviewBookBundle,
  getBookExportTrimPreset,
  type BookExportTrimPresetId,
} from "../lib/bookExport";
import { buildProductOverviewHtml, type WorkItemMetrics } from "../lib/productOverview";

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

  const { data: productWorkItemSummaries = [], isLoading: summariesLoading } = useQuery<ProductWorkItemSummary[]>({
    queryKey: ["productWorkItemSummary"],
    queryFn: summarizeWorkItemsByProduct,
    enabled: !!selectedProduct,
  });
  const selectedProductWorkItemSummary = React.useMemo(
    () => productWorkItemSummaries.find((summary) => summary.product_id === selectedProductId) ?? null,
    [productWorkItemSummaries, selectedProductId],
  );
  const overviewMetrics = React.useMemo<WorkItemMetrics>(() => {
    const total = selectedProductWorkItemSummary?.total_count ?? 0;
    const done = selectedProductWorkItemSummary?.done_count ?? 0;
    const blocked = selectedProductWorkItemSummary?.blocked_count ?? 0;
    const active = selectedProductWorkItemSummary?.active_count ?? 0;
    const wip = Math.max(0, active - blocked);
    const tbd = Math.max(0, total - done - active);
    return {
      total,
      done,
      wip,
      tbd,
      blocked,
      completion: total > 0 ? Math.round((done / total) * 100) : 0,
    };
  }, [selectedProductWorkItemSummary]);
  const workItems = React.useMemo<WorkItem[]>(() => [], []);

  const { data: productReferences = [], isLoading: referencesLoading } = useQuery<ProductReference[]>({
    queryKey: ["productOverviewPageReferences"],
    queryFn: () => listProductReferences(),
    enabled: !!selectedProduct,
  });

  useEffect(() => {
    if (treeLoading || summariesLoading) {
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
  }, [selectedProductId, treeLoading, summariesLoading, tree, overviewMetrics.total]);

  const goToProductWorkspace = () => {
    setActiveView("products");
    navigate("/products");
  };

  const exportHtml = async () => {
    await runExport("overview", buildProductOverviewHtml);
  };

  const runExport = async (
    variant: "overview" | "book",
    builder: (input: { product: Product; tree?: ProductTree; workItems?: WorkItem[]; references?: ProductReference[] }) => string,
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
        references: productReferences,
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
          references: productReferences,
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

  const editProductArea = (product_area: ProductArea) => {
    if (!selectedProductId) {
      return;
    }
    setActiveProduct(selectedProductId);
    setActiveProductArea(product_area.id);
    setActiveCapability(null);
    setProductWorkspaceTab("structure");
    setActiveView("products");
    navigate(`/products/${selectedProductId}`);
    openProductAreaDialog("edit");
  };

  const editCapability = (capability: Capability) => {
    if (!selectedProductId) {
      return;
    }
    setActiveProduct(selectedProductId);
    setActiveProductArea(capability.product_area_id);
    setActiveCapability(capability.id);
    setProductWorkspaceTab("structure");
    setActiveView("products");
    navigate(`/products/${selectedProductId}`);
    openCapabilityDialog("edit");
  };

  const openWorkItem = (workItem: WorkItem) => {
    setActiveProduct(workItem.product_id);
    setActiveProductArea(workItem.product_area_id ?? null);
    setActiveCapability(workItem.capability_id ?? null);
    setActiveWorkItem(workItem.id);
    setWorkItemWorkspaceTab("detail");
    setActiveView("work-items");
    navigate(`/work-items/${workItem.id}`);
  };

  const planFromItem = (action: ProductOverviewPlannerAction) => {
    const product = action.product;
    let prompt: string;

    setActiveProduct(product.id);
    setActiveWorkItem(null);

    switch (action.kind) {
      case "enhance_product":
        prompt = `Enhance product "${product.name}". Review the current product management tree, identify missing product areas, capabilities, features, stories, and tasks, then stage a concrete improvement plan.`;
        break;
      case "add_product_child":
        prompt = `Add a useful product area under product "${product.name}". Stage the new product area with its initial capabilities and starter stories.`;
        break;
      case "enhance_product_area":
        setActiveProductArea(action.product_area.id);
        prompt = `Enhance product area "${action.product_area.name}" in product "${product.name}". Add or revise child capabilities, features, stories, and tasks so this branch is execution-ready.`;
        break;
      case "add_product_area_child":
        setActiveProductArea(action.product_area.id);
        prompt = `Add child capabilities under product area "${action.product_area.name}" in product "${product.name}". Include concise descriptions, acceptance criteria, and starter stories where helpful.`;
        break;
      case "enhance_capability":
        setActiveProductArea(action.capability.product_area_id);
        setActiveCapability(action.capability.id);
        prompt = `Enhance ${action.capability.node_kind.replace(/_/g, " ")} "${action.capability.name}" under "${action.productAreaName}" in product "${product.name}". Improve its description, acceptance criteria, technical notes, and missing child structure.`;
        break;
      case "add_capability_child":
        setActiveProductArea(action.capability.product_area_id);
        setActiveCapability(action.capability.id);
        prompt = `Add child nodes under "${action.capability.name}" in product "${product.name}". Stage concrete features with clear descriptions and acceptance criteria.`;
        break;
      case "add_capability_work_item":
        setActiveProductArea(action.capability.product_area_id);
        setActiveCapability(action.capability.id);
        prompt = `Add delivery stories and tasks under "${action.capability.name}" in product "${product.name}". Make each story specific, testable, and scoped to this branch.`;
        break;
      case "enhance_work_item":
        setActiveProductArea(action.workItem.product_area_id ?? null);
        setActiveCapability(action.workItem.capability_id ?? null);
        setActiveWorkItem(action.workItem.id);
        prompt = `Enhance story "${action.workItem.title}" in product "${product.name}". Improve the problem statement, acceptance criteria, constraints, and split it into tasks if needed.`;
        break;
    }

    setActiveView("planner");
    navigate("/planner", { state: { plannerPrompt: prompt, plannerView: "conversation" } });
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
              onChange={(event) => setActiveProduct(event.target.value || null)}
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
          activeWorkItemCountOverride={selectedProductWorkItemSummary?.active_count ?? 0}
          references={productReferences}
          isLoading={treeLoading || summariesLoading || referencesLoading}
          onEditProduct={editProduct}
          onEditProductArea={editProductArea}
          onEditCapability={editCapability}
          onOpenWorkItem={openWorkItem}
          onPlanFromItem={planFromItem}
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
