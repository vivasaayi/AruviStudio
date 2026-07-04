import { useMemo } from "react";
import { useQueries } from "@tanstack/react-query";

import { getLatestWorkflowRunForWorkItem } from "../../../lib/tauri";
import type { WorkItem, WorkItemPage, WorkflowRun } from "../../../lib/types";
import {
  BACKLOG_OVERSCAN_ROWS,
  BACKLOG_ROW_ESTIMATED_HEIGHT,
  WORK_ITEM_PAGE_SIZE,
  orderWorkItemsByIds,
} from "../lib/workItemListPageHelpers";

type WorkItemBacklogViewInput = {
  filteredWorkItems: WorkItem[];
  workItemOrderIds: string[];
  backlogScrollTop: number;
  backlogViewportHeight: number;
  workItemPage: WorkItemPage | undefined;
  workItemPageIndex: number;
  selectedBacklogItemIds: string[];
  isBacklogActive: boolean;
};

export function useWorkItemBacklogView({
  filteredWorkItems,
  workItemOrderIds,
  backlogScrollTop,
  backlogViewportHeight,
  workItemPage,
  workItemPageIndex,
  selectedBacklogItemIds,
  isBacklogActive,
}: WorkItemBacklogViewInput) {
  const orderedWorkItems = useMemo(() => orderWorkItemsByIds(filteredWorkItems, workItemOrderIds), [filteredWorkItems, workItemOrderIds]);
  const backlogWindow = useMemo(() => {
    const start = Math.max(0, Math.floor(backlogScrollTop / BACKLOG_ROW_ESTIMATED_HEIGHT) - BACKLOG_OVERSCAN_ROWS);
    const visibleRows = Math.ceil(backlogViewportHeight / BACKLOG_ROW_ESTIMATED_HEIGHT) + BACKLOG_OVERSCAN_ROWS * 2;
    const end = Math.min(orderedWorkItems.length, start + Math.max(visibleRows, BACKLOG_OVERSCAN_ROWS * 2));
    return {
      start,
      end,
      topPadding: start * BACKLOG_ROW_ESTIMATED_HEIGHT,
      bottomPadding: Math.max(0, orderedWorkItems.length - end) * BACKLOG_ROW_ESTIMATED_HEIGHT,
      items: orderedWorkItems.slice(start, end),
    };
  }, [backlogScrollTop, backlogViewportHeight, orderedWorkItems]);
  const hasNextWorkItemPage = workItemPage?.has_more ?? false;
  const workItemPageStart = workItemPageIndex * WORK_ITEM_PAGE_SIZE + (orderedWorkItems.length > 0 ? 1 : 0);
  const workItemPageEnd = workItemPageIndex * WORK_ITEM_PAGE_SIZE + orderedWorkItems.length;
  const backlogRenderedRangeLabel =
    orderedWorkItems.length > 0 ? `${backlogWindow.start + 1}-${backlogWindow.end}` : "0-0";
  const selectedBacklogItems = useMemo(
    () => orderedWorkItems.filter((workItem) => selectedBacklogItemIds.includes(workItem.id)),
    [orderedWorkItems, selectedBacklogItemIds],
  );
  const backlogWorkflowRunQueries = useQueries({
    queries: backlogWindow.items.map((workItem) => ({
      queryKey: ["latestWorkflowRun", workItem.id],
      queryFn: () => getLatestWorkflowRunForWorkItem(workItem.id),
      enabled: isBacklogActive,
      refetchInterval: 4000,
    })),
  });
  const latestWorkflowRunByWorkItemId = useMemo(() => {
    const map = new Map<string, WorkflowRun | null>();
    backlogWindow.items.forEach((workItem, index) => {
      const run = backlogWorkflowRunQueries[index]?.data ?? null;
      map.set(workItem.id, run);
    });
    return map;
  }, [backlogWorkflowRunQueries, backlogWindow.items]);

  return {
    orderedWorkItems,
    backlogWindow,
    hasNextWorkItemPage,
    workItemPageStart,
    workItemPageEnd,
    backlogRenderedRangeLabel,
    selectedBacklogItems,
    latestWorkflowRunByWorkItemId,
  };
}
