import { getHierarchyNodeSectionId } from "../../../lib/hierarchyTree";
import type { CapabilityTree, HierarchyTreeNode, ProductAreaTree, ProductReference, ProductTree, WorkItem } from "../../../lib/types";
import { getHierarchyNodeKindLabel } from "../../../lib/hierarchyLabels";
import {
  filterReferencesForScope,
  getCapabilityReferenceScope,
  getProductAreaReferenceScope,
  getReferenceKindLabel,
} from "./productReferences";
import {
  PRODUCT_DELIVERY_ID,
  PRODUCT_OVERVIEW_TOP_ID,
  buildScopedWorkItemTree,
  buildWorkItemMetrics,
  countCapabilityTree,
  getCapabilitySectionId,
  getProductAreaSectionId,
  getWorkItemPresentation,
  type ProductOverviewTocItem,
  type WorkItemMetrics,
  type WorkItemNode,
} from "./productOverviewModel";

type ProductOverviewTocGroup = {
  item: ProductOverviewTocItem;
  children: ProductOverviewTocItem[];
};

export function groupTocItems(items: ProductOverviewTocItem[]): ProductOverviewTocGroup[] {
  const groups: ProductOverviewTocGroup[] = [];

  items.forEach((item) => {
    if (item.level === 0) {
      groups.push({ item, children: [] });
      return;
    }

    const currentGroup = groups[groups.length - 1];
    if (currentGroup) {
      currentGroup.children.push(item);
    }
  });

  return groups;
}

export function collectCapabilityIds(capabilities: CapabilityTree[]): Set<string> {
  const ids = new Set<string>();
  capabilities.forEach((capabilityTree) => {
    ids.add(capabilityTree.capability.id);
    collectCapabilityIds(capabilityTree.children).forEach((id) => ids.add(id));
  });
  return ids;
}

export function getProductAreaScopedWorkItems(productAreaTree: ProductAreaTree, allWorkItems: WorkItem[]) {
  const capabilityIds = collectCapabilityIds(productAreaTree.features);
  return allWorkItems.filter(
    (workItem) => workItem.product_area_id === productAreaTree.product_area.id || (workItem.capability_id ? capabilityIds.has(workItem.capability_id) : false),
  );
}

export function getCapabilityScopedWorkItems(capabilityTree: CapabilityTree, allWorkItems: WorkItem[]) {
  const capabilityIds = collectCapabilityIds([capabilityTree]);
  return allWorkItems.filter((workItem) => workItem.capability_id ? capabilityIds.has(workItem.capability_id) : false);
}

export function renderProductAreaHtml(productAreaTree: ProductAreaTree, chapterNumber: number, allWorkItems: WorkItem[], references: ProductReference[]): string {
  const productAreaScopedItems = getProductAreaScopedWorkItems(productAreaTree, allWorkItems);
  const productAreaMetrics = buildWorkItemMetrics(productAreaScopedItems);
  const directProductAreaWorkItems = buildScopedWorkItemTree(
    allWorkItems.filter((workItem) => workItem.product_area_id === productAreaTree.product_area.id && !workItem.capability_id),
  );
  const rootKindLabel = getHierarchyNodeKindLabel(productAreaTree.product_area.node_kind);
  const productAreaReferences = filterReferencesForScope(references, getProductAreaReferenceScope(productAreaTree.product_area.id));

  return `
    <section class="chapter" id="${getProductAreaSectionId(productAreaTree.product_area)}">
      <details open>
        <summary>
          <div class="chapter-header">
            <div>
              <div class="chapter-kicker">${escapeHtml(rootKindLabel)} ${chapterNumber}</div>
              <h3>${escapeHtml(productAreaTree.product_area.name)}</h3>
              <p>${toHtmlParagraph(productAreaTree.product_area.description || productAreaTree.product_area.purpose || "Document this product area so the product architecture remains readable.")}</p>
            </div>
            <div class="count-row">
              <span class="count-pill">${productAreaTree.features.length} ${productAreaTree.features.length === 1 ? "child node" : "child nodes"}</span>
              ${renderMetricSummaryPills(productAreaMetrics)}
            </div>
          </div>
        </summary>
        <div class="chapter-body">
          <div class="info-grid">
            ${renderNoteCardHtml("Purpose", productAreaTree.product_area.purpose)}
            ${renderNoteCardHtml("Explanation", productAreaTree.product_area.explanation)}
            ${renderNoteCardHtml("Examples", productAreaTree.product_area.examples)}
            ${renderNoteCardHtml("Implementation Notes", productAreaTree.product_area.implementation_notes)}
            ${renderNoteCardHtml("Test Guidance", productAreaTree.product_area.test_guidance)}
          </div>
          ${renderReferenceListHtml(productAreaReferences)}
          ${directProductAreaWorkItems.length > 0 ? `
            <div class="section-header" style="margin-bottom: 12px;">
              <div class="eyebrow">Direct Work</div>
              <h3 style="font-size: 20px;">Chapter Delivery</h3>
            </div>
            ${renderWorkItemTreeHtml(directProductAreaWorkItems)}
          ` : ""}
          ${productAreaTree.features.length > 0
            ? productAreaTree.features.map((capabilityTree, index) => renderCapabilityHtml(capabilityTree, `${chapterNumber}.${index + 1}`, allWorkItems, references)).join("")
            : `<p class="muted-line">No capabilities defined for this product area yet.</p>`}
        </div>
      </details>
    </section>
  `;
}

