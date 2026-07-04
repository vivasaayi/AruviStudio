import type { HierarchyTreeNode, ProductTree } from "../../../lib/types";
import { getHierarchyNodeKindLabel } from "../../../lib/hierarchyLabels";
import {
  PRODUCT_DELIVERY_ID,
  PRODUCT_OVERVIEW_TOP_ID,
  type ProductOverviewTocItem,
} from "./productOverview";
import { escapeHtml } from "./bookExportTextRendering";
import {
  BOOK_CONTENTS_ID,
  BOOK_EXPORT_NOTES_ID,
  BOOK_NODE_INDEX_ID,
  BOOK_PUBLISHING_DETAILS_ID,
  BOOK_REFERENCE_ATLAS_ID,
} from "./bookExportOptions";

export type BookTocNode = ProductOverviewTocItem & {
  children: BookTocNode[];
};

export type IndexEntry = {
  id: string;
  title: string;
  pathLabel: string;
  kindLabel: string;
};

export function renderBookContentsHtml(tocTree: BookTocNode[]): string {
  return tocTree.map((node) => renderBookContentsNode(node)).join("");
}

function renderBookContentsNode(node: BookTocNode): string {
  return `
    <div class="toc-group">
      <div class="toc-${node.level === 0 ? "product_area" : "capability"}">
        <a class="inline-link" href="#${node.id}">${escapeHtml(node.title)}</a>
        <span>${node.children.length > 0 ? `${node.children.length} sections` : ""}</span>
      </div>
      ${node.children.length > 0 ? `<div class="toc-children">${node.children.map((child) => renderBookContentsNode(child)).join("")}</div>` : ""}
    </div>
  `;
}

export function collectNodeIndex(nodes: HierarchyTreeNode[]): IndexEntry[] {
  const entries: IndexEntry[] = [];
  const visit = (node: HierarchyTreeNode) => {
    entries.push({
      id: getTreeNodeSectionId(node),
      title: node.name,
      pathLabel: node.path.join(" / "),
      kindLabel: getHierarchyNodeKindLabel(node.node_kind),
    });
    node.children.forEach(visit);
  };
  nodes.forEach(visit);
  return entries.sort((left, right) => left.title.localeCompare(right.title));
}

export function buildBookTocItems(
  tree: ProductTree | undefined,
  hasProductLevelWorkItems: boolean,
  includeBackMatter: boolean,
): ProductOverviewTocItem[] {
  const items: ProductOverviewTocItem[] = [
    { id: PRODUCT_OVERVIEW_TOP_ID, title: "Title Page", level: 0 },
    { id: BOOK_PUBLISHING_DETAILS_ID, title: "Publishing Details", level: 0 },
    { id: BOOK_CONTENTS_ID, title: "Contents", level: 0 },
  ];

  if (hasProductLevelWorkItems) {
    items.push({ id: PRODUCT_DELIVERY_ID, title: "Product Delivery Themes", level: 0 });
  }

  appendBookHierarchyToc(items, tree?.roots ?? [], "", 0);

  if (includeBackMatter) {
    items.push({ id: BOOK_REFERENCE_ATLAS_ID, title: "Reference Atlas", level: 0 });
    items.push({ id: BOOK_NODE_INDEX_ID, title: "Node Index", level: 0 });
    items.push({ id: BOOK_EXPORT_NOTES_ID, title: "Export Notes", level: 0 });
  }

  return items;
}

function appendBookHierarchyToc(
  items: ProductOverviewTocItem[],
  nodes: HierarchyTreeNode[],
  prefix: string,
  level: number,
) {
  nodes.forEach((node, index) => {
    const numbering = prefix ? `${prefix}.${index + 1}` : `${index + 1}`;
    items.push({
      id: getTreeNodeSectionId(node),
      title: `${numbering}. ${node.name}`,
      level,
    });
    appendBookHierarchyToc(items, node.children, numbering, level + 1);
  });
}

function getTreeNodeSectionId(node: HierarchyTreeNode) {
  return node.node_type === "product_area"
    ? `product-area-${node.id}`
    : `capability-${node.capability_id ?? node.id}`;
}

export function buildBookTocTree(items: ProductOverviewTocItem[]): BookTocNode[] {
  const roots: BookTocNode[] = [];
  const stack: BookTocNode[] = [];

  items.forEach((item) => {
    const node: BookTocNode = { ...item, children: [] };

    while (stack.length > 0 && stack[stack.length - 1].level >= item.level) {
      stack.pop();
    }

    if (stack.length === 0) {
      roots.push(node);
    } else {
      stack[stack.length - 1].children.push(node);
    }

    stack.push(node);
  });

  return roots;
}

export function renderWebSidebar(productName: string, tocTree: BookTocNode[]) {
  return `
    <aside class="book-sidebar">
      <div class="book-sidebar-inner">
        <div class="book-sidebar-kicker">Book Navigation</div>
        <div class="book-sidebar-title">${escapeHtml(productName)}</div>
        <div class="book-sidebar-note">Jump through front matter, chapters, and back matter from a persistent outline rather than a centered single-column preview.</div>
        <nav class="book-sidebar-nav">
          ${tocTree.map((node) => renderWebSidebarNode(node)).join("")}
        </nav>
      </div>
    </aside>
  `;
}

function renderWebSidebarNode(node: BookTocNode): string {
  return `
    <div class="book-sidebar-node">
      <a class="book-sidebar-link" href="#${node.id}">${escapeHtml(node.title)}</a>
      ${node.children.length > 0
        ? `<div class="book-sidebar-children">${node.children.map((child) => renderWebSidebarNode(child)).join("")}</div>`
        : ""}
    </div>
  `;
}
