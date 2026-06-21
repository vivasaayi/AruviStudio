import type { QueryKey } from "@tanstack/react-query";

export type ProductPageTab = "list" | "status" | "overview" | "design" | "dependencies";
export type ProductManagementTab = "areas" | "capabilities" | "features" | "work_items";

type ProductManagementRefreshScope = {
  selectedProductId: string | null;
  productManagementTab: ProductManagementTab;
  selectedManagementStoryIdForTasks?: string | null;
};

type ProductPageRefreshScope = ProductManagementRefreshScope & {
  productPageTab: ProductPageTab;
  statusProductId: string;
  hideExampleProductsKey: string;
};

export function getProductManagementRefreshQueryKeys({
  selectedProductId,
  productManagementTab,
  selectedManagementStoryIdForTasks,
}: ProductManagementRefreshScope): QueryKey[] {
  if (!selectedProductId) {
    return [];
  }

  const queryKeys: QueryKey[] = [
    ["products"],
    ["productTree", selectedProductId],
    ["workItemScopeSummary", selectedProductId],
  ];

  if (productManagementTab === "work_items") {
    queryKeys.push(
      ["productTasks", selectedProductId],
      ["subWorkItems", selectedManagementStoryIdForTasks],
    );
  }

  return queryKeys;
}

export function getProductPageRefreshQueryKeys(scope: ProductPageRefreshScope): QueryKey[] {
  const {
    productPageTab,
    selectedProductId,
    statusProductId,
    hideExampleProductsKey,
  } = scope;

  switch (productPageTab) {
    case "list":
      return [
        ["products"],
        ["setting", hideExampleProductsKey],
        ["productTree"],
        ["productWorkItemSummary"],
      ];
    case "status":
      return [
        ["products"],
        ["productTree"],
        ["workItemScopeSummary", statusProductId === "all" ? "all" : statusProductId],
        ["productWorkItemSummary"],
      ];
    case "overview":
      if (!selectedProductId) {
        return [];
      }
      return [
        ["products"],
        ["productOverviewProductAreas", selectedProductId],
        ["productTreeSummary", selectedProductId],
        ["productWorkItemSummary"],
        ["productOverviewPageReferences"],
      ];
    case "design":
      return getProductManagementRefreshQueryKeys(scope);
    case "dependencies":
      if (!selectedProductId) {
        return [];
      }
      return [
        ["products"],
        ["productTree"],
        ["product-dependencies"],
      ];
  }
}

export function getProductPageRefreshLabel(productPageTab: ProductPageTab): string {
  switch (productPageTab) {
    case "list":
      return "Refresh List";
    case "status":
      return "Refresh Status";
    case "overview":
      return "Refresh Overview";
    case "design":
      return "Refresh Management";
    case "dependencies":
      return "Refresh Dependencies";
  }
}

export function getProductManagementRefreshLabel(productManagementTab: ProductManagementTab): string {
  switch (productManagementTab) {
    case "areas":
      return "Refresh Areas";
    case "capabilities":
      return "Refresh Capabilities";
    case "features":
      return "Refresh Features";
    case "work_items":
      return "Refresh Work Items";
  }
}

export function isProductPageRefreshDisabled(
  productPageTab: ProductPageTab,
  selectedProductId: string | null,
): boolean {
  return !selectedProductId && productPageTab !== "list" && productPageTab !== "status";
}