export function renderCapabilityHtml(capabilityTree: CapabilityTree, numbering: string, allWorkItems: WorkItem[], references: ProductReference[]): string {
  const capabilityType = getHierarchyNodeKindLabel(capabilityTree.capability.node_kind);
  const scopedItems = getCapabilityScopedWorkItems(capabilityTree, allWorkItems);
  const directWorkItems = buildScopedWorkItemTree(
    allWorkItems.filter((workItem) => workItem.capability_id === capabilityTree.capability.id),
  );
  const metrics = buildWorkItemMetrics(scopedItems);
  const capabilityReferences = filterReferencesForScope(references, getCapabilityReferenceScope(capabilityTree.capability));

  return `
    <section class="capability" id="${getCapabilitySectionId(capabilityTree.capability)}">
      <details>
        <summary>
          <div class="capability-header">
            <div>
              <div class="chapter-kicker">${capabilityType} ${escapeHtml(numbering)}</div>
              <h4 class="capability-title">${escapeHtml(capabilityTree.capability.name)}</h4>
              <p>${toHtmlParagraph(capabilityTree.capability.description || `Document what this ${capabilityType.toLowerCase()} is responsible for.`)}</p>
            </div>
            <div class="count-row">
              <span class="meta-pill">${escapeHtml(capabilityTree.capability.status.replace(/_/g, " "))}</span>
              <span class="meta-pill">${escapeHtml(capabilityTree.capability.priority)} priority</span>
              <span class="meta-pill">${escapeHtml(capabilityTree.capability.risk)} risk</span>
              ${renderMetricSummaryPills(metrics)}
            </div>
          </div>
        </summary>
        <div class="capability-body">
          <div class="info-grid">
            ${renderNoteCardHtml("Acceptance Criteria", capabilityTree.capability.acceptance_criteria)}
            ${renderNoteCardHtml("Explanation", capabilityTree.capability.explanation)}
            ${renderNoteCardHtml("Examples", capabilityTree.capability.examples)}
            ${renderNoteCardHtml("Technical Notes", capabilityTree.capability.technical_notes)}
            ${renderNoteCardHtml("Implementation Notes", capabilityTree.capability.implementation_notes)}
            ${renderNoteCardHtml("Test Guidance", capabilityTree.capability.test_guidance)}
          </div>
          ${renderReferenceListHtml(capabilityReferences)}

          ${directWorkItems.length > 0 ? `
            <div class="section-header" style="margin: 18px 0 12px;">
              <div class="eyebrow">Delivery</div>
              <h3 style="font-size: 20px;">Stories</h3>
            </div>
            ${renderWorkItemTreeHtml(directWorkItems)}
          ` : `<p class="muted-line" style="margin-top: 16px;">No stories attached to this ${capabilityType.toLowerCase()} yet.</p>`}

          ${capabilityTree.children.length > 0
            ? capabilityTree.children.map((child, index) => renderCapabilityHtml(child, `${numbering}.${index + 1}`, allWorkItems, references)).join("")
            : ""}
        </div>
      </details>
    </section>
  `;
}

export function renderNoteCardHtml(label: string, value: string): string {
  if (!value.trim()) {
    return "";
  }

  return `
    <div class="note-card">
      <h4>${escapeHtml(label)}</h4>
      <p>${toHtmlParagraph(value)}</p>
    </div>
  `;
}

export function renderReferenceListHtml(references: ProductReference[]): string {
  if (references.length === 0) {
    return "";
  }

  return `
    <div class="reference-grid">
      ${references.map((reference) => `
        <div class="note-card reference-card">
          <h4>${escapeHtml(getReferenceKindLabel(reference.reference_kind))}</h4>
          <p><strong>${escapeHtml(reference.title)}</strong></p>
          ${reference.content ? `<p>${toHtmlParagraph(reference.content)}</p>` : ""}
          ${reference.uri ? `<a href="${escapeHtml(reference.uri)}">${escapeHtml(reference.uri)}</a>` : ""}
        </div>
      `).join("")}
    </div>
  `;
}

export function renderWorkItemTreeHtml(nodes: WorkItemNode[]): string {
  return `<div class="work-item-list">${nodes.map((node) => renderWorkItemNodeHtml(node)).join("")}</div>`;
}

