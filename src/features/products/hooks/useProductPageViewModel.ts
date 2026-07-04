import { useMemo } from "react";
import { useQueries, useQuery } from "@tanstack/react-query";

import {
  getProductTree,
  getSetting,
  listProductDependencies,
  listProducts,
  summarizeProductTree,
  summarizeWorkItemsByProduct,
  summarizeWorkItemsByScope,
} from "../../../lib/tauri";
import type {
  ProductTree,
  ProductTreeSummary,
  ProductWorkItemSummary,
  WorkItemScopeSummary,
} from "../../../lib/types";
import {
  HIDE_EXAMPLE_PRODUCTS_KEY,
  parseBooleanSetting,
} from "../lib/productListPageState";
import {
  buildProductCatalogRows,
  getProductCatalogTags,
  type ProductCatalogSort,
  type ProductCatalogSourceFilter,
  type ProductCatalogStatusFilter,
} from "../lib/productCatalogRows";
import type { ProductPageTab, ProductStatusGroupBy } from "../lib/productRefreshScopes";
import {
  buildProductStatusSummary,
  buildStatusRows,
  buildWorkItemScopeSummaryIndex,
} from "../lib/productStatusSummary";

type ProductPageViewModelInput = {
  activeProductId: string | null;
  productPageTab: ProductPageTab;
  productSearch: string;
  productStatusFilter: ProductCatalogStatusFilter;
  productSourceFilter: ProductCatalogSourceFilter;
  productTagFilter: string;
  productSort: ProductCatalogSort;
  showDefaultProductsInTable: boolean;
  showCustomProductsInTable: boolean;
  statusProductId: string;
  statusDepth: number;
  statusGroupBy: ProductStatusGroupBy;
};

