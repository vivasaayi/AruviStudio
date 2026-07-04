import type React from "react";

import type { Product } from "../../../lib/types";
import type {
  ProductCatalogRow,
  ProductCatalogSort,
  ProductCatalogSourceFilter,
  ProductCatalogStatusFilter,
} from "../lib/productCatalogRows";

type ProductCatalogTabStyles = {
  toolbar: React.CSSProperties;
  controlLabel: React.CSSProperties;
  input: React.CSSProperties;
  select: React.CSSProperties;
  btn: React.CSSProperties;
  toggleRow: React.CSSProperties;
  checkboxLabel: React.CSSProperties;
  contextText: React.CSSProperties;
  errorText: React.CSSProperties;
  table: React.CSSProperties;
  productTableHeader: React.CSSProperties;
  productTableRow: React.CSSProperties;
  productCell: React.CSSProperties;
  rowPrimary: React.CSSProperties;
  productDescription: React.CSSProperties;
  chipRow: React.CSSProperties;
  badgeMuted: React.CSSProperties;
  productMetaCell: React.CSSProperties;
  rowSecondary: React.CSSProperties;
  rowCell: React.CSSProperties;
  progressTrack: React.CSSProperties;
  progressFill: React.CSSProperties;
  productActions: React.CSSProperties;
  compactActionBtn: React.CSSProperties;
  compactDangerBtn: React.CSSProperties;
  empty: React.CSSProperties;
};

