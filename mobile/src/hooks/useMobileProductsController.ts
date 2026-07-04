import { useState } from "react";
import { PlannerMobileClient } from "../api/client";
import { productViewNeedsTree } from "../lib/productTree";
import type { Product, ProductExploreTab, ProductTree, ProductTreeSummary } from "../types";
import { useMobileProductViewModel } from "./useMobileProductViewModel";

type MobileProductsControllerInput = {
  mobileClient: PlannerMobileClient;
  token: string;
  describeError: (error: unknown) => string;
};

export function useMobileProductsController({
  mobileClient,
  token,
  describeError,
}: MobileProductsControllerInput) {
  const [products, setProducts] = useState<Product[]>([]);
  const [productSummary, setProductSummary] = useState<ProductTreeSummary | null>(null);
  const [productTree, setProductTree] = useState<ProductTree | null>(null);
  const [selectedProductId, setSelectedProductId] = useState<string | null>(null);
  const [selectedProductNodeId, setSelectedProductNodeId] = useState<string | null>(null);
  const [productExploreTab, setProductExploreTab] = useState<ProductExploreTab>("overview");
  const [productSearchQuery, setProductSearchQuery] = useState("");
  const [isProductLoading, setIsProductLoading] = useState(false);
  const [isProductTreeLoading, setIsProductTreeLoading] = useState(false);
  const [productError, setProductError] = useState<string | null>(null);
  const [isProductPickerOpen, setIsProductPickerOpen] = useState(false);

  const viewModel = useMobileProductViewModel({
    products,
    productSummary,
    productTree,
    selectedProductId,
    selectedProductNodeId,
    productSearchQuery,
  });

  const loadProductSummary = async (productId: string) => {
    const summary = await mobileClient.getProductSummary(productId);
    setProductSummary(summary);
    setSelectedProductId(productId);
    if (productTree?.product.id !== productId || !productViewNeedsTree(productExploreTab)) {
      setProductTree(null);
      setSelectedProductNodeId(null);
    }
  };

  const ensureProductTree = async (productId: string, force = false) => {
    if (!force && productTree?.product.id === productId) {
      return productTree;
    }
    try {
      setIsProductTreeLoading(true);
      setProductError(null);
      const previousProductId = selectedProductId;
      const tree = await mobileClient.getProductTree(productId);
      setProductTree(tree);
      setSelectedProductId(productId);
      if (previousProductId !== productId) {
        setSelectedProductNodeId(null);
      }
      return tree;
    } finally {
      setIsProductTreeLoading(false);
    }
  };

  const switchProductExploreTab = async (mode: ProductExploreTab) => {
    setProductExploreTab(mode);
    if (!productViewNeedsTree(mode) || !selectedProductId) {
      return;
    }
    try {
      await ensureProductTree(selectedProductId);
    } catch (error) {
      setProductError(describeError(error));
    }
  };

  const loadProducts = async (preferredProductId?: string | null) => {
    if (!token.trim()) {
      setProductError("Save a mobile API token before loading products.");
      return;
    }
    try {
      setIsProductLoading(true);
      setProductError(null);
      const loadedProducts = await mobileClient.listProducts();
      setProducts(loadedProducts);
      const nextProductId =
        preferredProductId && loadedProducts.some((product) => product.id === preferredProductId)
          ? preferredProductId
          : selectedProductId && loadedProducts.some((product) => product.id === selectedProductId)
            ? selectedProductId
            : loadedProducts[0]?.id ?? null;
      if (nextProductId) {
        await loadProductSummary(nextProductId);
        if (productViewNeedsTree(productExploreTab)) {
          await ensureProductTree(nextProductId, true);
        }
      } else {
        setProductSummary(null);
        setProductTree(null);
        setSelectedProductId(null);
        setSelectedProductNodeId(null);
      }
    } catch (error) {
      setProductError(describeError(error));
    } finally {
      setIsProductLoading(false);
    }
  };

  const openProductNode = (nodeId: string) => {
    setSelectedProductNodeId(nodeId);
    setProductExploreTab("map");
  };

  return {
    ...viewModel,
    products,
    productSummary,
    productTree,
    selectedProductId,
    selectedProductNodeId,
    productExploreTab,
    productSearchQuery,
    isProductLoading,
    isProductTreeLoading,
    productError,
    isProductPickerOpen,
    loadProducts,
    ensureProductTree,
    setProductError,
    openProductNode,
    setSelectedProductNodeId,
    switchProductExploreTab,
    setProductSearchQuery,
    setIsProductPickerOpen,
  };
}
