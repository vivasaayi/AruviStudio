import { getHierarchyNodeSectionId } from "../../../lib/hierarchyTree";
import type { CapabilityTree, HierarchyTreeNode, ProductAreaTree, ProductTree, WorkItem } from "../../../lib/types";
import { getHierarchyNodeKindLabel } from "../../../lib/hierarchyLabels";
import {
  PRODUCT_DELIVERY_ID,
  PRODUCT_OVERVIEW_TOP_ID,
  buildScopedWorkItemTree,
  buildWorkItemMetrics,
  getCapabilitySectionId,
  getProductAreaSectionId,
  getWorkItemPresentation,
  type WorkItemNode,
} from "./productOverviewModel";
import {
  escapeHtml,
  getProductAreaScopedWorkItems,
  renderWorkItemTreeHtml,
  summarizeText,
  toHtmlParagraph,
} from "./productOverviewRenderHelpers";

export function renderBookContentsHtml(tree: ProductTree | undefined, hasProductLevelWorkItems: boolean): string {
  const blocks: string[] = [];

  blocks.push(`
    <div class="toc-group">
      <div class="toc-product_area">
        <a href="#${PRODUCT_OVERVIEW_TOP_ID}" class="inline-link">Title Page</a>
        <span>1</span>
      </div>
    </div>
  `);

  if (hasProductLevelWorkItems) {
    blocks.push(`
      <div class="toc-group">
        <div class="toc-product_area">
          <a href="#${PRODUCT_DELIVERY_ID}" class="inline-link">Product Delivery Themes</a>
          <span>Prelude</span>
        </div>
      </div>
    `);
  }

  (tree?.roots ?? []).forEach((node, index) => {
    blocks.push(renderBookContentsNode(node, `${index + 1}`));
  });

  return blocks.join("");
}

function renderBookContentsNode(node: HierarchyTreeNode, numbering: string): string {
  const childrenMarkup = node.children.length > 0
    ? `<div class="toc-children">${node.children.map((child, index) => renderBookContentsNode(child, `${numbering}.${index + 1}`)).join("")}</div>`
    : "";

  return `
    <div class="toc-group">
      <div class="toc-product_area">
        <a href="#${getHierarchyNodeSectionId(node)}" class="inline-link">${escapeHtml(numbering)}. ${escapeHtml(node.name)}</a>
        <span>${node.children.length > 0 ? node.children.length : ""}</span>
      </div>
      ${childrenMarkup}
    </div>
  `;
}

export function renderBookProductAreaHtml(productAreaTree: ProductAreaTree, chapterNumber: number, allWorkItems: WorkItem[]): string {
  const productAreaScopedItems = getProductAreaScopedWorkItems(productAreaTree, allWorkItems);
  const metrics = buildWorkItemMetrics(productAreaScopedItems);
  const directProductAreaWorkItems = buildScopedWorkItemTree(
    allWorkItems.filter((workItem) => workItem.product_area_id === productAreaTree.product_area.id && !workItem.capability_id),
  );
  const rootKindLabel = getHierarchyNodeKindLabel(productAreaTree.product_area.node_kind);
  const childCountLabel = productAreaTree.features.length === 1 ? "child node" : "child nodes";

  return `
    <section class="page page-break" id="${getProductAreaSectionId(productAreaTree.product_area)}">
      <div class="chapter-kicker">${escapeHtml(rootKindLabel)} ${chapterNumber}</div>
      <h2 class="chapter-title">${escapeHtml(productAreaTree.product_area.name)}</h2>
      <div class="chapter-intro">${toHtmlParagraph(productAreaTree.product_area.description || productAreaTree.product_area.purpose || "This chapter describes the product area's role inside the product.")}</div>
      <div class="chapter-stats">
        <span class="stat-pill">${productAreaTree.features.length} ${childCountLabel}</span>
        <span class="stat-pill">${metrics.done} done</span>
        <span class="stat-pill">${metrics.wip} active</span>
        <span class="stat-pill">${metrics.tbd} planned</span>
      </div>
      ${productAreaTree.product_area.purpose ? `
        <div class="note-block">
          <div class="note-label">Purpose</div>
          <div class="note-copy">${toHtmlParagraph(productAreaTree.product_area.purpose)}</div>
        </div>
      ` : ""}
      ${productAreaTree.product_area.explanation ? `
        <div class="note-block">
          <div class="note-label">Explanation</div>
          <div class="note-copy">${toHtmlParagraph(productAreaTree.product_area.explanation)}</div>
        </div>
      ` : ""}
      ${productAreaTree.product_area.examples ? `
        <div class="note-block">
          <div class="note-label">Examples</div>
          <div class="note-copy">${toHtmlParagraph(productAreaTree.product_area.examples)}</div>
        </div>
      ` : ""}
      ${productAreaTree.product_area.implementation_notes ? `
        <div class="note-block">
          <div class="note-label">Implementation Notes</div>
          <div class="note-copy">${toHtmlParagraph(productAreaTree.product_area.implementation_notes)}</div>
        </div>
      ` : ""}
      ${productAreaTree.product_area.test_guidance ? `
        <div class="note-block">
          <div class="note-label">Test Guidance</div>
          <div class="note-copy">${toHtmlParagraph(productAreaTree.product_area.test_guidance)}</div>
        </div>
      ` : ""}
      ${directProductAreaWorkItems.length > 0 ? `
        <div class="section-block">
          <div class="section-kicker">Direct Delivery Notes</div>
          ${renderBookWorkItemList(directProductAreaWorkItems)}
        </div>
      ` : ""}
      ${productAreaTree.features.length > 0
        ? productAreaTree.features.map((capabilityTree, index) => renderBookCapabilityHtml(capabilityTree, `${chapterNumber}.${index + 1}`, allWorkItems)).join("")
        : `<div class="section-block"><div class="body-copy">No capabilities are defined for this product area yet.</div></div>`}
      <div class="footer-note">End of chapter ${chapterNumber}.</div>
    </section>
  `;
}

