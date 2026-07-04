import type React from "react";

import type { Capability, CapabilityTree, ProductArea } from "../../../lib/types";

export interface ProductManagementFeatureEntry {
  capabilityTree: CapabilityTree;
  productArea: ProductArea;
  parentCapability: Capability | null;
}

type ProductManagementWorkItemFeatureSelectorStyles = {
  managementPane: React.CSSProperties;
  controlLabel: React.CSSProperties;
  managementList: React.CSSProperties;
  managementListButton: React.CSSProperties;
  managementListButtonActive: React.CSSProperties;
  rowPrimary: React.CSSProperties;
  rowSecondary: React.CSSProperties;
};

export function ProductManagementWorkItemFeatureSelector({
  features,
  selectedFeature,
  onSelectFeature,
  renderCopyableEntityId,
  styles,
}: {
  features: ProductManagementFeatureEntry[];
  selectedFeature: ProductManagementFeatureEntry | null;
  onSelectFeature: (entry: ProductManagementFeatureEntry) => void;
  renderCopyableEntityId: (label: string, id: string) => React.ReactNode;
  styles: ProductManagementWorkItemFeatureSelectorStyles;
}) {
  return (
    <div style={styles.managementPane}>
      <div style={styles.controlLabel}>Features</div>
      <div style={styles.managementList}>
        {features.map((entry) => (
          <FeatureSelectorRow
            key={entry.capabilityTree.capability.id}
            entry={entry}
            selected={selectedFeature?.capabilityTree.capability.id === entry.capabilityTree.capability.id}
            onSelectFeature={onSelectFeature}
            renderCopyableEntityId={renderCopyableEntityId}
            styles={styles}
          />
        ))}
      </div>
    </div>
  );
}

function FeatureSelectorRow({
  entry,
  selected,
  onSelectFeature,
  renderCopyableEntityId,
  styles,
}: {
  entry: ProductManagementFeatureEntry;
  selected: boolean;
  onSelectFeature: (entry: ProductManagementFeatureEntry) => void;
  renderCopyableEntityId: (label: string, id: string) => React.ReactNode;
  styles: ProductManagementWorkItemFeatureSelectorStyles;
}) {
  const select = () => onSelectFeature(entry);

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
      <div style={styles.rowPrimary}>{entry.capabilityTree.capability.name}</div>
      <div style={styles.rowSecondary}>
        {entry.productArea.name}
        {entry.parentCapability ? ` / ${entry.parentCapability.name}` : ""}
      </div>
      {renderCopyableEntityId("Feature ID", entry.capabilityTree.capability.id)}
    </div>
  );
}
