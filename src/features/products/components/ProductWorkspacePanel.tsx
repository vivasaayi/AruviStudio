import type { Dispatch, ReactNode, SetStateAction } from "react";

import type { Product, ProductDependency } from "../../../lib/types";
import { ProductOverviewPage } from "../pages/ProductOverviewPage";
import type { ProductDependencyDraft } from "../lib/productListPageState";
import type {
  ProductCatalogRow,
  ProductCatalogSort,
  ProductCatalogSourceFilter,
  ProductCatalogStatusFilter,
} from "../lib/productCatalogRows";
import type { ProductPageTab, ProductStatusGroupBy } from "../lib/productRefreshScopes";
import type { ProductStatusSummary, StatusRow } from "../lib/productStatusSummary";
import { styles } from "../lib/productListPageStyles";
import { ProductCatalogTab } from "./ProductCatalogTab";
import { ProductDependenciesTab } from "./ProductDependenciesTab";
import { ProductStatusTab } from "./ProductStatusTab";

type CapabilityOption = {
  id: string;
  label: string;
};

type ProductWorkspacePanelProps = {
  productPageTab: ProductPageTab;
  selectedProduct: Product | null;
  products: Product[];
  isLoading: boolean;
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
  statusProductId: string;
  statusDepth: number;
  statusGroupBy: ProductStatusGroupBy;
  statusSummary: ProductStatusSummary;
  statusRows: StatusRow[];
  onStatusProductChange: (productId: string) => void;
  onStatusDepthChange: (depth: number) => void;
  onStatusGroupByChange: (groupBy: ProductStatusGroupBy) => void;
  onOpenStatusRow: (row: StatusRow) => void;
  selectedProductId: string | null;
  dependencyDraft: ProductDependencyDraft;
  setDependencyDraft: Dispatch<SetStateAction<ProductDependencyDraft>>;
  selectedCapabilityOptions: CapabilityOption[];
  dependencyTargetCapabilityOptions: CapabilityOption[];
  selectedProductDependencies: ProductDependency[];
  productNameById: Map<string, string>;
  capabilityLabelById: Map<string, string>;
  isCreatingDependency: boolean;
  onCreateDependency: () => void;
  productManagementConsole: ReactNode;
};

export function ProductWorkspacePanel({
  productPageTab,
  selectedProduct,
  products,
  isLoading,
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
  statusProductId,
  statusDepth,
  statusGroupBy,
  statusSummary,
  statusRows,
  onStatusProductChange,
  onStatusDepthChange,
  onStatusGroupByChange,
  onOpenStatusRow,
  selectedProductId,
  dependencyDraft,
  setDependencyDraft,
  selectedCapabilityOptions,
  dependencyTargetCapabilityOptions,
  selectedProductDependencies,
  productNameById,
  capabilityLabelById,
  isCreatingDependency,
  onCreateDependency,
  productManagementConsole,
}: ProductWorkspacePanelProps) {
  return (
    <div style={styles.workspace}>
      <div style={styles.panel}>
        <div style={styles.panelInner}>
          {productPageTab === "list" ? (
            <ProductCatalogTab
              productSearch={productSearch}
              productStatusFilter={productStatusFilter}
              productSourceFilter={productSourceFilter}
              productTagFilter={productTagFilter}
              productSort={productSort}
              allProductTags={allProductTags}
              showCustomProductsInTable={showCustomProductsInTable}
              showDefaultProductsInTable={showDefaultProductsInTable}
              includeDefaultProductsInCatalog={includeDefaultProductsInCatalog}
              catalogFilterMsg={catalogFilterMsg}
              catalogFilterError={catalogFilterError}
              productTableRows={productTableRows}
              isLoading={isLoading}
              onProductSearchChange={onProductSearchChange}
              onProductStatusFilterChange={onProductStatusFilterChange}
              onProductSourceFilterChange={onProductSourceFilterChange}
              onProductTagFilterChange={onProductTagFilterChange}
              onProductSortChange={onProductSortChange}
              onShowCustomProductsInTableChange={onShowCustomProductsInTableChange}
              onShowDefaultProductsInTableChange={onShowDefaultProductsInTableChange}
              onIncludeDefaultProductsInCatalogChange={onIncludeDefaultProductsInCatalogChange}
              onAddProduct={onAddProduct}
              onEditProduct={onEditProduct}
              onOpenProductStatus={onOpenProductStatus}
              onOpenProductOverview={onOpenProductOverview}
              onOpenProductDesign={onOpenProductDesign}
              onOpenProductDependencies={onOpenProductDependencies}
              onDeleteProduct={onDeleteProduct}
              styles={styles}
            />
          ) : productPageTab === "status" ? (
            <ProductStatusTab
              products={products}
              statusProductId={statusProductId}
              statusDepth={statusDepth}
              statusGroupBy={statusGroupBy}
              statusSummary={statusSummary}
              statusRows={statusRows}
              isLoading={isLoading}
              onStatusProductChange={onStatusProductChange}
              onStatusDepthChange={onStatusDepthChange}
              onStatusGroupByChange={onStatusGroupByChange}
              onOpenStatusRow={onOpenStatusRow}
              styles={styles}
            />
          ) : productPageTab === "overview" ? (
            selectedProduct ? (
              <ProductOverviewPage />
            ) : (
              <div style={styles.empty}>Select a product to view the product overview.</div>
            )
          ) : productPageTab === "dependencies" ? (
            selectedProduct ? (
              <ProductDependenciesTab
                selectedProduct={selectedProduct}
                products={products}
                selectedProductId={selectedProductId}
                dependencyDraft={dependencyDraft}
                setDependencyDraft={setDependencyDraft}
                selectedCapabilityOptions={selectedCapabilityOptions}
                dependencyTargetCapabilityOptions={dependencyTargetCapabilityOptions}
                selectedProductDependencies={selectedProductDependencies}
                productNameById={productNameById}
                capabilityLabelById={capabilityLabelById}
                isCreatingDependency={isCreatingDependency}
                onCreateDependency={onCreateDependency}
                styles={styles}
              />
            ) : (
              <div style={styles.empty}>Select a product before editing dependencies.</div>
            )
          ) : selectedProduct ? (
            productManagementConsole
          ) : (
            <div style={styles.empty}>
              {isLoading
                ? "Loading products..."
                : products.length > 0
                  ? "Select a product from Product List to start refining the management tree."
                  : "No visible products yet. Use Add Product or disable Hide Example Products in Settings."}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