function renderBookCapabilityHtml(capabilityTree: CapabilityTree, numbering: string, allWorkItems: WorkItem[]): string {
  const capabilityType = getHierarchyNodeKindLabel(capabilityTree.capability.node_kind);
  const directWorkItems = buildScopedWorkItemTree(
    allWorkItems.filter((workItem) => workItem.capability_id === capabilityTree.capability.id),
  );

  return `
    <section class="capability" id="${getCapabilitySectionId(capabilityTree.capability)}">
      <div class="section-kicker">${capabilityType} ${escapeHtml(numbering)}</div>
      <h3 class="capability-title">${escapeHtml(capabilityTree.capability.name)}</h3>
      <div class="body-copy" style="margin-top: 10px;">${toHtmlParagraph(capabilityTree.capability.description || `This ${capabilityType.toLowerCase()} needs a fuller narrative in Aruvi Studio.`)}</div>
      <div class="capability-meta">
        <span class="meta-chip">${escapeHtml(capabilityTree.capability.status.replace(/_/g, " "))}</span>
        <span class="meta-chip">${escapeHtml(capabilityTree.capability.priority)} priority</span>
        <span class="meta-chip">${escapeHtml(capabilityTree.capability.risk)} risk</span>
      </div>
      ${capabilityTree.capability.acceptance_criteria ? `
        <div class="note-block">
          <div class="note-label">Acceptance</div>
          <div class="note-copy">${toHtmlParagraph(capabilityTree.capability.acceptance_criteria)}</div>
        </div>
      ` : ""}
      ${capabilityTree.capability.explanation ? `
        <div class="note-block">
          <div class="note-label">Explanation</div>
          <div class="note-copy">${toHtmlParagraph(capabilityTree.capability.explanation)}</div>
        </div>
      ` : ""}
      ${capabilityTree.capability.examples ? `
        <div class="note-block">
          <div class="note-label">Examples</div>
          <div class="note-copy">${toHtmlParagraph(capabilityTree.capability.examples)}</div>
        </div>
      ` : ""}
      ${capabilityTree.capability.technical_notes ? `
        <div class="note-block">
          <div class="note-label">Implementation Notes</div>
          <div class="note-copy">${toHtmlParagraph(capabilityTree.capability.technical_notes)}</div>
        </div>
      ` : ""}
      ${capabilityTree.capability.implementation_notes ? `
        <div class="note-block">
          <div class="note-label">Build Notes</div>
          <div class="note-copy">${toHtmlParagraph(capabilityTree.capability.implementation_notes)}</div>
        </div>
      ` : ""}
      ${capabilityTree.capability.test_guidance ? `
        <div class="note-block">
          <div class="note-label">Test Guidance</div>
          <div class="note-copy">${toHtmlParagraph(capabilityTree.capability.test_guidance)}</div>
        </div>
      ` : ""}
      ${directWorkItems.length > 0 ? `
        <div class="section-block">
          <div class="section-kicker">Delivery Notes</div>
          ${renderBookWorkItemList(directWorkItems)}
        </div>
      ` : ""}
      ${capabilityTree.children.length > 0
        ? capabilityTree.children.map((child, index) => renderBookCapabilityHtml(child, `${numbering}.${index + 1}`, allWorkItems)).join("")
        : ""}
    </section>
  `;
}

export function renderBookWorkItemList(nodes: WorkItemNode[]): string {
  return `<div class="work-list">${nodes.map((node) => renderBookWorkItem(node)).join("")}</div>`;
}

function renderBookWorkItem(node: WorkItemNode): string {
  const presentation = getWorkItemPresentation(node.workItem.status);
  const excerpt = summarizeText(
    node.workItem.description
      || node.workItem.problem_statement
      || node.workItem.acceptance_criteria
      || "No delivery notes captured yet.",
    280,
  );

  return `
    <div class="work-item">
      <div class="work-head">
        <div class="work-title">${escapeHtml(node.workItem.title)}</div>
        <div class="work-status ${presentation.toneClass}">${presentation.label}</div>
      </div>
      <div class="work-copy">${escapeHtml(excerpt)}</div>
      ${node.children.length > 0 ? `<div class="child-work">${renderBookWorkItemList(node.children)}</div>` : ""}
    </div>
  `;
}