export function useProductPageViewModel({
  activeProductId,
  productPageTab,
  productSearch,
  productStatusFilter,
  productSourceFilter,
  productTagFilter,
  productSort,
  showDefaultProductsInTable,
  showCustomProductsInTable,
  statusProductId,
  statusDepth,
  statusGroupBy,
}: ProductPageViewModelInput) {
  const { data: products, isLoading } = useQuery({ queryKey: ["products"], queryFn: listProducts });
  const { data: productDependencies = [] } = useQuery({
    queryKey: ["product-dependencies"],
    queryFn: listProductDependencies,
    enabled: productPageTab === "design" || productPageTab === "dependencies",
  });
  const { data: hideExampleProductsSetting } = useQuery({
    queryKey: ["setting", HIDE_EXAMPLE_PRODUCTS_KEY],
    queryFn: () => getSetting(HIDE_EXAMPLE_PRODUCTS_KEY),
  });
  const { data: productWorkItemSummaries = [] } = useQuery<ProductWorkItemSummary[]>({
    queryKey: ["productWorkItemSummary"],
    queryFn: summarizeWorkItemsByProduct,
    enabled: productPageTab === "list" || productPageTab === "status",
  });
  const visibleActiveProductId = products?.some((product) => product.id === activeProductId)
    ? activeProductId
    : null;
  const selectedProductId = visibleActiveProductId ?? products?.[0]?.id ?? null;
  const selectedProduct = useMemo(
    () => products?.find((product) => product.id === selectedProductId) ?? null,
    [products, selectedProductId],
  );
  const { data: tree } = useQuery({
    queryKey: ["productTree", selectedProductId],
    queryFn: () => getProductTree(selectedProductId!),
    enabled: !!selectedProduct && (productPageTab === "design" || productPageTab === "dependencies"),
  });

  const productTreeQueries = useQueries({
    queries: (products ?? []).map((product) => ({
      queryKey: ["productTree", product.id],
      queryFn: () => getProductTree(product.id),
      enabled: !!product.id && (productPageTab === "dependencies" || (productPageTab === "status" && statusGroupBy !== "work_status")),
    })),
  });

  const productTreeSummaryQueries = useQueries({
    queries: (products ?? []).map((product) => ({
      queryKey: ["productTreeSummary", product.id],
      queryFn: () => summarizeProductTree(product.id),
      enabled: !!product.id && (productPageTab === "list" || productPageTab === "status"),
    })),
  });

  const workItemScopeSummaryProductId = productPageTab === "status"
    ? statusProductId === "all" ? undefined : statusProductId
    : selectedProductId ?? undefined;
  const { data: workItemScopeSummaries = [] } = useQuery<WorkItemScopeSummary[]>({
    queryKey: ["workItemScopeSummary", workItemScopeSummaryProductId ?? "all"],
    queryFn: () => summarizeWorkItemsByScope({ productId: workItemScopeSummaryProductId }),
    enabled: productPageTab === "status" || (!!selectedProduct && productPageTab === "design"),
  });

  const productTreeById = useMemo(() => {
    const map = new Map<string, ProductTree>();
    (products ?? []).forEach((product, index) => {
      const result = productTreeQueries[index]?.data;
      if (result) {
        map.set(product.id, result);
      }
    });
    return map;
  }, [productTreeQueries, products]);

  const productTreeSummaryById = useMemo(() => {
    const map = new Map<string, ProductTreeSummary>();
    (products ?? []).forEach((product, index) => {
      const result = productTreeSummaryQueries[index]?.data;
      if (result) {
        map.set(product.id, result);
      }
    });
    return map;
  }, [productTreeSummaryQueries, products]);

  const scopeSummaryIndex = useMemo(() => buildWorkItemScopeSummaryIndex(workItemScopeSummaries), [workItemScopeSummaries]);

  const productSummaryById = useMemo(() => {
    const map = new Map<string, ProductWorkItemSummary>();
    productWorkItemSummaries.forEach((summary) => map.set(summary.product_id, summary));
    return map;
  }, [productWorkItemSummaries]);

  const allProductTags = useMemo(() => getProductCatalogTags(products ?? []), [products]);
  const includeDefaultProductsInCatalog = !parseBooleanSetting(hideExampleProductsSetting, true);
  const productTableRows = useMemo(() => buildProductCatalogRows({
    products: products ?? [],
    productTreeSummaryById,
    productSummaryById,
    search: productSearch,
    statusFilter: productStatusFilter,
    sourceFilter: productSourceFilter,
    tagFilter: productTagFilter,
    sort: productSort,
    showDefaultProducts: showDefaultProductsInTable,
    showCustomProducts: showCustomProductsInTable,
  }), [
    productSearch,
    productSort,
    productSourceFilter,
    productStatusFilter,
    productTagFilter,
    productSummaryById,
    productTreeSummaryById,
    products,
    showCustomProductsInTable,
    showDefaultProductsInTable,
  ]);

  const selectedStatusProduct = statusProductId === "all"
    ? null
    : products?.find((product) => product.id === statusProductId) ?? null;
  const selectedStatusProducts = selectedStatusProduct ? [selectedStatusProduct] : (products ?? []);
  const statusSummary = useMemo(
    () => buildProductStatusSummary(selectedStatusProducts, productTreeSummaryById, productSummaryById),
    [productSummaryById, productTreeSummaryById, selectedStatusProducts],
  );
  const statusRows = useMemo(
    () => buildStatusRows(selectedStatusProducts, productTreeById, scopeSummaryIndex, statusDepth, statusGroupBy),
    [productTreeById, scopeSummaryIndex, selectedStatusProducts, statusDepth, statusGroupBy],
  );

  return {
    products,
    isLoading,
    productDependencies,
    selectedProductId,
    selectedProduct,
    tree,
    productTreeById,
    productTreeSummaryById,
    scopeSummaryIndex,
    allProductTags,
    includeDefaultProductsInCatalog,
    productTableRows,
    statusSummary,
    statusRows,
  };
}
