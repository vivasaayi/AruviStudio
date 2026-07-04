import type { CapabilityTree, ProductReference, ProductTree, WorkItem } from "../../../lib/types";
import { getHierarchyNodeKindLabel } from "../../../lib/hierarchyLabels";
import { filterReferencesForScope, getCapabilityReferenceScope, getProductAreaReferenceScope } from "./productReferences";
import {
  buildScopedWorkItemTree,
  buildWorkItemMetrics,
  getCapabilitySectionId,
  getProductAreaSectionId,
  getWorkItemPresentation,
  type WorkItemNode,
} from "./productOverview";
import {
  escapeHtml,
  renderRichTextHtml,
  summarizeText,
} from "./bookExportTextRendering";
import { renderBookReferencesHtml } from "./bookExportReferences";

export function renderBookProductAreaHtml(
  productAreaTree: ProductTree["product_areas"][number],
  chapterNumber: number,
  allWorkItems: WorkItem[],
  references: ProductReference[],
): string {
  const productAreaScopedItems = allWorkItems.filter((workItem) => workItem.product_area_id === productAreaTree.product_area.id);
  const metrics = buildWorkItemMetrics(productAreaScopedItems);
  const directProductAreaWorkItems = buildScopedWorkItemTree(
    allWorkItems.filter((workItem) => workItem.product_area_id === productAreaTree.product_area.id && !workItem.capability_id),
  );
  const rootKindLabel = getHierarchyNodeKindLabel(productAreaTree.product_area.node_kind);
  const childCountLabel = productAreaTree.features.length === 1 ? "child node" : "child nodes";
  const productAreaReferences = filterReferencesForScope(references, getProductAreaReferenceScope(productAreaTree.product_area.id));

  return `
    <section class="page" id="${getProductAreaSectionId(productAreaTree.product_area)}">
      <div class="chapter-kicker">${escapeHtml(rootKindLabel)} ${chapterNumber}</div>
      <h2 class="chapter-title">${escapeHtml(productAreaTree.product_area.name)}</h2>
      <div class="chapter-intro">${renderRichTextHtml(productAreaTree.product_area.description || productAreaTree.product_area.purpose || "This chapter describes the product area's role inside the product.")}</div>
      <div class="chapter-stats">
        <span class="stat-pill">${productAreaTree.features.length} ${childCountLabel}</span>
        <span class="stat-pill">${metrics.done} done</span>
        <span class="stat-pill">${metrics.wip} active</span>
        <span class="stat-pill">${metrics.tbd} planned</span>
      </div>
      ${renderNoteBlock("Purpose", productAreaTree.product_area.purpose)}
      ${renderNoteBlock("Explanation", productAreaTree.product_area.explanation)}
      ${renderNoteBlock("Examples", productAreaTree.product_area.examples)}
      ${renderNoteBlock("Implementation Notes", productAreaTree.product_area.implementation_notes)}
      ${renderNoteBlock("Test Guidance", productAreaTree.product_area.test_guidance)}
      ${renderBookReferencesHtml(productAreaReferences)}
      ${directProductAreaWorkItems.length > 0 ? `
        <div class="section-block">
          <div class="section-kicker">Direct Delivery Notes</div>
          ${renderBookWorkItemList(directProductAreaWorkItems)}
        </div>
      ` : ""}
      ${productAreaTree.features.length > 0
        ? productAreaTree.features.map((capabilityTree, index) => renderBookCapabilityHtml(capabilityTree, `${chapterNumber}.${index + 1}`, allWorkItems, references)).join("")
        : `<div class="section-block"><div class="body-copy">No child nodes are defined for this product area yet.</div></div>`}
      <div class="footer-note">End of chapter ${chapterNumber}.</div>
    </section>
  `;
}

function renderBookCapabilityHtml(capabilityTree: CapabilityTree, numbering: string, allWorkItems: WorkItem[], references: ProductReference[]): string {
  const capabilityType = getHierarchyNodeKindLabel(capabilityTree.capability.node_kind);
  const directWorkItems = buildScopedWorkItemTree(
    allWorkItems.filter((workItem) => workItem.capability_id === capabilityTree.capability.id),
  );
  const capabilityReferences = filterReferencesForScope(references, getCapabilityReferenceScope(capabilityTree.capability));

  return `
    <section class="capability" id="${getCapabilitySectionId(capabilityTree.capability)}">
      <div class="section-kicker">${escapeHtml(capabilityType)} ${escapeHtml(numbering)}</div>
      <h3 class="capability-title">${escapeHtml(capabilityTree.capability.name)}</h3>
      <div class="body-copy" style="margin-top: 10px;">${renderRichTextHtml(capabilityTree.capability.description || `This ${capabilityType.toLowerCase()} needs a fuller narrative in Aruvi Studio.`)}</div>
      <div class="capability-meta">
        <span class="meta-chip">${escapeHtml(capabilityTree.capability.status.replace(/_/g, " "))}</span>
        <span class="meta-chip">${escapeHtml(capabilityTree.capability.priority)} priority</span>
        <span class="meta-chip">${escapeHtml(capabilityTree.capability.risk)} risk</span>
      </div>
      ${renderNoteBlock("Acceptance", capabilityTree.capability.acceptance_criteria)}
      ${renderNoteBlock("Explanation", capabilityTree.capability.explanation)}
      ${renderNoteBlock("Examples", capabilityTree.capability.examples)}
      ${renderNoteBlock("Technical Notes", capabilityTree.capability.technical_notes)}
      ${renderNoteBlock("Implementation Notes", capabilityTree.capability.implementation_notes)}
      ${renderNoteBlock("Test Guidance", capabilityTree.capability.test_guidance)}
      ${renderBookReferencesHtml(capabilityReferences)}
      ${directWorkItems.length > 0 ? `
        <div class="section-block">
          <div class="section-kicker">Delivery Notes</div>
          ${renderBookWorkItemList(directWorkItems)}
        </div>
      ` : ""}
      ${capabilityTree.children.length > 0
        ? capabilityTree.children.map((child, index) => renderBookCapabilityHtml(child, `${numbering}.${index + 1}`, allWorkItems, references)).join("")
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

function renderNoteBlock(label: string, text: string) {
  if (!text.trim()) {
    return "";
  }

  return `
    <div class="note-block">
      <div class="note-label">${escapeHtml(label)}</div>
      <div class="note-copy">${renderRichTextHtml(text)}</div>
    </div>
  `;
}
