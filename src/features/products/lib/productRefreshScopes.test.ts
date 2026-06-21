import { describe, expect, it } from "vitest";

import {
  getProductManagementRefreshLabel,
  getProductManagementRefreshQueryKeys,
  getProductPageRefreshLabel,
  getProductPageRefreshQueryKeys,
  isProductPageRefreshDisabled,
} from "./productRefreshScopes";

describe("productRefreshScopes", () => {
  it("keeps overview refresh scoped to aggregate and lazy overview queries", () => {
    const queryKeys = getProductPageRefreshQueryKeys({
      productPageTab: "overview",
      selectedProductId: "product-1",
      statusProductId: "all",
      hideExampleProductsKey: "catalog.hide_example_products",
      productManagementTab: "areas",
      selectedManagementStoryIdForTasks: null,
    });

    expect(queryKeys).toEqual([
      ["products"],
      ["productOverviewProductAreas", "product-1"],
      ["productTreeSummary", "product-1"],
      ["productWorkItemSummary"],
      ["productOverviewPageReferences"],
    ]);
    expect(queryKeys).not.toContainEqual(["productTree"]);
    expect(queryKeys).not.toContainEqual(["productTree", "product-1"]);
    expect(queryKeys).not.toContainEqual(["productTasks", "product-1"]);
    expect(queryKeys).not.toContainEqual(["subWorkItems", expect.anything()]);
  });

  it("builds list, status, and dependency refresh scopes", () => {
    const baseScope = {
      selectedProductId: "product-1",
      statusProductId: "all",
      hideExampleProductsKey: "catalog.hide_example_products",
      productManagementTab: "areas" as const,
      selectedManagementStoryIdForTasks: null,
    };

    expect(getProductPageRefreshQueryKeys({
      ...baseScope,
      productPageTab: "list",
    })).toEqual([
      ["products"],
      ["setting", "catalog.hide_example_products"],
      ["productTree"],
      ["productWorkItemSummary"],
    ]);

    expect(getProductPageRefreshQueryKeys({
      ...baseScope,
      productPageTab: "status",
      statusProductId: "product-2",
    })).toEqual([
      ["products"],
      ["productTree"],
      ["workItemScopeSummary", "product-2"],
      ["productWorkItemSummary"],
    ]);

    expect(getProductPageRefreshQueryKeys({
      ...baseScope,
      productPageTab: "dependencies",
    })).toEqual([
      ["products"],
      ["productTree"],
      ["product-dependencies"],
    ]);
  });

  it("returns no scoped selected-product queries when product context is missing", () => {
    const baseScope = {
      selectedProductId: null,
      statusProductId: "all",
      hideExampleProductsKey: "catalog.hide_example_products",
      productManagementTab: "areas" as const,
      selectedManagementStoryIdForTasks: null,
    };

    expect(getProductPageRefreshQueryKeys({
      ...baseScope,
      productPageTab: "overview",
    })).toEqual([]);
    expect(getProductPageRefreshQueryKeys({
      ...baseScope,
      productPageTab: "dependencies",
    })).toEqual([]);
    expect(getProductManagementRefreshQueryKeys({
      selectedProductId: null,
      productManagementTab: "work_items",
      selectedManagementStoryIdForTasks: "story-1",
    })).toEqual([]);
  });

  it("adds work item query scopes only for the management work-items tab", () => {
    expect(getProductManagementRefreshQueryKeys({
      selectedProductId: "product-1",
      productManagementTab: "areas",
      selectedManagementStoryIdForTasks: "story-1",
    })).toEqual([
      ["products"],
      ["productTree", "product-1"],
      ["workItemScopeSummary", "product-1"],
    ]);

    expect(getProductManagementRefreshQueryKeys({
      selectedProductId: "product-1",
      productManagementTab: "work_items",
      selectedManagementStoryIdForTasks: "story-1",
    })).toEqual([
      ["products"],
      ["productTree", "product-1"],
      ["workItemScopeSummary", "product-1"],
      ["productTasks", "product-1"],
      ["subWorkItems", "story-1"],
    ]);
  });

  it("maps labels and disabled state consistently", () => {
    expect(getProductPageRefreshLabel("list")).toBe("Refresh List");
    expect(getProductPageRefreshLabel("status")).toBe("Refresh Status");
    expect(getProductPageRefreshLabel("overview")).toBe("Refresh Overview");
    expect(getProductPageRefreshLabel("design")).toBe("Refresh Management");
    expect(getProductPageRefreshLabel("dependencies")).toBe("Refresh Dependencies");
    expect(getProductManagementRefreshLabel("areas")).toBe("Refresh Areas");
    expect(getProductManagementRefreshLabel("capabilities")).toBe("Refresh Capabilities");
    expect(getProductManagementRefreshLabel("features")).toBe("Refresh Features");
    expect(getProductManagementRefreshLabel("work_items")).toBe("Refresh Work Items");
    expect(isProductPageRefreshDisabled("overview", null)).toBe(true);
    expect(isProductPageRefreshDisabled("list", null)).toBe(false);
    expect(isProductPageRefreshDisabled("status", null)).toBe(false);
    expect(isProductPageRefreshDisabled("dependencies", "product-1")).toBe(false);
  });
});
