import type React from "react";

import type { Product, ProductAreaTree } from "../../../lib/types";
import { flattenCapabilityTreeList } from "../lib/productHierarchyHelpers";

type ProductManagementAreasTabStyles = {
  section: React.CSSProperties;
  sectionTitle: React.CSSProperties;
  managementActions: React.CSSProperties;
  ghostBtn: React.CSSProperties;
  btn: React.CSSProperties;
  table: React.CSSProperties;
  managementTableHeader: React.CSSProperties;
  managementTableRow: React.CSSProperties;
  rowPrimary: React.CSSProperties;
  rowSecondary: React.CSSProperties;
  rowCell: React.CSSProperties;
  compactActionBtn: React.CSSProperties;
  compactDangerBtn: React.CSSProperties;
  empty: React.CSSProperties;
};

export function ProductManagementAreasTab({
  selectedProduct,
  productAreas,
  onResetProductPlan,
  onCreateProductArea,
  onOpenProductArea,
  onEditProductArea,
  onDeleteProductArea,
  renderCopyableEntityId,
  styles,
}: {
  selectedProduct: Product | null;
  productAreas: ProductAreaTree[];
  onResetProductPlan: (product: Product) => void;
  onCreateProductArea: () => void;
  onOpenProductArea: (productAreaTree: ProductAreaTree) => void;
  onEditProductArea: (productAreaTree: ProductAreaTree) => void;
  onDeleteProductArea: (productAreaTree: ProductAreaTree) => void;
  renderCopyableEntityId: (label: string, id: string) => React.ReactNode;
  styles: ProductManagementAreasTabStyles;
}) {
  return (
    <div style={styles.section}>
      <div style={styles.sectionTitle}>
        <span>Product Areas</span>
        <div style={styles.managementActions}>
          <button
            style={styles.ghostBtn}
            onClick={() => selectedProduct && onResetProductPlan(selectedProduct)}
            disabled={!selectedProduct}
          >
            Reset Plan
          </button>
          <button style={styles.btn} onClick={onCreateProductArea}>+ Product Area</button>
        </div>
      </div>
      {productAreas.length > 0 ? (
        <div style={styles.table}>
          <div style={styles.managementTableHeader}>
            <div>Product Area</div>
            <div>Capabilities</div>
            <div>Features</div>
            <div>Actions</div>
          </div>
          {productAreas.map((productAreaTree) => {
            const capabilityCount = productAreaTree.features.filter((node) => node.capability.node_kind === "capability").length;
            const featureCount = flattenCapabilityTreeList(productAreaTree.features).filter((node) => node.capability.node_kind === "feature").length;

            return (
              <div key={productAreaTree.product_area.id} style={styles.managementTableRow}>
                <div>
                  <div style={styles.rowPrimary}>{productAreaTree.product_area.name}</div>
                  <div style={styles.rowSecondary}>
                    {productAreaTree.product_area.description || productAreaTree.product_area.purpose || "No description yet."}
                  </div>
                  {renderCopyableEntityId("Area ID", productAreaTree.product_area.id)}
                </div>
                <div style={styles.rowCell}>{capabilityCount}</div>
                <div style={styles.rowCell}>{featureCount}</div>
                <div style={styles.managementActions}>
                  <button style={styles.compactActionBtn} onClick={() => onOpenProductArea(productAreaTree)}>Open</button>
                  <button style={styles.compactActionBtn} onClick={() => onEditProductArea(productAreaTree)}>Edit</button>
                  <button style={styles.compactDangerBtn} onClick={() => onDeleteProductArea(productAreaTree)}>Delete</button>
                </div>
              </div>
            );
          })}
        </div>
      ) : (
        <div style={styles.empty}>No product areas yet. Add a product area to start the management model.</div>
      )}
    </div>
  );
}
