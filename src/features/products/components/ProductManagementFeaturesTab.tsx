import type React from "react";

import { findHierarchyNode } from "../../../lib/hierarchyTree";
import type { CapabilityTree, ProductTree } from "../../../lib/types";
import { getDirectWorkItemCounts, type WorkItemScopeSummaryIndex } from "../lib/productStatusSummary";

type ProductManagementFeaturesTabStyles = {
  managementLayout: React.CSSProperties;
  managementPane: React.CSSProperties;
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

export function ProductManagementFeaturesTab({
  capabilities,
  selectedCapabilityTree,
  features,
  productTree,
  selectedProductId,
  scopeSummaryIndex,
  onSelectCapability,
  onCreateFeature,
  onOpenFeatureStories,
  onEditFeature,
  onDeleteFeature,
  renderCopyableEntityId,
  styles,
}: {
  capabilities: CapabilityTree[];
  selectedCapabilityTree: CapabilityTree | null;
  features: CapabilityTree[];
  productTree: ProductTree | null | undefined;
  selectedProductId: string | null;
  scopeSummaryIndex: WorkItemScopeSummaryIndex;
  onSelectCapability: (capabilityTree: CapabilityTree) => void;
  onCreateFeature: (capabilityTree: CapabilityTree) => void;
  onOpenFeatureStories: (featureTree: CapabilityTree) => void;
  onEditFeature: (featureTree: CapabilityTree) => void;
  onDeleteFeature: (featureTree: CapabilityTree) => void;
  renderCopyableEntityId: (label: string, id: string) => React.ReactNode;
  styles: ProductManagementFeaturesTabStyles;
}) {
  return (
    <div style={styles.managementLayout}>
      <div style={styles.managementPane}>
        <div style={styles.controlLabel}>Capabilities</div>
        <div style={styles.managementList}>
          {capabilities.map((capabilityTree) => (
            <CapabilitySelectorRow
              key={capabilityTree.capability.id}
              capabilityTree={capabilityTree}
              selected={selectedCapabilityTree?.capability.id === capabilityTree.capability.id}
              onSelect={onSelectCapability}
              renderCopyableEntityId={renderCopyableEntityId}
              styles={styles}
            />
          ))}
        </div>
      </div>
      <div style={styles.managementPane}>
        <div style={styles.sectionTitle}>
          <span>{selectedCapabilityTree?.capability.name ?? "Features"}</span>
          <button
            style={styles.btn}
            onClick={() => selectedCapabilityTree && onCreateFeature(selectedCapabilityTree)}
            disabled={!selectedCapabilityTree}
          >
            + Feature
          </button>
        </div>
        {features.length > 0 ? (
          <div style={styles.table}>
            <div style={styles.managementTableHeader}>
              <div>Feature</div>
              <div>Status</div>
              <div>Stories</div>
              <div>Actions</div>
            </div>
            {features.map((featureTree) => {
              const featureNode = findHierarchyNode(productTree?.roots ?? [], featureTree.capability.id, "capability");
              const storyCount = featureNode && selectedProductId
                ? getDirectWorkItemCounts(featureNode, selectedProductId, scopeSummaryIndex).topLevel
                : 0;

              return (
                <div key={featureTree.capability.id} style={styles.managementTableRow}>
                  <div>
                    <div style={styles.rowPrimary}>{featureTree.capability.name}</div>
                    <div style={styles.rowSecondary}>{featureTree.capability.description || "No description yet."}</div>
                    {renderCopyableEntityId("Feature ID", featureTree.capability.id)}
                  </div>
                  <div style={styles.rowCell}>{featureTree.capability.status}</div>
                  <div style={styles.rowCell}>{storyCount}</div>
                  <div style={styles.managementActions}>
                    <button style={styles.compactActionBtn} onClick={() => onOpenFeatureStories(featureTree)}>Stories</button>
                    <button style={styles.compactActionBtn} onClick={() => onEditFeature(featureTree)}>Edit</button>
                    <button style={styles.compactDangerBtn} onClick={() => onDeleteFeature(featureTree)}>Delete</button>
                  </div>
                </div>
              );
            })}
          </div>
        ) : (
          <div style={styles.empty}>No features in this capability yet.</div>
        )}
      </div>
    </div>
  );
}

function CapabilitySelectorRow({
  capabilityTree,
  selected,
  onSelect,
  renderCopyableEntityId,
  styles,
}: {
  capabilityTree: CapabilityTree;
  selected: boolean;
  onSelect: (capabilityTree: CapabilityTree) => void;
  renderCopyableEntityId: (label: string, id: string) => React.ReactNode;
  styles: Pick<ProductManagementFeaturesTabStyles, "managementListButton" | "managementListButtonActive" | "rowPrimary" | "rowSecondary">;
}) {
  const select = () => onSelect(capabilityTree);

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
      <div style={styles.rowPrimary}>{capabilityTree.capability.name}</div>
      <div style={styles.rowSecondary}>{capabilityTree.children.filter((node) => node.capability.node_kind === "feature").length} features</div>
      {renderCopyableEntityId("Capability ID", capabilityTree.capability.id)}
    </div>
  );
}
