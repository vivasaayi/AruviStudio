import { countHierarchyNodes } from "../../../lib/hierarchyTree";
import type { Product, ProductTree, ProductWorkItemSummary } from "../../../lib/types";
import { getProgressSummaryFromCounts, type ProgressSummary } from "./productStatusSummary";

export type ProductCatalogSource = "default" | "custom";
export type ProductCatalogStatusFilter = "all" | Product["status"];
export type ProductCatalogSourceFilter = "all" | ProductCatalogSource;
export type ProductCatalogSort = "name" | "updated" | "progress" | "work";

export interface ProductCatalogRow {
  product: Product;
  source: ProductCatalogSource;
  rootCount: number;
  nodeCount: number;
  workItemCount: number;
  activeWorkItemCount: number;
  progress: ProgressSummary;
}

export interface BuildProductCatalogRowsOptions {
  products: Product[];
  productTreeById: Map<string, ProductTree>;
  productSummaryById: Map<string, ProductWorkItemSummary>;
  search: string;
  statusFilter: ProductCatalogStatusFilter;
  sourceFilter: ProductCatalogSourceFilter;
  tagFilter: string;
  sort: ProductCatalogSort;
  showDefaultProducts: boolean;
  showCustomProducts: boolean;
}

export function getProductCatalogTags(products: Product[]) {
  const tags = new Set<string>();
  products.forEach((product) => product.tags.forEach((tag) => tags.add(tag)));
  return Array.from(tags).sort((a, b) => a.localeCompare(b));
}

export function buildProductCatalogRows({
  products,
  productTreeById,
  productSummaryById,
  search,
  statusFilter,
  sourceFilter,
  tagFilter,
  sort,
  showDefaultProducts,
  showCustomProducts,
}: BuildProductCatalogRowsOptions): ProductCatalogRow[] {
  const normalizedSearch = search.trim().toLowerCase();
  return products
    .map((product) => buildProductCatalogRow(product, productTreeById, productSummaryById))
    .filter((row) => productCatalogRowMatches(row, {
      search: normalizedSearch,
      statusFilter,
      sourceFilter,
      tagFilter,
      showDefaultProducts,
      showCustomProducts,
    }))
    .sort((a, b) => compareProductCatalogRows(a, b, sort));
}

export function isExampleProduct(product: Product) {
  return product.id.startsWith("example-") || product.tags.includes("example_product") || product.tags.includes("seeded_catalog");
}

function buildProductCatalogRow(
  product: Product,
  productTreeById: Map<string, ProductTree>,
  productSummaryById: Map<string, ProductWorkItemSummary>,
): ProductCatalogRow {
  const treeForProduct = productTreeById.get(product.id);
  const summary = productSummaryById.get(product.id);
  const total = summary?.total_count ?? 0;
  const done = summary?.done_count ?? 0;

  return {
    product,
    source: isExampleProduct(product) ? "default" : "custom",
    rootCount: treeForProduct?.roots.length ?? 0,
    nodeCount: treeForProduct ? countHierarchyNodes(treeForProduct.roots) : 0,
    workItemCount: total,
    activeWorkItemCount: summary?.active_count ?? 0,
    progress: getProgressSummaryFromCounts(total, done),
  };
}

function productCatalogRowMatches(
  row: ProductCatalogRow,
  filters: {
    search: string;
    statusFilter: ProductCatalogStatusFilter;
    sourceFilter: ProductCatalogSourceFilter;
    tagFilter: string;
    showDefaultProducts: boolean;
    showCustomProducts: boolean;
  },
) {
  if (!filters.showDefaultProducts && row.source === "default") return false;
  if (!filters.showCustomProducts && row.source === "custom") return false;
  if (filters.statusFilter !== "all" && row.product.status !== filters.statusFilter) return false;
  if (filters.sourceFilter !== "all" && row.source !== filters.sourceFilter) return false;
  if (filters.tagFilter !== "all" && !row.product.tags.includes(filters.tagFilter)) return false;
  if (!filters.search) return true;

  return [
    row.product.name,
    row.product.description,
    row.product.vision,
    row.product.status,
    row.product.lifecycle,
    row.product.health,
    row.product.owner_label,
    row.product.investment_status,
    row.product.roadmap,
    row.product.evidence,
    row.source,
    ...row.product.tags,
  ].join(" ").toLowerCase().includes(filters.search);
}

function compareProductCatalogRows(
  a: ProductCatalogRow,
  b: ProductCatalogRow,
  sort: ProductCatalogSort,
) {
  switch (sort) {
    case "updated":
      return Date.parse(b.product.updated_at) - Date.parse(a.product.updated_at);
    case "progress":
      return b.progress.percent - a.progress.percent || a.product.name.localeCompare(b.product.name);
    case "work":
      return b.workItemCount - a.workItemCount || a.product.name.localeCompare(b.product.name);
    case "name":
    default:
      return a.product.name.localeCompare(b.product.name);
  }
}