export function ProductCatalogTab({
  productSearch,
  productStatusFilter,
  productSourceFilter,
  productTagFilter,
  productSort,
  allProductTags,
  showCustomProductsInTable,
  showDefaultProductsInTable,
  includeDefaultProductsInCatalog,
  catalogFilterMsg,
  catalogFilterError,
  productTableRows,
  isLoading,
  onProductSearchChange,
  onProductStatusFilterChange,
  onProductSourceFilterChange,
  onProductTagFilterChange,
  onProductSortChange,
  onShowCustomProductsInTableChange,
  onShowDefaultProductsInTableChange,
  onIncludeDefaultProductsInCatalogChange,
  onAddProduct,
  onEditProduct,
  onOpenProductStatus,
  onOpenProductOverview,
  onOpenProductDesign,
  onOpenProductDependencies,
  onDeleteProduct,
  styles,
}: {
  productSearch: string;
  productStatusFilter: ProductCatalogStatusFilter;
  productSourceFilter: ProductCatalogSourceFilter;
  productTagFilter: string;
  productSort: ProductCatalogSort;
  allProductTags: string[];
  showCustomProductsInTable: boolean;
  showDefaultProductsInTable: boolean;
  includeDefaultProductsInCatalog: boolean;
  catalogFilterMsg: string | null;
  catalogFilterError: string | null;
  productTableRows: ProductCatalogRow[];
  isLoading: boolean;
  onProductSearchChange: (search: string) => void;
  onProductStatusFilterChange: (filter: ProductCatalogStatusFilter) => void;
  onProductSourceFilterChange: (filter: ProductCatalogSourceFilter) => void;
  onProductTagFilterChange: (tag: string) => void;
  onProductSortChange: (sort: ProductCatalogSort) => void;
  onShowCustomProductsInTableChange: (show: boolean) => void;
  onShowDefaultProductsInTableChange: (show: boolean) => void;
  onIncludeDefaultProductsInCatalogChange: (include: boolean) => void | Promise<void>;
  onAddProduct: () => void;
  onEditProduct: (product: Product) => void;
  onOpenProductStatus: (product: Product) => void;
  onOpenProductOverview: (product: Product) => void;
  onOpenProductDesign: (product: Product) => void;
  onOpenProductDependencies: (product: Product) => void;
  onDeleteProduct: (product: Product) => void;
  styles: ProductCatalogTabStyles;
}) {
  return (
    <>
      <div style={styles.toolbar}>
        <div>
          <div style={styles.controlLabel}>Search</div>
          <input
            style={styles.input}
            value={productSearch}
            onChange={(event) => onProductSearchChange(event.target.value)}
            placeholder="Filter by name, tag, status, or description"
          />
        </div>
        <div>
          <div style={styles.controlLabel}>Status</div>
          <select
            style={styles.select}
            value={productStatusFilter}
            onChange={(event) => onProductStatusFilterChange(event.target.value as ProductCatalogStatusFilter)}
          >
            <option value="all">All statuses</option>
            <option value="active">Active</option>
            <option value="archived">Archived</option>
          </select>
        </div>
        <div>
          <div style={styles.controlLabel}>Source</div>
          <select
            style={styles.select}
            value={productSourceFilter}
            onChange={(event) => onProductSourceFilterChange(event.target.value as ProductCatalogSourceFilter)}
          >
            <option value="all">All sources</option>
            <option value="custom">Custom</option>
            <option value="default">Default</option>
          </select>
        </div>
        <div>
          <div style={styles.controlLabel}>Tag</div>
          <select style={styles.select} value={productTagFilter} onChange={(event) => onProductTagFilterChange(event.target.value)}>
            <option value="all">All tags</option>
            {allProductTags.map((tag) => <option key={tag} value={tag}>{tag}</option>)}
          </select>
        </div>
        <div>
          <div style={styles.controlLabel}>Sort</div>
          <select
            style={styles.select}
            value={productSort}
            onChange={(event) => onProductSortChange(event.target.value as ProductCatalogSort)}
          >
            <option value="name">Name</option>
            <option value="updated">Recently updated</option>
            <option value="progress">Progress</option>
            <option value="work">Work items</option>
          </select>
        </div>
        <button style={{ ...styles.btn, alignSelf: "center", justifySelf: "end" }} onClick={onAddProduct}>+ Add Product</button>
      </div>
      <div style={styles.toggleRow}>
        <label style={styles.checkboxLabel}>
          <input type="checkbox" checked={showCustomProductsInTable} onChange={(event) => onShowCustomProductsInTableChange(event.target.checked)} />
          Show custom products
        </label>
        <label style={styles.checkboxLabel}>
          <input type="checkbox" checked={showDefaultProductsInTable} onChange={(event) => onShowDefaultProductsInTableChange(event.target.checked)} />
          Show default products in table
        </label>
        <label style={styles.checkboxLabel}>
          <input
            type="checkbox"
            checked={includeDefaultProductsInCatalog}
            onChange={(event) => void onIncludeDefaultProductsInCatalogChange(event.target.checked)}
          />
          Include default products from catalog
        </label>
        {catalogFilterMsg && <span style={{ ...styles.contextText, color: "#4ec9b0" }}>{catalogFilterMsg}</span>}
        {catalogFilterError && <span style={styles.errorText}>{catalogFilterError}</span>}
      </div>
      <div style={styles.table}>
        <div style={styles.productTableHeader}>
          <div>Product</div>
          <div>Source</div>
          <div>Status</div>
          <div>Management</div>
          <div>Progress</div>
          <div>Actions</div>
        </div>
        {productTableRows.length > 0 ? productTableRows.map((row) => (
          <div key={row.product.id} style={styles.productTableRow}>
            <div style={styles.productCell}>
              <div style={styles.rowPrimary}>{row.product.name}</div>
              <div style={styles.productDescription}>{row.product.description || row.product.vision || "No description yet."}</div>
              <div style={styles.chipRow}>
                {row.product.tags.slice(0, 4).map((tag) => <span key={tag} style={styles.badgeMuted}>{tag}</span>)}
              </div>
            </div>
            <div style={styles.productMetaCell}>{row.source}</div>
            <div style={styles.productMetaCell}>
              <div>{row.product.lifecycle}</div>
              <div style={styles.rowSecondary}>{row.product.health}</div>
            </div>
            <div style={styles.productMetaCell}>
              <div>{row.rootCount} product areas</div>
              <div style={styles.rowSecondary}>{row.nodeCount} management nodes</div>
            </div>
            <div>
              <div style={styles.rowCell}>{row.progress.percent}%</div>
              <div style={styles.progressTrack}><div style={{ ...styles.progressFill, width: `${row.progress.percent}%` }} /></div>
              <div style={styles.rowSecondary}>{row.progress.done}/{row.progress.total} done</div>
              <div style={styles.rowSecondary}>{row.activeWorkItemCount} active stories</div>
            </div>
            <div style={styles.productActions}>
              <button style={styles.compactActionBtn} onClick={() => onEditProduct(row.product)}>Edit</button>
              <button style={styles.compactActionBtn} onClick={() => onOpenProductStatus(row.product)}>Status</button>
              <button style={styles.compactActionBtn} onClick={() => onOpenProductOverview(row.product)}>Overview</button>
              <button style={styles.compactActionBtn} onClick={() => onOpenProductDesign(row.product)}>Manage</button>
              <button style={styles.compactActionBtn} onClick={() => onOpenProductDependencies(row.product)}>Dependencies</button>
              <button style={styles.compactDangerBtn} onClick={() => onDeleteProduct(row.product)}>Delete</button>
            </div>
          </div>
        )) : (
          <div style={styles.empty}>{isLoading ? "Loading products..." : "No products match the current filters."}</div>
        )}
      </div>
    </>
  );
}
