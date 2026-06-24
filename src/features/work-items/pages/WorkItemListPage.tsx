import { WorkItemListPageBody } from "../components/WorkItemListPageBody";
import { useWorkItemListPageController } from "../hooks/useWorkItemListPageController";

export function WorkItemListPage() {
  const controller = useWorkItemListPageController();

  return <WorkItemListPageBody controller={controller} />;
}
