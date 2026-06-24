import type React from "react";

import { ScopeBreadcrumb } from "../../../app/layout/ScopeBreadcrumb";
import type { Capability, Product, ProductArea, WorkItem, WorkflowRun } from "../../../lib/types";
import {
  describeWorkItemRuntime,
  formatWorkItemTypeLabel,
  getToneBadgeStyle,
  moveTaskIdToIndex,
  statusColors,
} from "../lib/workItemListPageHelpers";
import { styles } from "../lib/workItemListPageStyles";

type BacklogWindow = {
  start: number;
  end: number;
  topPadding: number;
  bottomPadding: number;
  items: WorkItem[];
};

type WorkItemOwner = {
  badge: string;
  path: string;
  isRoot: boolean;
};

type BacklogApprovalAction = "approve" | "reject";

type WorkItemBacklogTabProps = {
  activeProduct: Product | null;
  activeProductArea: ProductArea | null;
  activeCapability: Capability | null;
  selectedBacklogItems: WorkItem[];
  bulkActionInFlight: BacklogApprovalAction | null;
  onRunBulkApprovalAction: (action: BacklogApprovalAction) => void;
  onOpenCreateStory: () => void;
  activeProductId: string | null;
  actionError: string | null;
  statusFilter: string;
  onStatusFilterChange: (status: string) => void;
  scopeDescriptor: string;
  workItemPageIndex: number;
  workItemPageStart: number;
  workItemPageEnd: number;
  backlogRenderedRangeLabel: string;
  onWorkItemPageIndexChange: React.Dispatch<React.SetStateAction<number>>;
  hasNextWorkItemPage: boolean;
  isLoading: boolean;
  orderedWorkItems: WorkItem[];
  backlogViewportRef: React.RefObject<HTMLDivElement | null>;
  onBacklogScrollTopChange: (scrollTop: number) => void;
  backlogWindow: BacklogWindow;
  latestWorkflowRunByWorkItemId: Map<string, WorkflowRun | null>;
  workItemOwnerMap: Map<string, WorkItemOwner>;
  selectedWorkItemId: string | null;
  draggedWorkItemId: string | null;
  onDraggedWorkItemIdChange: (workItemId: string | null) => void;
  workItemOrderIds: string[];
  onWorkItemOrderIdsChange: (orderedIds: string[]) => void;
  onReorderWorkItems: (orderedIds: string[]) => void;
  onSelectWorkItem: (workItemId: string) => void;
  selectedBacklogItemIds: string[];
  onSelectedBacklogItemIdsChange: React.Dispatch<React.SetStateAction<string[]>>;
  onRunRowApprovalAction: (workItemId: string, action: BacklogApprovalAction) => void;
  isRowActionPending: (workItemId: string) => boolean;
  openOverflowWorkItemId: string | null;
  onOpenOverflowWorkItemIdChange: React.Dispatch<React.SetStateAction<string | null>>;
  onDeleteWorkItem: (workItemId: string) => void;
};

