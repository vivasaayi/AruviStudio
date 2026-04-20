import React, { useEffect } from "react";
import { useQuery } from "@tanstack/react-query";
import { useNavigate } from "react-router-dom";
import { getProductTree, listProducts, listWorkItems } from "../../../lib/tauri";
import { useWorkspaceStore } from "../../../state/workspaceStore";
import { useUIStore } from "../../../state/uiStore";
import { ProductOverviewDocument } from "../components/ProductOverviewDocument";
import type { Capability, Module, Product, WorkItem } from "../../../lib/types";
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

  const { data: products = [], isLoading: productsLoading } = useQuery<Product[]>({
    queryKey: ["products"],
    queryFn: listProducts,
  });

  const selectedProductId = activeProductId ?? products[0]?.id ?? null;
  const selectedProduct = products.find((product) => product.id === selectedProductId) ?? null;

  useEffect(() => {
    if (!activeProductId && products[0]?.id) {
      setActiveProduct(products[0].id);
    }
  }, [activeProductId, products, setActiveProduct]);

  const { data: tree, isLoading: treeLoading } = useQuery({
    queryKey: ["productOverviewTree", selectedProductId],
    queryFn: () => getProductTree(selectedProductId!),
    enabled: !!selectedProductId,
  });

  const { data: workItems = [], isLoading: workItemsLoading } = useQuery<WorkItem[]>({
    queryKey: ["productOverviewPageWorkItems", selectedProductId],
    queryFn: () => listWorkItems({ productId: selectedProductId ?? undefined }),
    enabled: !!selectedProductId,
  });

  const goToProductWorkspace = () => {
    setActiveView("products");
    navigate("/products");
  };

  const exportHtml = () => {
    if (!selectedProduct) {
      return;
    }

    const html = buildProductOverviewHtml({
      product: selectedProduct,
      tree,
      workItems,
    });
    const blob = new Blob([html], { type: "text/html;charset=utf-8" });
    const url = window.URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = `${slugify(selectedProduct.name)}-overview.html`;
    document.body.appendChild(anchor);
    anchor.click();
    anchor.remove();
    window.setTimeout(() => window.URL.revokeObjectURL(url), 0);
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
            This route is product-wide. Use the single dropdown here instead of the left hierarchy rail.
          </div>
          <div style={styles.actionRow}>
            <button
              style={styles.primaryBtn}
              onClick={exportHtml}
              disabled={!selectedProduct || treeLoading || workItemsLoading}
            >
              Export HTML
            </button>
            <button style={styles.ghostBtn} onClick={goToProductWorkspace}>
              Open Products
            </button>
          </div>
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
