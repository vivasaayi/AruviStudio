import { PlannerPageBody } from "../components/PlannerPageBody";
import { usePlannerPageController } from "../hooks/usePlannerPageController";

export function PlannerPage() {
  const controller = usePlannerPageController();

  return <PlannerPageBody controller={controller} />;
}
