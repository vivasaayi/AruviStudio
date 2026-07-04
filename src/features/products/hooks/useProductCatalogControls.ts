import { useState } from "react";
import type { QueryClient } from "@tanstack/react-query";

import { setSetting } from "../../../lib/tauri";
import type { ProductCatalogSort, ProductCatalogSourceFilter, ProductCatalogStatusFilter } from "../lib/productCatalogRows";
import { HIDE_EXAMPLE_PRODUCTS_KEY } from "../lib/productListPageState";
import type { ProductStatusGroupBy } from "../lib/productRefreshScopes";

type ProductCatalogControlsInput = {
  queryClient: QueryClient;
};

export function useProductCatalogControls({
  queryClient,
}: ProductCatalogControlsInput) {
  const [productSearch, setProductSearch] = useState("");
  const [productStatusFilter, setProductStatusFilter] = useState<ProductCatalogStatusFilter>("all");
  const [productSourceFilter, setProductSourceFilter] = useState<ProductCatalogSourceFilter>("all");
  const [productTagFilter, setProductTagFilter] = useState("all");
  const [productSort, setProductSort] = useState<ProductCatalogSort>("name");
  const [showDefaultProductsInTable, setShowDefaultProductsInTable] = useState(true);
  const [showCustomProductsInTable, setShowCustomProductsInTable] = useState(true);
  const [catalogFilterMsg, setCatalogFilterMsg] = useState<string | null>(null);
  const [catalogFilterError, setCatalogFilterError] = useState<string | null>(null);
  const [statusProductId, setStatusProductId] = useState<string>("all");
  const [statusDepth, setStatusDepth] = useState(1);
  const [statusGroupBy, setStatusGroupBy] = useState<ProductStatusGroupBy>("work_status");

  const updateDefaultProductVisibility = async (includeDefaultProducts: boolean) => {
    try {
      setCatalogFilterMsg(null);
      setCatalogFilterError(null);
      await setSetting(HIDE_EXAMPLE_PRODUCTS_KEY, includeDefaultProducts ? "false" : "true");
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["products"] }),
        queryClient.invalidateQueries({ queryKey: ["setting", HIDE_EXAMPLE_PRODUCTS_KEY] }),
      ]);
      setCatalogFilterMsg(includeDefaultProducts ? "Default products are included." : "Default products are hidden.");
    } catch (error) {
      setCatalogFilterError(String(error));
    }
  };

  return {
    productSearch,
    setProductSearch,
    productStatusFilter,
    setProductStatusFilter,
    productSourceFilter,
    setProductSourceFilter,
    productTagFilter,
    setProductTagFilter,
    productSort,
    setProductSort,
    showDefaultProductsInTable,
    setShowDefaultProductsInTable,
    showCustomProductsInTable,
    setShowCustomProductsInTable,
    catalogFilterMsg,
    catalogFilterError,
    statusProductId,
    setStatusProductId,
    statusDepth,
    setStatusDepth,
    statusGroupBy,
    setStatusGroupBy,
    updateDefaultProductVisibility,
  };
}
