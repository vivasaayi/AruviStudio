import { useMemo } from "react";
import type { Product, ProductTree, ProductTreeSummary } from "../types";
import {
  countLeafNodes,
  countTreeNodes,
  findProductNode,
  findProductNodePath,
  flattenProductNodes,
} from "../lib/productTree";

type MobileProductViewModelInput = {
  products: Product[];
  productSummary: ProductTreeSummary | null;
  productTree: ProductTree | null;
  selectedProductId: string | null;
  selectedProductNodeId: string | null;
  productSearchQuery: string;
};

export function useMobileProductViewModel({
  products,
  productSummary,
  productTree,
  selectedProductId,
  selectedProductNodeId,
  productSearchQuery,
}: MobileProductViewModelInput) {
  const selectedProduct = useMemo(() => {
    return products.find((product) => product.id === selectedProductId) ?? productTree?.product ?? products[0] ?? null;
  }, [productTree?.product, products, selectedProductId]);

  const selectedProductNode = useMemo(() => {
    return findProductNode(productTree?.roots ?? [], selectedProductNodeId);
  }, [productTree?.roots, selectedProductNodeId]);

  const selectedProductNodePath = useMemo(() => {
    return findProductNodePath(productTree?.roots ?? [], selectedProductNodeId);
  }, [productTree?.roots, selectedProductNodeId]);

  const productFlatNodes = useMemo(() => {
    return flattenProductNodes(productTree?.roots ?? []);
  }, [productTree?.roots]);

  const productStats = useMemo(() => {
    const roots = productTree?.roots ?? [];
    return {
      productAreas: productSummary?.product_area_count ?? productTree?.product_areas.length ?? 0,
      capabilities: productSummary?.capability_count ?? Math.max(countTreeNodes(roots) - (productTree?.product_areas.length ?? 0), 0),
      totalNodes: productSummary?.total_node_count ?? countTreeNodes(roots),
      leafNodes: productSummary?.leaf_node_count ?? countLeafNodes(roots),
    };
  }, [productSummary, productTree?.product_areas.length, productTree?.roots]);

  const visibleProductChildren = selectedProductNode?.children ?? productTree?.roots ?? [];

  const filteredProductNodes = useMemo(() => {
    const query = productSearchQuery.trim().toLowerCase();
    if (!query) return productFlatNodes;
    return productFlatNodes.filter(({ node, pathLabel }) => {
      return [
        node.name,
        node.summary,
        node.description,
        node.node_kind,
        node.node_type,
        pathLabel,
      ]
        .filter(Boolean)
        .some((value) => String(value).toLowerCase().includes(query));
    });
  }, [productFlatNodes, productSearchQuery]);

  return {
    selectedProduct,
    selectedProductNode,
    selectedProductNodePath,
    productStats,
    visibleProductChildren,
    filteredProductNodes,
  };
}
