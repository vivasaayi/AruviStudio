import React, { useEffect } from "react";
import { useParams } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { getWorkItem } from "../../../lib/tauri";
import { WorkItemListPage } from "./WorkItemListPage";
import { useWorkspaceStore } from "../../../state/workspaceStore";

export function WorkItemDetailPage() {
  const { workItemId } = useParams<{ workItemId: string }>();
  const { setActiveProduct, setActiveProductArea, setActiveCapability, setActiveWorkItem } = useWorkspaceStore();

  const { data: workItem } = useQuery({
    queryKey: ["workItemRoute", workItemId],
    queryFn: () => getWorkItem(workItemId!),
    enabled: !!workItemId,
  });

  useEffect(() => {
    if (!workItem) {
      return;
    }
    setActiveProduct(workItem.product_id ?? null);
    setActiveProductArea(workItem.product_area_id ?? null);
    setActiveCapability(workItem.capability_id ?? null);
    setActiveWorkItem(workItem.id);
  }, [workItem, setActiveCapability, setActiveProductArea, setActiveProduct, setActiveWorkItem]);

  return <WorkItemListPage />;
}
