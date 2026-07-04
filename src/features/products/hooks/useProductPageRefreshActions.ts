import type { QueryClient } from "@tanstack/react-query";

import { HIDE_EXAMPLE_PRODUCTS_KEY } from "../lib/productListPageState";
import { refreshScopedProductQueries } from "../lib/productQueryRefresh";
import {
  getProductManagementRefreshLabel,
  getProductManagementRefreshQueryKeys,
  getProductPageRefreshLabel,
  getProductPageRefreshQueryKeys,
  isProductPageRefreshDisabled,
  type ProductManagementTab,
  type ProductPageTab,
  type ProductStatusGroupBy,
} from "../lib/productRefreshScopes";

type ProductPageRefreshActionsInput = {
  queryClient: QueryClient;
  productPageTab: ProductPageTab;
  selectedProductId: string | null;
  statusGroupBy: ProductStatusGroupBy;
  statusProductId: string;
  productManagementTab: ProductManagementTab;
  selectedManagementStoryIdForTasks: string | null;
};

export function useProductPageRefreshActions({
  queryClient,
  productPageTab,
  selectedProductId,
  statusGroupBy,
  statusProductId,
  productManagementTab,
  selectedManagementStoryIdForTasks,
}: ProductPageRefreshActionsInput) {
  const refreshProductManagementTabQueries = async () => {
    await refreshScopedProductQueries(queryClient, getProductManagementRefreshQueryKeys({
      selectedProductId,
      productManagementTab,
      selectedManagementStoryIdForTasks,
    }));
  };

  const refreshActiveProductPageTab = async () => {
    await refreshScopedProductQueries(queryClient, getProductPageRefreshQueryKeys({
      productPageTab,
      selectedProductId,
      statusGroupBy,
      statusProductId,
      hideExampleProductsKey: HIDE_EXAMPLE_PRODUCTS_KEY,
      productManagementTab,
      selectedManagementStoryIdForTasks,
    }));
  };

  return {
    activeProductPageRefreshLabel: getProductPageRefreshLabel(productPageTab),
    activeProductPageRefreshDisabled: isProductPageRefreshDisabled(productPageTab, selectedProductId),
    productManagementRefreshLabel: getProductManagementRefreshLabel(productManagementTab),
    refreshActiveProductPageTab,
    refreshProductManagementTabQueries,
  };
}
