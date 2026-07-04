import type { HierarchyTreeNode, Product, ProductReference, ProductTree } from "../../../lib/types";
import { getReferenceKindLabel } from "./productReferences";
import { escapeHtml, renderRichTextHtml } from "./bookExportTextRendering";

export type ReferenceAtlasEntry = {
  id: string;
  kindLabel: string;
  pathLabel: string;
  title: string;
  summary: string;
  uri: string;
};

export function renderBookReferencesHtml(references: ProductReference[]): string {
  if (references.length === 0) {
    return "";
  }

  return `
    <div class="reference-list">
      ${references.map((reference) => `
        <div class="reference-item">
          <div class="meta-label">${escapeHtml(getReferenceKindLabel(reference.reference_kind))}</div>
          <h3 style="margin-top: 6px;">${escapeHtml(reference.title)}</h3>
          ${reference.content ? `<div class="note-copy">${renderRichTextHtml(reference.content)}</div>` : ""}
          ${reference.uri ? `<a class="reference-uri" href="${escapeHtml(reference.uri)}">${escapeHtml(reference.uri)}</a>` : ""}
        </div>
      `).join("")}
    </div>
  `;
}

export function filterBookReferences(
  product: Product,
  tree: ProductTree | undefined,
  references: ProductReference[],
): ProductReference[] {
  const scopeLabels = buildReferenceScopeLabels(product, tree);
  return references.filter((reference) => scopeLabels.has(getReferenceScopeKey(reference)));
}

export function buildReferenceAtlas(
  product: Product,
  tree: ProductTree | undefined,
  references: ProductReference[],
): ReferenceAtlasEntry[] {
  const scopeLabels = buildReferenceScopeLabels(product, tree);
  return references.map((reference) => ({
    id: reference.id,
    kindLabel: getReferenceKindLabel(reference.reference_kind),
    pathLabel: scopeLabels.get(getReferenceScopeKey(reference)) ?? `${reference.scope_type} / ${reference.scope_id}`,
    title: reference.title,
    summary: reference.content,
    uri: reference.uri,
  }));
}

function buildReferenceScopeLabels(product: Product, tree: ProductTree | undefined): Map<string, string> {
  const scopeLabels = new Map<string, string>();
  scopeLabels.set(`product:${product.id}`, product.name);

  const visit = (node: HierarchyTreeNode) => {
    if (node.node_type === "product_area") {
      scopeLabels.set(`product_area:${node.id}`, node.path.join(" / "));
    } else if (node.capability_id) {
      const scopeType = node.node_kind === "feature" ? "feature" : "capability";
      scopeLabels.set(`${scopeType}:${node.capability_id}`, node.path.join(" / "));
    }
    node.children.forEach(visit);
  };

  (tree?.roots ?? []).forEach(visit);
  return scopeLabels;
}

function getReferenceScopeKey(reference: ProductReference) {
  return `${reference.scope_type}:${reference.scope_id}`;
}
