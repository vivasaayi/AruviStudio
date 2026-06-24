import type { Product } from "../../../lib/types";
import { ScopedRefreshButton } from "./ScopedRefreshButton";
import type { ProductPageTab } from "../lib/productRefreshScopes";
import { styles } from "../lib/productListPageStyles";

type Props = {
  productPageTab: ProductPageTab;
  selectedProductId: string | null;
  selectedProduct: Product | null | undefined;
  products: Product[];
  refreshLabel: string;
  isRefreshDisabled: boolean;
  onProductPageTabChange: (tab: ProductPageTab) => void;
  onSelectedProductChange: (productId: string | null) => void;
  onRefresh: () => Promise<void>;
};

export function ProductPageTabs({
  productPageTab,
  selectedProductId,
  selectedProduct,
  products,
  refreshLabel,
  isRefreshDisabled,
  onProductPageTabChange,
  onSelectedProductChange,
  onRefresh,
}: Props) {
  return (
    <div style={styles.pageTabs}>
      <div style={styles.pageTabGroup}>
        <span style={styles.pageTabGroupLabel}>Catalog</span>
        <button style={productPageTab === "list" ? styles.pageTabActive : styles.pageTab} onClick={() => onProductPageTabChange("list")}>Product List</button>
        <button style={productPageTab === "status" ? styles.pageTabActive : styles.pageTab} onClick={() => onProductPageTabChange("status")}>Product Status</button>
      </div>
      <div style={styles.pageTabGroup}>
        <span style={styles.pageTabGroupLabel}>Selected Product</span>
        <select
          aria-label="Selected product"
          style={styles.pageTabProductSelect}
          value={selectedProductId ?? ""}
          onChange={(event) => onSelectedProductChange(event.target.value || null)}
        >
          <option value="">Select product</option>
          {products.map((product) => <option key={product.id} value={product.id}>{product.name}</option>)}
        </select>
        <button style={productPageTab === "overview" ? styles.pageTabActive : styles.pageTab} onClick={() => onProductPageTabChange("overview")} disabled={!selectedProduct}>Product Overview</button>
        <button style={productPageTab === "design" ? styles.pageTabActive : styles.pageTab} onClick={() => onProductPageTabChange("design")} disabled={!selectedProduct}>Product Management</button>
        <button style={productPageTab === "dependencies" ? styles.pageTabActive : styles.pageTab} onClick={() => onProductPageTabChange("dependencies")} disabled={!selectedProduct}>Dependencies</button>
      </div>
      <div style={styles.tabRefreshSlot}>
        <ScopedRefreshButton
          label={refreshLabel}
          onRefresh={onRefresh}
          disabled={isRefreshDisabled}
        />
      </div>
    </div>
  );
}
