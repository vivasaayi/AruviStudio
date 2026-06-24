import { getHierarchyNodeSectionId } from "../../../lib/hierarchyTree";
import type { Capability, CapabilityTree, HierarchyTreeNode, ProductArea, ProductAreaTree, ProductTree, WorkItem } from "../../../lib/types";

export const PRODUCT_OVERVIEW_TOP_ID = "product-overview-top";
export const PRODUCT_DELIVERY_ID = "product-delivery";

export type WorkItemNode = {
  workItem: WorkItem;
  children: WorkItemNode[];
};

export type WorkItemMetrics = {
  total: number;
  done: number;
  wip: number;
  tbd: number;
  blocked: number;
  completion: number;
};

export type ProductOverviewTocItem = {
  id: string;
  title: string;
  level: number;
};

export type WorkItemPresentation = {
  bucket: "done" | "wip" | "tbd" | "blocked";
  label: "Done" | "WIP" | "TBD" | "Blocked";
  toneClass: "is-done" | "is-wip" | "is-tbd" | "is-blocked";
  accentColor: string;
  borderColor: string;
  backgroundColor: string;
  badgeBackground: string;
  badgeColor: string;
};

export function sortWorkItems(workItems: WorkItem[]) {
  return [...workItems].sort(
    (left, right) =>
      left.sort_order - right.sort_order
      || left.title.localeCompare(right.title)
      || left.created_at.localeCompare(right.created_at),
  );
}

export function buildScopedWorkItemTree(workItems: WorkItem[]): WorkItemNode[] {
  if (workItems.length === 0) {
    return [];
  }

  const sortedItems = sortWorkItems(workItems);
  const itemIds = new Set(sortedItems.map((workItem) => workItem.id));
  const childrenByParent = new Map<string, WorkItem[]>();

  sortedItems.forEach((workItem) => {
    if (workItem.parent_work_item_id && itemIds.has(workItem.parent_work_item_id)) {
      const siblings = childrenByParent.get(workItem.parent_work_item_id) ?? [];
      siblings.push(workItem);
      childrenByParent.set(workItem.parent_work_item_id, siblings);
    }
  });

  const roots = sortedItems.filter(
    (workItem) => !workItem.parent_work_item_id || !itemIds.has(workItem.parent_work_item_id),
  );

  const materialize = (workItem: WorkItem): WorkItemNode => ({
    workItem,
    children: (childrenByParent.get(workItem.id) ?? []).map(materialize),
  });

  return roots.map(materialize);
}

export function countCapabilities(product_areas: ProductAreaTree[]) {
  return product_areas.reduce((total, productAreaTree) => total + productAreaTree.features.reduce((sum, capabilityTree) => sum + countCapabilityTree(capabilityTree), 0), 0);
}

export function countCapabilityTree(capabilityTree: CapabilityTree): number {
  return 1 + capabilityTree.children.reduce((sum, child) => sum + countCapabilityTree(child), 0);
}

export function buildWorkItemMetrics(workItems: WorkItem[]): WorkItemMetrics {
  const totals = workItems.reduce(
    (accumulator, workItem) => {
      const bucket = getWorkItemPresentation(workItem.status).bucket;
      if (bucket === "done") accumulator.done += 1;
      else if (bucket === "wip") accumulator.wip += 1;
      else if (bucket === "blocked") accumulator.blocked += 1;
      else accumulator.tbd += 1;
      return accumulator;
    },
    { done: 0, wip: 0, tbd: 0, blocked: 0 },
  );

  const total = totals.done + totals.wip + totals.tbd + totals.blocked;
  const completion = total === 0 ? 0 : Math.round((totals.done / total) * 100);

  return { ...totals, total, completion };
}

export function getWorkItemPresentation(status: WorkItem["status"]): WorkItemPresentation {
  switch (status) {
    case "done":
      return {
        bucket: "done",
        label: "Done",
        toneClass: "is-done",
        accentColor: "#4aa37c",
        borderColor: "#335d4c",
        backgroundColor: "#121d18",
        badgeBackground: "#1d4737",
        badgeColor: "#a8f4d0",
      };
    case "in_progress":
    case "in_planning":
    case "in_validation":
    case "waiting_human_review":
    case "ready_for_review":
      return {
        bucket: "wip",
        label: "WIP",
        toneClass: "is-wip",
        accentColor: "#d1a643",
        borderColor: "#655533",
        backgroundColor: "#221c10",
        badgeBackground: "#5c4818",
        badgeColor: "#ffe8a8",
      };
    case "blocked":
    case "failed":
      return {
        bucket: "blocked",
        label: "Blocked",
        toneClass: "is-blocked",
        accentColor: "#cb6469",
        borderColor: "#6c373b",
        backgroundColor: "#241315",
        badgeBackground: "#722b31",
        badgeColor: "#ffbfc3",
      };
    case "cancelled":
    case "approved":
    case "draft":
    default:
      return {
        bucket: "tbd",
        label: "TBD",
        toneClass: "is-tbd",
        accentColor: "#6797d8",
        borderColor: "#36506f",
        backgroundColor: "#13202d",
        badgeBackground: "#284360",
        badgeColor: "#b9d9ff",
      };
  }
}

export function getProductAreaSectionId(product_area: ProductArea) {
  return `product-area-${product_area.id}`;
}

export function getCapabilitySectionId(capability: Capability) {
  return `capability-${capability.id}`;
}

export function buildProductOverviewToc(tree: ProductTree | undefined, hasProductLevelWorkItems: boolean): ProductOverviewTocItem[] {
  const items: ProductOverviewTocItem[] = [{ id: PRODUCT_OVERVIEW_TOP_ID, title: "Overview", level: 0 }];

  if (hasProductLevelWorkItems) {
    items.push({ id: PRODUCT_DELIVERY_ID, title: "Product Delivery", level: 0 });
  }

  appendHierarchyToc(items, tree?.roots ?? [], "", 0);

  return items;
}

function appendHierarchyToc(items: ProductOverviewTocItem[], nodes: HierarchyTreeNode[], prefix: string, level: number) {
  nodes.forEach((node, index) => {
    const numbering = prefix ? `${prefix}.${index + 1}` : `${index + 1}`;
    items.push({
      id: getHierarchyNodeSectionId(node),
      title: `${numbering}. ${node.name}`,
      level,
    });
    appendHierarchyToc(items, node.children, numbering, level + 1);
  });
}
