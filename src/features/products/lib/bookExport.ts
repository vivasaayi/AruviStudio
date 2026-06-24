import { countHierarchyNodes, getProductDirectWorkItems } from "../../../lib/hierarchyTree";
import type { Product, ProductReference, ProductTree, WorkItem } from "../../../lib/types";
import {
  PRODUCT_DELIVERY_ID,
  PRODUCT_OVERVIEW_TOP_ID,
  buildScopedWorkItemTree,
  buildWorkItemMetrics,
  sortWorkItems,
  type WorkItemNode,
} from "./productOverview";
import {
  escapeHtml,
  renderInlineRichText,
  renderRichTextHtml,
} from "./bookExportTextRendering";
import { renderBookExportStyles } from "./bookExportStyles";
import {
  BOOK_CONTENTS_ID,
  BOOK_EXPORT_NOTES_ID,
  BOOK_NODE_INDEX_ID,
  BOOK_PUBLISHING_DETAILS_ID,
  BOOK_REFERENCE_ATLAS_ID,
  getBookExportTrimPreset,
  type BookExportTrimPreset,
  type ProductOverviewBookBundle,
  type ProductOverviewBookOptions,
} from "./bookExportOptions";
import {
  buildBookTocItems,
  buildBookTocTree,
  collectNodeIndex,
  renderBookContentsHtml,
  renderWebSidebar,
  type BookTocNode,
  type IndexEntry,
} from "./bookExportToc";
import {
  buildReferenceAtlas,
  filterBookReferences,
  renderBookReferencesHtml,
  type ReferenceAtlasEntry,
} from "./bookExportReferences";
import { renderBookProductAreaHtml, renderBookWorkItemList } from "./bookExportChapters";
import { filterReferencesForScope } from "./productReferences";
export {
  BOOK_EXPORT_TRIM_PRESETS,
  getBookExportTrimPreset,
  type BookExportRenderMode,
  type BookExportTrimPreset,
  type BookExportTrimPresetId,
  type ProductOverviewBookBundle,
  type ProductOverviewBookOptions,
} from "./bookExportOptions";

export function buildProductOverviewBookHtml(
  input: { product: Product; tree?: ProductTree; workItems?: WorkItem[]; references?: ProductReference[] },
  options: ProductOverviewBookOptions = {},
) {
  return buildProductOverviewBookBundle(input, options).html;
}

export function buildProductOverviewBookBundle(
  {
    product,
    tree,
    workItems = [],
    references = [],
  }: {
    product: Product;
    tree?: ProductTree;
    workItems?: WorkItem[];
    references?: ProductReference[];
  },
  options: ProductOverviewBookOptions = {},
): ProductOverviewBookBundle {
  const trimPreset = getBookExportTrimPreset(options.trimPreset);
  const renderMode = options.renderMode ?? "web";
  const includeFrontMatter = options.includeFrontMatter ?? true;
  const includeBackMatter = options.includeBackMatter ?? true;
  const allWorkItems = sortWorkItems(workItems);
  const metrics = buildWorkItemMetrics(allWorkItems);
  const rootSectionCount = tree?.roots.length ?? 0;
  const totalNodeCount = tree ? countHierarchyNodes(tree.roots) : 0;
  const productLevelWorkItems = buildScopedWorkItemTree(getProductDirectWorkItems(allWorkItems));
  const generatedAt = new Date().toLocaleString();
  const tocItems = buildBookTocItems(
    tree,
    productLevelWorkItems.length > 0,
    includeBackMatter,
  );
  const tocTree = buildBookTocTree(tocItems);
  const bookReferences = filterBookReferences(product, tree, references);
  const referenceAtlas = buildReferenceAtlas(product, tree, bookReferences);
  const nodeIndex = collectNodeIndex(tree?.roots ?? []);

  return {
    tocItems,
    trimPreset,
    html: `<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml" lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>${escapeHtml(product.name)} - Book</title>
    <style>
${renderBookExportStyles({ trimPreset, renderMode })}
    </style>
  </head>
  <body>
    <div class="book-shell">
      ${renderMode === "web" ? renderWebSidebar(product.name, tocTree) : ""}
      <main class="book-main">
        <article class="book">
          ${includeFrontMatter ? renderFrontMatter({
            product,
            tree,
            metrics,
            rootSectionCount,
            totalNodeCount,
            productLevelWorkItems,
            references: bookReferences,
            generatedAt,
            tocTree,
          }) : ""}
          ${productLevelWorkItems.length > 0 ? `
            <section class="page" id="${PRODUCT_DELIVERY_ID}">
              <div class="chapter-kicker">Prelude</div>
              <h2 class="chapter-title">Product Delivery Themes</h2>
              <div class="chapter-intro">Cross-cutting work attached directly to the book, presented as implementation themes rather than ticket-level execution details.</div>
              ${renderBookWorkItemList(productLevelWorkItems)}
            </section>
          ` : ""}
          ${(tree?.product_areas ?? []).length > 0
            ? (tree?.product_areas ?? []).map((productAreaTree, index) => renderBookProductAreaHtml(productAreaTree, index + 1, allWorkItems, bookReferences)).join("")
            : `
              <section class="page">
                <div class="chapter-kicker">Catalog</div>
                <h2 class="chapter-title">No Product Areas Yet</h2>
                <div class="chapter-intro">Create the first product area in Aruvi Studio to turn this product into a readable book.</div>
              </section>
            `}
          ${includeBackMatter ? renderBackMatter({
            product,
            generatedAt,
            trimPreset,
            totalNodeCount,
            metrics,
            referenceAtlas,
            nodeIndex,
          }) : ""}
        </article>
      </main>
    </div>
  </body>
</html>`,
  };
}

