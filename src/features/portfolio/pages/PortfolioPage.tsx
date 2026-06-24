import { PortfolioPageBody } from "../components/PortfolioPageBody";
import { usePortfolioPageController } from "../hooks/usePortfolioPageController";

export function PortfolioPage() {
  const controller = usePortfolioPageController();

  return <PortfolioPageBody controller={controller} />;
}