export function renderWorkItemNodeHtml(node: WorkItemNode): string {
  const presentation = getWorkItemPresentation(node.workItem.status);
  const excerpt = summarizeText(node.workItem.description || node.workItem.problem_statement || node.workItem.acceptance_criteria || "No delivery notes captured yet.");

  return `
    <article class="work-item ${presentation.toneClass}">
      <div class="work-item-header">
        <div>
          <h5>${escapeHtml(node.workItem.title)}</h5>
          <div class="meta-row" style="margin-top: 8px;">
            <span class="status-pill ${presentation.toneClass}">${presentation.label}</span>
            <span class="meta-pill">${escapeHtml(node.workItem.work_item_type.replace(/_/g, " "))}</span>
            <span class="meta-pill">${escapeHtml(node.workItem.priority)} priority</span>
            <span class="meta-pill">${escapeHtml(node.workItem.complexity.replace(/_/g, " "))}</span>
            ${node.children.length > 0 ? `<span class="count-pill">${node.children.length} sub-item${node.children.length === 1 ? "" : "s"}</span>` : ""}
          </div>
        </div>
      </div>
      <p>${escapeHtml(excerpt)}</p>
      ${node.children.length > 0 ? `<div class="work-item-children">${renderWorkItemTreeHtml(node.children)}</div>` : ""}
    </article>
  `;
}

export function renderMetricHtml(label: string, value: number) {
  return `
    <div class="metric">
      <div class="metric-label">${escapeHtml(label)}</div>
      <div class="metric-value">${value}</div>
    </div>
  `;
}

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

export function renderBookContentsNode(node: HierarchyTreeNode, numbering: string): string {
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

export function renderBookContentsChildren(capabilities: CapabilityTree[], prefix: string): string {
  return capabilities.map((capabilityTree, index) => {
    const numbering = `${prefix}.${index + 1}`;
    const self = `
      <div class="toc-capability">
        <a href="#${getCapabilitySectionId(capabilityTree.capability)}" class="inline-link">${escapeHtml(numbering)}. ${escapeHtml(capabilityTree.capability.name)}</a>
        <span>${capabilityTree.children.length > 0 ? countCapabilityTree(capabilityTree) - 1 : ""}</span>
      </div>
    `;

    if (capabilityTree.children.length === 0) {
      return self;
    }

    return `${self}${renderBookContentsChildren(capabilityTree.children, numbering)}`;
  }).join("");
}

export function countCapabilityTreeList(capabilities: CapabilityTree[]) {
  return capabilities.reduce((total, capabilityTree) => total + countCapabilityTree(capabilityTree), 0);
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

export function renderBookCapabilityHtml(capabilityTree: CapabilityTree, numbering: string, allWorkItems: WorkItem[]): string {
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

export function renderBookWorkItem(node: WorkItemNode): string {
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

export function renderTocGroupHtml(group: ProductOverviewTocGroup): string {
  if (group.children.length === 0) {
    return `<a href="#${group.item.id}" data-level="${group.item.level}">${escapeHtml(group.item.title)}</a>`;
  }

  return `
    <details class="toc-group" open>
      <summary>
        <span>${escapeHtml(group.item.title)}</span>
        <span>${group.children.length}</span>
      </summary>
      <div class="toc-group-children">
        <a href="#${group.item.id}" data-level="0">Section overview</a>
        ${group.children.map((item) => `<a href="#${item.id}" data-level="${item.level}">${escapeHtml(item.title)}</a>`).join("")}
      </div>
    </details>
  `;
}

export function renderMetricSummaryPills(metrics: WorkItemMetrics) {
  if (metrics.total === 0) {
    return `<span class="count-pill">No stories</span>`;
  }

  return [
    metrics.done > 0 ? `<span class="status-pill is-done">${metrics.done} done</span>` : "",
    metrics.wip > 0 ? `<span class="status-pill is-wip">${metrics.wip} WIP</span>` : "",
    metrics.tbd > 0 ? `<span class="status-pill is-tbd">${metrics.tbd} TBD</span>` : "",
    metrics.blocked > 0 ? `<span class="status-pill is-blocked">${metrics.blocked} blocked</span>` : "",
  ].filter(Boolean).join("");
}

export function renderLegendRow(label: string, color: string) {
  return `<div class="legend-row"><span class="legend-dot" style="background: ${color};"></span>${escapeHtml(label)}</div>`;
}

export function summarizeText(text: string, maxLength = 220) {
  const normalized = text.replace(/\s+/g, " ").trim();
  if (normalized.length <= maxLength) {
    return normalized;
  }
  return `${normalized.slice(0, maxLength - 1).trimEnd()}…`;
}

export function toHtmlParagraph(text: string) {
  return escapeHtml(text).replace(/\n/g, "<br />");
}

export function escapeHtml(value: string) {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll("\"", "&quot;")
    .replaceAll("'", "&#39;");
}