function renderFrontMatter({
  product,
  tree,
  metrics,
  rootSectionCount,
  totalNodeCount,
  productLevelWorkItems,
  references,
  generatedAt,
  tocTree,
}: {
  product: Product;
  tree?: ProductTree;
  metrics: ReturnType<typeof buildWorkItemMetrics>;
  rootSectionCount: number;
  totalNodeCount: number;
  productLevelWorkItems: WorkItemNode[];
  references: ProductReference[];
  generatedAt: string;
  tocTree: BookTocNode[];
}) {
  const productReferences = filterReferencesForScope(references, { scopeType: "product", scopeId: product.id });

  return `
    <section class="page title-page" id="${PRODUCT_OVERVIEW_TOP_ID}">
      <div class="kicker">Aruvi Studio Book</div>
      <h1>${escapeHtml(product.name)}</h1>
      <div class="deck">${renderRichTextHtml(product.description || "A durable product narrative generated from Aruvi Studio.")}</div>
      <div class="book-meta">
        <div class="meta-item"><strong>Product Areas</strong>${rootSectionCount}</div>
        <div class="meta-item"><strong>Total Nodes</strong>${totalNodeCount}</div>
        <div class="meta-item"><strong>References</strong>${references.length}</div>
        <div class="meta-item"><strong>Delivery</strong>${metrics.done} done, ${metrics.wip} active, ${metrics.tbd} planned</div>
        <div class="meta-item"><strong>Generated</strong>${escapeHtml(generatedAt)}</div>
      </div>
    </section>

    <section class="page" id="${BOOK_PUBLISHING_DETAILS_ID}">
      <div class="chapter-kicker">Front Matter</div>
      <h2 class="chapter-title">Publishing Details</h2>
      <div class="front-grid">
        <div class="panel">
          <h3>Rights</h3>
          <div class="index-copy">Generated from the Aruvi Studio semantic product tree for review, EPUB export, and print-ready PDF creation.</div>
        </div>
        <div class="panel">
          <h3>Edition</h3>
          <div class="index-copy">Prepared on ${escapeHtml(generatedAt)} from the current live product structure and attached delivery stories.</div>
        </div>
        <div class="panel">
          <h3>Catalog Shape</h3>
          <div class="index-copy">${rootSectionCount} product areas, ${totalNodeCount} total management nodes, ${productLevelWorkItems.length} product-level delivery themes.</div>
        </div>
        <div class="panel">
          <h3>Evidence</h3>
          <div class="index-copy">${references.length} scoped ${references.length === 1 ? "reference" : "references"} included in this edition.</div>
        </div>
      </div>
    </section>

    <section class="page" id="${BOOK_CONTENTS_ID}">
      <div class="toc-title">Contents</div>
      <div class="lead">This edition keeps the product tree readable as a technical book: orientation first, then product areas, capabilities, and features, with delivery stories shown as concise implementation notes.</div>
      <div class="section-grid">
        <div class="panel">
          <h3>Vision</h3>
          <div class="body-copy">${renderRichTextHtml(product.vision || "No product vision recorded yet.")}</div>
        </div>
        <div class="panel">
          <h3>Goals</h3>
          ${product.goals.length > 0 ? `<ol class="goal-list">${product.goals.map((goal) => `<li>${renderInlineRichText(goal)}</li>`).join("")}</ol>` : `<div class="body-copy">No goals recorded yet.</div>`}
        </div>
      </div>
      ${renderBookReferencesHtml(productReferences)}
      <div class="toc-list">
        ${renderBookContentsHtml(tocTree)}
      </div>
    </section>
  `;
}

