import { ProductListPageBody } from "../components/ProductListPageBody";
import { useProductListPageController } from "../hooks/useProductListPageController";

export function ProductListPage() {
  const controller = useProductListPageController();

  return <ProductListPageBody controller={controller} />;
}
