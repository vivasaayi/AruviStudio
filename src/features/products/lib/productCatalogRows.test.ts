import { describe, expect, it } from "vitest";

import {
  buildProductCatalogRows,
  getProductCatalogTags,
  isExampleProduct,
} from "./productCatalogRows";
import type { Product, ProductTreeSummary, ProductWorkItemSummary } from "../../../lib/types";

function product(overrides: Partial<Product> = {}): Product {
  return {
    id: "product-1",
    name: "Mayyam",
    description: "Operations platform",
    vision: "Reliable operations",
    goals: [],
    tags: [],
    status: "active",
    lifecycle: "active",
    health: "healthy",
    owner_label: "SRE",
    investment_status: "invest",
    roadmap: "",
    evidence: "",
    created_at: "2026-01-01T00:00:00Z",
    updated_at: "2026-01-01T00:00:00Z",
    ...overrides,
  };
}

function treeSummary(productId: string, overrides: Partial<ProductTreeSummary> = {}): ProductTreeSummary {
  return {
    product_id: productId,
    product_area_count: 0,
    capability_count: 0,
    total_node_count: 0,
    leaf_node_count: 0,
    ...overrides,
  };
}

function summary(productId: string, overrides: Partial<ProductWorkItemSummary> = {}): ProductWorkItemSummary {
  return {
    product_id: productId,
    total_count: 0,
    active_count: 0,
    done_count: 0,
    blocked_count: 0,
    ...overrides,
  };
}

describe("productCatalogRows", () => {
  it("classifies example products and returns sorted catalog tags", () => {
    expect(isExampleProduct(product({ id: "example-product" }))).toBe(true);
    expect(isExampleProduct(product({ tags: ["seeded_catalog"] }))).toBe(true);
    expect(isExampleProduct(product({ tags: ["custom"] }))).toBe(false);
    expect(getProductCatalogTags([
      product({ tags: ["kubernetes", "sre"] }),
      product({ id: "product-2", tags: ["cloud", "sre"] }),
    ])).toEqual(["cloud", "kubernetes", "sre"]);
  });

  it("builds rows with hierarchy and aggregate work counts", () => {
    const mayyam = product();
    const rows = buildProductCatalogRows({
      products: [mayyam],
      productTreeSummaryById: new Map([[mayyam.id, treeSummary(mayyam.id, { product_area_count: 1, total_node_count: 2 })]]),
      productSummaryById: new Map([[mayyam.id, summary(mayyam.id, { total_count: 12, active_count: 7, done_count: 3 })]]),
      search: "",
      statusFilter: "all",
      sourceFilter: "all",
      tagFilter: "all",
      sort: "name",
      showDefaultProducts: true,
      showCustomProducts: true,
    });

    expect(rows).toMatchObject([
      {
        source: "custom",
        rootCount: 1,
        nodeCount: 2,
        workItemCount: 12,
        activeWorkItemCount: 7,
        progress: { total: 12, done: 3, percent: 25 },
      },
    ]);
  });

  it("filters by visibility, source, status, tag, and text search", () => {
    const products = [
      product({ id: "example-seed", name: "Seed", tags: ["seeded_catalog"], updated_at: "2026-01-01T00:00:00Z" }),
      product({ id: "custom-active", name: "Mayyam", tags: ["sre"], owner_label: "Platform", updated_at: "2026-01-02T00:00:00Z" }),
      product({ id: "custom-archived", name: "Legacy", status: "archived", tags: ["sre"], updated_at: "2026-01-03T00:00:00Z" }),
    ];

    expect(buildProductCatalogRows({
      products,
      productTreeSummaryById: new Map(),
      productSummaryById: new Map(),
      search: "platform",
      statusFilter: "active",
      sourceFilter: "custom",
      tagFilter: "sre",
      sort: "name",
      showDefaultProducts: false,
      showCustomProducts: true,
    }).map((row) => row.product.id)).toEqual(["custom-active"]);
  });

  it("sorts by updated date, progress, and work count", () => {
    const alpha = product({ id: "alpha", name: "Alpha", updated_at: "2026-01-01T00:00:00Z" });
    const beta = product({ id: "beta", name: "Beta", updated_at: "2026-01-03T00:00:00Z" });
    const summaries = new Map<string, ProductWorkItemSummary>([
      [alpha.id, summary(alpha.id, { total_count: 10, done_count: 8 })],
      [beta.id, summary(beta.id, { total_count: 20, done_count: 2 })],
    ]);
    const baseOptions = {
      products: [alpha, beta],
      productTreeSummaryById: new Map<string, ProductTreeSummary>(),
      productSummaryById: summaries,
      search: "",
      statusFilter: "all" as const,
      sourceFilter: "all" as const,
      tagFilter: "all",
      showDefaultProducts: true,
      showCustomProducts: true,
    };

    expect(buildProductCatalogRows({ ...baseOptions, sort: "updated" }).map((row) => row.product.id)).toEqual(["beta", "alpha"]);
    expect(buildProductCatalogRows({ ...baseOptions, sort: "progress" }).map((row) => row.product.id)).toEqual(["alpha", "beta"]);
    expect(buildProductCatalogRows({ ...baseOptions, sort: "work" }).map((row) => row.product.id)).toEqual(["beta", "alpha"]);
  });
});
