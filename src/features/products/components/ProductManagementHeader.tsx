import type React from "react";

import { ScopedRefreshButton } from "./ScopedRefreshButton";
import type { Product } from "../../../lib/types";
import type { ProductManagementTab } from "../lib/productRefreshScopes";

const MANAGEMENT_TABS: Array<{ id: ProductManagementTab; label: string }> = [
  { id: "areas", label: "Product Areas" },
  { id: "capabilities", label: "Capabilities" },
  { id: "features", label: "Features" },
  { id: "work_items", label: "Work Items" },
];

type ProductManagementHeaderStyles = {
  contextCard: React.CSSProperties;
  contextLabel: React.CSSProperties;
  contextTitle: React.CSSProperties;
  managementTabs: React.CSSProperties;
  tab: React.CSSProperties;
  tabActive: React.CSSProperties;
  tabRefreshSlot: React.CSSProperties;
};

export function ProductManagementHeader({
  selectedProduct,
  activeTab,
  onTabChange,
  refreshLabel,
  onRefresh,
  refreshDisabled,
  renderCopyableEntityId,
  styles,
}: {
  selectedProduct: Product | null;
  activeTab: ProductManagementTab;
  onTabChange: (tab: ProductManagementTab) => void;
  refreshLabel: string;
  onRefresh: () => Promise<void>;
  refreshDisabled: boolean;
  renderCopyableEntityId: (label: string, id: string) => React.ReactNode;
  styles: ProductManagementHeaderStyles;
}) {
  return (
    <>
      {selectedProduct ? (
        <div style={styles.contextCard}>
          <div style={styles.contextLabel}>Selected Product</div>
          <div style={styles.contextTitle}>{selectedProduct.name}</div>
          {renderCopyableEntityId("Product ID", selectedProduct.id)}
        </div>
      ) : null}
      <div style={styles.managementTabs}>
        {MANAGEMENT_TABS.map((tab) => (
          <button
            key={tab.id}
            style={activeTab === tab.id ? styles.tabActive : styles.tab}
            onClick={() => onTabChange(tab.id)}
          >
            {tab.label}
          </button>
        ))}
        <div style={styles.tabRefreshSlot}>
          <ScopedRefreshButton
            label={refreshLabel}
            onRefresh={onRefresh}
            disabled={refreshDisabled}
          />
        </div>
      </div>
    </>
  );
}