function renderBackMatter({
  product,
  generatedAt,
  trimPreset,
  totalNodeCount,
  metrics,
  referenceAtlas,
  nodeIndex,
}: {
  product: Product;
  generatedAt: string;
  trimPreset: BookExportTrimPreset;
  totalNodeCount: number;
  metrics: ReturnType<typeof buildWorkItemMetrics>;
  referenceAtlas: ReferenceAtlasEntry[];
  nodeIndex: IndexEntry[];
}) {
  return `
    <section class="page" id="${BOOK_REFERENCE_ATLAS_ID}">
      <div class="chapter-kicker">Back Matter</div>
      <h2 class="chapter-title">Reference Atlas</h2>
      <div class="chapter-intro">Quick lookup for scoped notes, external docs, evidence, architecture references, and standards attached to this product book.</div>
      ${referenceAtlas.length > 0 ? `
        <div class="reference-list">
          ${referenceAtlas.map((entry) => `
            <div class="reference-item" id="reference-${entry.id}">
              <div class="meta-label">${escapeHtml(entry.kindLabel)}</div>
              <h3 style="margin-top: 6px;">${escapeHtml(entry.title)}</h3>
              <div class="reference-path">${escapeHtml(entry.pathLabel)}</div>
              <div class="note-copy">${renderRichTextHtml(entry.summary || "No summary recorded yet.")}</div>
              ${entry.uri ? `<a class="reference-uri" href="${escapeHtml(entry.uri)}">${escapeHtml(entry.uri)}</a>` : ""}
            </div>
          `).join("")}
        </div>
      ` : `<div class="body-copy">No scoped references are attached to this edition yet.</div>`}
    </section>

    <section class="page" id="${BOOK_NODE_INDEX_ID}">
      <div class="chapter-kicker">Back Matter</div>
      <h2 class="chapter-title">Node Index</h2>
      <div class="chapter-intro">Alphabetical index of structural nodes, useful when the PDF sidebar or EPUB nav is too coarse.</div>
      <div class="index-list">
        ${nodeIndex.map((entry) => `
          <div class="index-item">
            <a href="#${entry.id}" class="inline-link"><strong>${escapeHtml(entry.title)}</strong></a>
            <div class="index-path">${escapeHtml(entry.kindLabel)} · ${escapeHtml(entry.pathLabel)}</div>
          </div>
        `).join("")}
      </div>
    </section>

    <section class="page" id="${BOOK_EXPORT_NOTES_ID}">
      <div class="chapter-kicker">Colophon</div>
      <h2 class="chapter-title">Export Notes</h2>
      <div class="back-grid">
        <div class="panel">
          <h3>Trim Preset</h3>
          <div class="index-copy">${escapeHtml(trimPreset.label)}. ${escapeHtml(trimPreset.description)}</div>
        </div>
        <div class="panel">
          <h3>Delivery Snapshot</h3>
          <div class="index-copy">${metrics.done} done, ${metrics.wip} active, ${metrics.tbd} planned, ${metrics.blocked} blocked.</div>
        </div>
        <div class="panel">
          <h3>Catalog Snapshot</h3>
          <div class="index-copy">${escapeHtml(product.name)} exported with ${totalNodeCount} semantic nodes on ${escapeHtml(generatedAt)}.</div>
        </div>
      </div>
    </section>
  `;
}
