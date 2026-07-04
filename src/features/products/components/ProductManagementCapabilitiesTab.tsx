import type React from "react";

import { findHierarchyNode } from "../../../lib/hierarchyTree";
import type { CapabilityTree, ProductAreaTree, ProductTree } from "../../../lib/types";
import { getSubtreeWorkItemCounts, type WorkItemScopeSummaryIndex } from "../lib/productStatusSummary";

type ProductManagementCapabilitiesTabStyles = {
  managementLayout: React.CSSProperties;
  managementPane: React.CSSProperties;
  managementPaneHeader: React.CSSProperties;
  controlLabel: React.CSSProperties;
  managementList: React.CSSProperties;
  managementListButton: React.CSSProperties;
  managementListButtonActive: React.CSSProperties;
  sectionTitle: React.CSSProperties;
  btn: React.CSSProperties;
  table: React.CSSProperties;
  managementTableHeader: React.CSSProperties;
  managementTableRow: React.CSSProperties;
  rowPrimary: React.CSSProperties;
  rowSecondary: React.CSSProperties;
  rowCell: React.CSSProperties;
  managementActions: React.CSSProperties;
  compactActionBtn: React.CSSProperties;
  compactDangerBtn: React.CSSProperties;
  empty: React.CSSProperties;
};

export function ProductManagementCapabilitiesTab({
  productAreas,
  selectedProductAreaTree,
  capabilities,
  productTree,
  selectedProductId,
  scopeSummaryIndex,
  onSelectProductArea,
  onCreateCapability,
  onOpenCapability,
  onEditCapability,
  onDeleteCapability,
  renderCopyableEntityId,
  styles,
}: {
  productAreas: ProductAreaTree[];
  selectedProductAreaTree: ProductAreaTree | null;
  capabilities: CapabilityTree[];
  productTree: ProductTree | null | undefined;
  selectedProductId: string | null;
  scopeSummaryIndex: WorkItemScopeSummaryIndex;
  onSelectProductArea: (productAreaTree: ProductAreaTree) => void;
  onCreateCapability: (productAreaTree: ProductAreaTree) => void;
  onOpenCapability: (capabilityTree: CapabilityTree) => void;
  onEditCapability: (capabilityTree: CapabilityTree) => void;
  onDeleteCapability: (capabilityTree: CapabilityTree) => void;
  renderCopyableEntityId: (label: string, id: string) => React.ReactNode;
  styles: ProductManagementCapabilitiesTabStyles;
}) {
  return (
    <div style={styles.managementLayout}>
      <div style={styles.managementPane}>
        <div style={styles.managementPaneHeader}>
          <div style={styles.controlLabel}>Product Areas</div>
        </div>
        <div style={styles.managementList}>
          {productAreas.map((productAreaTree) => (
            <ProductAreaSelectorRow
              key={productAreaTree.product_area.id}
              productAreaTree={productAreaTree}
              selected={selectedProductAreaTree?.product_area.id === productAreaTree.product_area.id}
              onSelect={onSelectProductArea}
              renderCopyableEntityId={renderCopyableEntityId}
              styles={styles}
            />
          ))}
        </div>
      </div>
      <div style={styles.managementPane}>
        <div style={styles.sectionTitle}>
          <span>{selectedProductAreaTree?.product_area.name ?? "Capabilities"}</span>
          <button
            style={styles.btn}
            onClick={() => selectedProductAreaTree && onCreateCapability(selectedProductAreaTree)}
            disabled={!selectedProductAreaTree}
          >
            + Capability
          </button>
        </div>
        {capabilities.length > 0 ? (
          <div style={styles.table}>
            <div style={styles.managementTableHeader}>
              <div>Capability</div>
              <div>Features</div>
              <div>Stories</div>
              <div>Actions</div>
            </div>
            {capabilities.map((capabilityTree) => {
              const capabilityNode = findHierarchyNode(productTree?.roots ?? [], capabilityTree.capability.id, "capability");
              const storyCount = capabilityNode && selectedProductId
                ? getSubtreeWorkItemCounts(capabilityNode, selectedProductId, scopeSummaryIndex).topLevel
                : 0;

              return (
                <div key={capabilityTree.capability.id} style={styles.managementTableRow}>
                  <div>
                    <div style={styles.rowPrimary}>{capabilityTree.capability.name}</div>
                    <div style={styles.rowSecondary}>{capabilityTree.capability.description || "No description yet."}</div>
                    {renderCopyableEntityId("Capability ID", capabilityTree.capability.id)}
                  </div>
                  <div style={styles.rowCell}>{capabilityTree.children.filter((node) => node.capability.node_kind === "feature").length}</div>
                  <div style={styles.rowCell}>{storyCount}</div>
                  <div style={styles.managementActions}>
                    <button style={styles.compactActionBtn} onClick={() => onOpenCapability(capabilityTree)}>Open</button>
                    <button style={styles.compactActionBtn} onClick={() => onEditCapability(capabilityTree)}>Edit</button>
                    <button style={styles.compactDangerBtn} onClick={() => onDeleteCapability(capabilityTree)}>Delete</button>
                  </div>
                </div>
              );
            })}
          </div>
        ) : (
          <div style={styles.empty}>No capabilities in this product area yet.</div>
        )}
      </div>
    </div>
  );
}

function ProductAreaSelectorRow({
  productAreaTree,
  selected,
  onSelect,
  renderCopyableEntityId,
  styles,
}: {
  productAreaTree: ProductAreaTree;
  selected: boolean;
  onSelect: (productAreaTree: ProductAreaTree) => void;
  renderCopyableEntityId: (label: string, id: string) => React.ReactNode;
  styles: Pick<ProductManagementCapabilitiesTabStyles, "managementListButton" | "managementListButtonActive" | "rowPrimary" | "rowSecondary">;
}) {
  const select = () => onSelect(productAreaTree);

  return (
    <div
      style={selected ? styles.managementListButtonActive : styles.managementListButton}
      onClick={select}
      onKeyDown={(event) => {
        if (event.key !== "Enter" && event.key !== " ") {
          return;
        }
        event.preventDefault();
        select();
      }}
      role="button"
      tabIndex={0}
    >
      <div style={styles.rowPrimary}>{productAreaTree.product_area.name}</div>
      <div style={styles.rowSecondary}>{productAreaTree.features.length} child nodes</div>
      {renderCopyableEntityId("Area ID", productAreaTree.product_area.id)}
    </div>
  );
}