export function WorkItemBacklogTab({
  activeProduct,
  activeProductArea,
  activeCapability,
  selectedBacklogItems,
  bulkActionInFlight,
  onRunBulkApprovalAction,
  onOpenCreateStory,
  activeProductId,
  actionError,
  statusFilter,
  onStatusFilterChange,
  scopeDescriptor,
  workItemPageIndex,
  workItemPageStart,
  workItemPageEnd,
  backlogRenderedRangeLabel,
  onWorkItemPageIndexChange,
  hasNextWorkItemPage,
  isLoading,
  orderedWorkItems,
  backlogViewportRef,
  onBacklogScrollTopChange,
  backlogWindow,
  latestWorkflowRunByWorkItemId,
  workItemOwnerMap,
  selectedWorkItemId,
  draggedWorkItemId,
  onDraggedWorkItemIdChange,
  workItemOrderIds,
  onWorkItemOrderIdsChange,
  onReorderWorkItems,
  onSelectWorkItem,
  selectedBacklogItemIds,
  onSelectedBacklogItemIdsChange,
  onRunRowApprovalAction,
  isRowActionPending,
  openOverflowWorkItemId,
  onOpenOverflowWorkItemIdChange,
  onDeleteWorkItem,
}: WorkItemBacklogTabProps) {
  return (
    <>
      <ScopeBreadcrumb
        label="Current Scope"
        productName={activeProduct?.name}
        productAreaName={activeProductArea?.name}
        capabilityName={activeCapability?.name}
      />
      <div style={styles.sectionTitle}>
        <span>Backlog</span>
        <div style={{ display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
          {selectedBacklogItems.length > 0 ? (
            <>
              <button
                style={{ ...styles.btn, backgroundColor: "#2d6a3f" }}
                onClick={() => onRunBulkApprovalAction("approve")}
                disabled={bulkActionInFlight !== null}
              >
                {bulkActionInFlight === "approve" ? "Approving..." : `Approve Selected (${selectedBacklogItems.length})`}
              </button>
              <button
                style={styles.btnDanger}
                onClick={() => onRunBulkApprovalAction("reject")}
                disabled={bulkActionInFlight !== null}
              >
                {bulkActionInFlight === "reject" ? "Rejecting..." : `Reject Selected (${selectedBacklogItems.length})`}
              </button>
            </>
          ) : null}
          <button style={styles.ghostBtn} onClick={onOpenCreateStory}>
            + New Story
          </button>
        </div>
      </div>
      {!activeProductId && <div style={styles.warning}>Select a product to load the backlog.</div>}
      {actionError && <div style={styles.errorText}>{actionError}</div>}
      <select style={styles.filterSelect} value={statusFilter} onChange={(event) => onStatusFilterChange(event.target.value)}>
        <option value="">All Statuses</option>
        {Object.keys(statusColors).map((status) => (
          <option key={status} value={status}>{status.replace(/_/g, " ")}</option>
        ))}
      </select>
      <div style={styles.smallText}>
        Showing stories for: {scopeDescriptor}. Page {workItemPageIndex + 1}, rows {workItemPageStart}-{workItemPageEnd}. Rendering visible rows {backlogRenderedRangeLabel}.
      </div>
      <div style={{ display: "flex", gap: 8, alignItems: "center", marginBottom: 10 }}>
        <button
          style={styles.ghostBtn}
          onClick={() => onWorkItemPageIndexChange((current) => Math.max(0, current - 1))}
          disabled={workItemPageIndex === 0 || isLoading}
        >
          Previous
        </button>
        <button
          style={styles.ghostBtn}
          onClick={() => onWorkItemPageIndexChange((current) => current + 1)}
          disabled={!hasNextWorkItemPage || isLoading}
        >
          Next
        </button>
        <span style={styles.smallText}>100 rows per page. Use status/scope filters to narrow large products.</span>
      </div>
      {isLoading ? (
        <div style={styles.empty}>Loading stories...</div>
      ) : orderedWorkItems.length > 0 ? (
        <div
          data-testid="work-item-virtual-list"
          ref={backlogViewportRef}
          style={styles.virtualTaskViewport}
          onScroll={(event) => onBacklogScrollTopChange(event.currentTarget.scrollTop)}
        >
          <div style={{ ...styles.virtualTaskSpacer, paddingTop: backlogWindow.topPadding, paddingBottom: backlogWindow.bottomPadding }}>
            {backlogWindow.items.map((workItem, visibleWorkItemIndex) => {
              const workItemIndex = backlogWindow.start + visibleWorkItemIndex;
              const latestRun = latestWorkflowRunByWorkItemId.get(workItem.id) ?? null;
              const runtimeStatus = describeWorkItemRuntime(workItem, latestRun);
              const owner = workItemOwnerMap.get(workItem.id);

              return (
                <div
                  key={workItem.id}
                  style={{
                    ...(selectedWorkItemId === workItem.id ? styles.taskCardActive : styles.taskCard),
                    ...(draggedWorkItemId === workItem.id ? styles.dropTarget : null),
                  }}
                  draggable
                  onDragStart={() => onDraggedWorkItemIdChange(workItem.id)}
                  onDragEnd={() => onDraggedWorkItemIdChange(null)}
                  onDragOver={(event) => event.preventDefault()}
                  onDrop={() => {
                    if (!draggedWorkItemId || draggedWorkItemId === workItem.id) {
                      return;
                    }
                    const nextOrder = moveTaskIdToIndex(workItemOrderIds, draggedWorkItemId, workItemIndex);
                    onWorkItemOrderIdsChange(nextOrder);
                    onReorderWorkItems(nextOrder);
                    onDraggedWorkItemIdChange(null);
                  }}
                  onClick={() => onSelectWorkItem(workItem.id)}
                >
                  <div style={styles.taskRowCard}>
                    <div style={styles.taskMain}>
                      <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
                        <input
                          type="checkbox"
                          checked={selectedBacklogItemIds.includes(workItem.id)}
                          onChange={(event) => {
                            event.stopPropagation();
                            onSelectedBacklogItemIdsChange((current) =>
                              event.target.checked
                                ? [...current, workItem.id]
                                : current.filter((id) => id !== workItem.id),
                            );
                          }}
                          onClick={(event) => event.stopPropagation()}
                        />
                        <div style={styles.taskTitle}>{workItem.title}</div>
                      </div>
                      {owner ? (
                        <div style={styles.ownerRow}>
                          <span style={owner.isRoot ? styles.ownerBadgeRoot : styles.ownerBadge}>
                            Owner: {owner.badge}
                          </span>
                          <span style={styles.ownerPath}>{owner.path}</span>
                        </div>
                      ) : null}
                      <div style={styles.taskMeta}>
                        <span>{formatWorkItemTypeLabel(workItem.work_item_type)}</span>
                        <span>{workItem.priority}</span>
                        {runtimeStatus.stageLabel ? <span>stage: {runtimeStatus.stageLabel}</span> : null}
                      </div>
                    </div>
                    <div style={styles.taskStatusLine}>
                      <div style={styles.badgeRow}>
                        <span style={{ ...styles.badge, backgroundColor: statusColors[workItem.status] || "#444", color: "#fff" }}>
                          {workItem.status.replace(/_/g, " ")}
                        </span>
                        {runtimeStatus.label !== workItem.status.replace(/_/g, " ") ? (
                          <span style={{ ...styles.badge, ...getToneBadgeStyle(runtimeStatus.tone) }}>
                            {runtimeStatus.label}
                          </span>
                        ) : null}
                      </div>
                      <div style={styles.taskStatusSummary}>{runtimeStatus.detail}</div>
                    </div>
                    <div style={styles.taskActions}>
                      <button
                        style={workItem.status === "approved" ? styles.ghostBtn : { ...styles.btn, backgroundColor: "#2d6a3f" }}
                        onClick={(event) => {
                          event.stopPropagation();
                          onRunRowApprovalAction(workItem.id, "approve");
                        }}
                        disabled={isRowActionPending(workItem.id)}
                      >
                        {isRowActionPending(workItem.id) ? "Working..." : workItem.status === "approved" ? "Approved" : "Approve"}
                      </button>
                      <button
                        style={styles.btnDanger}
                        onClick={(event) => {
                          event.stopPropagation();
                          onRunRowApprovalAction(workItem.id, "reject");
                        }}
                        disabled={isRowActionPending(workItem.id)}
                      >
                        {isRowActionPending(workItem.id) ? "Working..." : "Reject"}
                      </button>
                      <span style={styles.dragHandle} title="Drag to reorder">::</span>
                      <div style={styles.overflowWrap}>
                        <button
                          style={styles.ghostBtn}
                          onClick={(event) => {
                            event.stopPropagation();
                            onOpenOverflowWorkItemIdChange((current) => (current === workItem.id ? null : workItem.id));
                          }}
                          aria-label={`More actions for ${workItem.title}`}
                        >
                          ...
                        </button>
                        {openOverflowWorkItemId === workItem.id && (
                          <div style={styles.overflowMenu} onClick={(event) => event.stopPropagation()}>
                            <button
                              style={styles.overflowMenuItem}
                              onClick={() => {
                                onSelectWorkItem(workItem.id);
                                onOpenOverflowWorkItemIdChange(null);
                              }}
                            >
                              Open details
                            </button>
                            <button
                              style={styles.overflowMenuItemDanger}
                              onClick={() => {
                                onOpenOverflowWorkItemIdChange(null);
                                onDeleteWorkItem(workItem.id);
                              }}
                            >
                              Delete story
                            </button>
                          </div>
                        )}
                      </div>
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      ) : (
        <div style={styles.empty}>No stories in the current scope yet.</div>
      )}
    </>
  );
}
