import React, { useEffect } from "react";
import { useParams } from "react-router-dom";
import { ProductListPage } from "./ProductListPage";
import { useWorkspaceStore } from "../../../state/workspaceStore";

export function ProductDetailPage() {
  const { productId } = useParams<{ productId: string }>();
  const { setActiveProduct } = useWorkspaceStore();

  useEffect(() => {
    if (productId) {
      setActiveProduct(productId);
    }
  }, [productId, setActiveProduct]);

  return <ProductListPage />;
}
