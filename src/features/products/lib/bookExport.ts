import { countHierarchyNodes, getProductDirectWorkItems } from "../../../lib/hierarchyTree";
import type { CapabilityTree, HierarchyTreeNode, Product, ProductReference, ProductTree, WorkItem } from "../../../lib/types";
import { getHierarchyNodeKindLabel } from "../../../lib/hierarchyLabels";
import { filterReferencesForScope, getCapabilityReferenceScope, getProductAreaReferenceScope, getReferenceKindLabel } from "./productReferences";
import {
  PRODUCT_DELIVERY_ID,
  PRODUCT_OVERVIEW_TOP_ID,
  buildScopedWorkItemTree,
  buildWorkItemMetrics,
  getCapabilitySectionId,
  getProductAreaSectionId,
  getWorkItemPresentation,
  sortWorkItems,
  type ProductOverviewTocItem,
  type WorkItemNode,
} from "./productOverview";
import {
  escapeHtml,
  renderInlineRichText,
  renderRichTextHtml,
  summarizeText,
} from "./bookExportTextRendering";

export type BookExportTrimPresetId = "trade-6x9" | "a5" | "us-letter";
export type BookExportRenderMode = "web" | "print" | "epub";

export type BookExportTrimPreset = {
  id: BookExportTrimPresetId;
  label: string;
  description: string;
  pageWidth: string;
  pageHeight: string;
  contentWidth: string;
  marginTop: string;
  marginRight: string;
  marginBottom: string;
  marginLeft: string;
};

export type ProductOverviewBookOptions = {
  trimPreset?: BookExportTrimPreset | BookExportTrimPresetId;
  renderMode?: BookExportRenderMode;
  includeFrontMatter?: boolean;
  includeBackMatter?: boolean;
};

export type ProductOverviewBookBundle = {
  html: string;
  tocItems: ProductOverviewTocItem[];
  trimPreset: BookExportTrimPreset;
};

type BookTocNode = ProductOverviewTocItem & {
  children: BookTocNode[];
};

export const BOOK_EXPORT_TRIM_PRESETS: BookExportTrimPreset[] = [
  {
    id: "trade-6x9",
    label: "Trade Paperback (6×9)",
    description: "KDP-friendly trade paperback trim with balanced margins.",
    pageWidth: "6in",
    pageHeight: "9in",
    contentWidth: "5.8in",
    marginTop: "0.75in",
    marginRight: "0.72in",
    marginBottom: "0.85in",
    marginLeft: "0.82in",
  },
  {
    id: "a5",
    label: "A5 Book",
    description: "Compact technical handbook format.",
    pageWidth: "148mm",
    pageHeight: "210mm",
    contentWidth: "146mm",
    marginTop: "18mm",
    marginRight: "18mm",
    marginBottom: "22mm",
    marginLeft: "20mm",
  },
  {
    id: "us-letter",
    label: "US Letter Review",
    description: "Reviewer-friendly PDF with larger page real estate.",
    pageWidth: "8.5in",
    pageHeight: "11in",
    contentWidth: "8.25in",
    marginTop: "0.85in",
    marginRight: "0.85in",
    marginBottom: "0.95in",
    marginLeft: "0.95in",
  },
];

type ReferenceAtlasEntry = {
  id: string;
  kindLabel: string;
  pathLabel: string;
  title: string;
  summary: string;
  uri: string;
};

type IndexEntry = {
  id: string;
  title: string;
  pathLabel: string;
  kindLabel: string;
};

const DEFAULT_TRIM_PRESET_ID: BookExportTrimPresetId = "trade-6x9";
const BOOK_PUBLISHING_DETAILS_ID = "book-publishing-details";
const BOOK_CONTENTS_ID = "book-contents";
const BOOK_REFERENCE_ATLAS_ID = "book-reference-atlas";
const BOOK_NODE_INDEX_ID = "book-node-index";
const BOOK_EXPORT_NOTES_ID = "book-export-notes";

export function getBookExportTrimPreset(
  preset: BookExportTrimPreset | BookExportTrimPresetId | undefined,
): BookExportTrimPreset {
  if (preset && typeof preset !== "string") {
    return preset;
  }
  return BOOK_EXPORT_TRIM_PRESETS.find((candidate) => candidate.id === (preset ?? DEFAULT_TRIM_PRESET_ID))
    ?? BOOK_EXPORT_TRIM_PRESETS[0];
}

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
      :root {
        color-scheme: light;
        --paper: #f8f4ec;
        --ink: #1f2733;
        --muted: #596574;
        --rule: #d8ccbc;
        --accent: #8e4c2d;
        --accent-soft: #f0e1d4;
        --chapter: #2f3c4d;
        --done: #2d7d57;
        --wip: #a06c00;
        --tbd: #486aa0;
        --blocked: #a44545;
        --page-width: ${trimPreset.pageWidth};
        --page-height: ${trimPreset.pageHeight};
        --content-width: ${trimPreset.contentWidth};
        --page-margin-top: ${trimPreset.marginTop};
        --page-margin-right: ${trimPreset.marginRight};
        --page-margin-bottom: ${trimPreset.marginBottom};
        --page-margin-left: ${trimPreset.marginLeft};
      }

      * { box-sizing: border-box; }
      html { scroll-behavior: smooth; }

      body {
        margin: 0;
        background: ${renderMode === "web"
          ? 'radial-gradient(circle at top, rgba(255,255,255,0.7), transparent 36%), linear-gradient(180deg, #efe8db 0%, var(--paper) 100%)'
          : "#ffffff"};
        color: var(--ink);
        font-family: "Iowan Old Style", "Palatino Linotype", "Book Antiqua", Palatino, Georgia, serif;
      }

      .book-shell {
        width: 100%;
        max-width: ${renderMode === "web" ? "1680px" : "none"};
        margin: ${renderMode === "web" ? "0 auto" : "0"};
        display: ${renderMode === "web" ? "grid" : "block"};
        grid-template-columns: ${renderMode === "web" ? "320px minmax(0, 1fr)" : "1fr"};
        min-height: ${renderMode === "web" ? "100vh" : "auto"};
      }

      .book-sidebar {
        display: ${renderMode === "web" ? "block" : "none"};
        position: ${renderMode === "web" ? "sticky" : "static"};
        top: 0;
        align-self: start;
        height: 100vh;
        overflow-y: auto;
        padding: 28px 20px 28px 24px;
        border-right: 1px solid rgba(120, 96, 72, 0.12);
        background: rgba(250, 246, 239, 0.92);
        backdrop-filter: blur(10px);
      }

      .book-sidebar-inner {
        display: flex;
        flex-direction: column;
        gap: 16px;
      }

      .book-sidebar-kicker {
        font-family: Inter, ui-sans-serif, system-ui, sans-serif;
        text-transform: uppercase;
        letter-spacing: 0.14em;
        font-size: 11px;
        font-weight: 800;
        color: var(--accent);
      }

      .book-sidebar-title {
        font-family: Inter, ui-sans-serif, system-ui, sans-serif;
        font-size: 23px;
        font-weight: 800;
        line-height: 1.18;
        color: var(--chapter);
      }

      .book-sidebar-note {
        font-family: Inter, ui-sans-serif, system-ui, sans-serif;
        font-size: 12px;
        line-height: 1.6;
        color: var(--muted);
      }

      .book-sidebar-nav {
        display: flex;
        flex-direction: column;
        gap: 8px;
      }

      .book-sidebar-node {
        display: flex;
        flex-direction: column;
        gap: 8px;
      }

      .book-sidebar-link {
        display: block;
        padding: 8px 10px;
        border-radius: 10px;
        color: var(--chapter);
        text-decoration: none;
        font-family: Inter, ui-sans-serif, system-ui, sans-serif;
        font-size: 13px;
        font-weight: 700;
        line-height: 1.45;
      }

      .book-sidebar-link:hover {
        background: rgba(240, 225, 212, 0.58);
        color: var(--accent);
      }

      .book-sidebar-children {
        display: flex;
        flex-direction: column;
        gap: 6px;
        margin-left: 14px;
        padding-left: 12px;
        border-left: 1px solid rgba(120, 96, 72, 0.18);
      }

      .book-main {
        min-width: 0;
        padding: ${renderMode === "web" ? "30px 34px 64px" : "0"};
      }

      .book {
        width: 100%;
        max-width: ${renderMode === "print" ? "var(--content-width)" : "none"};
        margin: ${renderMode === "web" ? "0" : "0 auto"};
        background: ${renderMode === "web" ? "rgba(255,255,255,0.78)" : "#ffffff"};
        border: ${renderMode === "web" ? "1px solid rgba(120, 96, 72, 0.14)" : "none"};
        box-shadow: ${renderMode === "web" ? "0 24px 60px rgba(64, 46, 31, 0.12)" : "none"};
      }

      .page {
        padding: ${renderMode === "epub" ? "2rem 1.4rem" : "58px 72px"};
        border-top: 1px solid rgba(120, 96, 72, 0.08);
        page-break-after: always;
        break-after: page;
      }

      .page:last-child {
        page-break-after: auto;
        break-after: auto;
      }

      .page:first-child { border-top: none; }

      .title-page {
        min-height: ${renderMode === "web" ? "86vh" : "calc(var(--page-height) - var(--page-margin-top) - var(--page-margin-bottom))"};
        display: flex;
        flex-direction: column;
        justify-content: center;
      }

      .kicker,
      .toc-title,
      .section-kicker,
      .chapter-kicker,
      .meta-label,
      .note-label {
        font-family: Inter, ui-sans-serif, system-ui, sans-serif;
        text-transform: uppercase;
        letter-spacing: 0.14em;
        font-size: 11px;
        font-weight: 800;
        color: var(--accent);
      }

      .kicker { letter-spacing: 0.18em; font-size: 12px; }

      h1, h2, h3, h4 {
        margin: 0;
        font-weight: 700;
        color: var(--chapter);
      }

      h1 {
        margin-top: 18px;
        font-size: clamp(42px, 6vw, 64px);
        line-height: 0.96;
        letter-spacing: -0.03em;
      }

      .deck {
        margin-top: 18px;
        max-width: 620px;
        font-size: 23px;
        line-height: 1.6;
        color: var(--muted);
      }

      .book-meta {
        margin-top: 28px;
        padding-top: 20px;
        border-top: 1px solid var(--rule);
        display: flex;
        flex-wrap: wrap;
        gap: 22px;
        font-family: Inter, ui-sans-serif, system-ui, sans-serif;
        font-size: 12px;
        color: var(--muted);
      }

      .meta-item strong {
        display: block;
        margin-bottom: 4px;
        font-size: 11px;
        text-transform: uppercase;
        letter-spacing: 0.08em;
        color: var(--chapter);
      }

      .front-grid,
      .back-grid,
      .section-grid {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
        gap: 18px;
        margin-top: 24px;
      }

      .panel {
        padding: 16px 18px;
        border: 1px solid rgba(120, 96, 72, 0.16);
        background: rgba(255,255,255,0.55);
      }

      .panel h3,
      .panel h4 {
        font-size: 14px;
        font-family: Inter, ui-sans-serif, system-ui, sans-serif;
        text-transform: uppercase;
        letter-spacing: 0.08em;
        margin-bottom: 10px;
      }

      .goal-list {
        margin: 0;
        padding-left: 18px;
      }

      .goal-list li,
      .book-list li {
        margin-top: 10px;
        line-height: 1.7;
      }

      .body-copy,
      .lead,
      .note-copy,
      .work-copy,
      .index-copy {
        line-height: 1.85;
        color: var(--ink);
      }

      .lead {
        font-size: 19px;
        color: var(--muted);
      }

      .body-copy { font-size: 17px; }

      .toc-list {
        display: flex;
        flex-direction: column;
        gap: 18px;
        margin-top: 24px;
      }

      .toc-group {
        border-top: 1px solid var(--rule);
        padding-top: 14px;
      }

      .toc-product_area,
      .toc-capability {
        display: flex;
        justify-content: space-between;
        gap: 18px;
        align-items: baseline;
      }

      .toc-product_area {
        font-family: Inter, ui-sans-serif, system-ui, sans-serif;
        font-size: 18px;
        font-weight: 700;
      }

      .toc-capability {
        font-size: 14px;
        line-height: 1.45;
        color: var(--muted);
      }

      .toc-product_area a,
      .toc-capability a,
      .inline-link {
        color: inherit;
        text-decoration: none;
      }

      .toc-product_area a:hover,
      .toc-capability a:hover,
      .inline-link:hover {
        color: var(--accent);
      }

      .toc-children {
        margin-top: 10px;
        display: flex;
        flex-direction: column;
        gap: 8px;
        padding-left: 14px;
        border-left: 1px solid rgba(120, 96, 72, 0.18);
      }

      .chapter-title {
        margin-top: 10px;
        font-size: 34px;
        line-height: 1.08;
      }

      .chapter-intro {
        margin-top: 12px;
        font-size: 19px;
        line-height: 1.75;
        color: var(--muted);
      }

      .chapter-stats,
      .capability-meta {
        margin-top: 18px;
        display: flex;
        flex-wrap: wrap;
        gap: 10px;
        font-family: Inter, ui-sans-serif, system-ui, sans-serif;
      }

      .stat-pill,
      .meta-chip {
        display: inline-flex;
        align-items: center;
        border-radius: 999px;
        padding: 6px 10px;
        background: var(--accent-soft);
        color: var(--chapter);
        font-size: 12px;
        font-weight: 700;
      }

      .capability {
        margin-top: 32px;
        padding-top: 20px;
        border-top: 1px solid rgba(120, 96, 72, 0.18);
      }

      .capability-title {
        margin-top: 8px;
        font-size: 26px;
        line-height: 1.18;
      }

      .section-block {
        margin-top: 24px;
        padding-top: 18px;
        border-top: 1px solid var(--rule);
      }

      .note-block {
        margin-top: 18px;
        padding-left: 16px;
        border-left: 3px solid var(--rule);
      }

      .note-copy {
        margin-top: 8px;
        font-size: 15px;
        color: var(--muted);
      }

      .book-paragraph {
        margin: 0 0 14px;
        line-height: 1.8;
      }

      .book-list {
        margin: 0;
        padding-left: 20px;
      }

      .book-inline-code {
        font-family: "SFMono-Regular", "SFMono", "Cascadia Code", Menlo, Consolas, monospace;
        font-size: 0.9em;
        background: rgba(46, 61, 82, 0.08);
        border-radius: 4px;
        padding: 0.08rem 0.34rem;
      }

      .book-code,
      .book-figure,
      .book-table-wrap,
      .book-quote {
        margin: 18px 0;
      }

      .book-code pre {
        margin: 0;
        padding: 16px 18px;
        overflow-x: auto;
        background: #1d2430;
        color: #eef4ff;
        border-radius: 12px;
        font-size: 13px;
        line-height: 1.65;
      }

      .book-figure img {
        max-width: 100%;
        border-radius: 10px;
        border: 1px solid rgba(120, 96, 72, 0.18);
      }

      .book-figure figcaption,
      .book-code figcaption,
      .book-table-wrap figcaption {
        margin-top: 8px;
        color: var(--muted);
        font-size: 13px;
        font-family: Inter, ui-sans-serif, system-ui, sans-serif;
      }

      .book-table {
        width: 100%;
        border-collapse: collapse;
        font-size: 14px;
        background: rgba(255,255,255,0.62);
      }

      .book-table th,
      .book-table td {
        padding: 10px 12px;
        border: 1px solid rgba(120, 96, 72, 0.16);
        text-align: left;
        vertical-align: top;
      }

      .book-table th {
        font-family: Inter, ui-sans-serif, system-ui, sans-serif;
        font-size: 12px;
        text-transform: uppercase;
        letter-spacing: 0.08em;
        background: rgba(240, 225, 212, 0.55);
      }

      .book-quote {
        padding: 12px 16px;
        border-left: 3px solid var(--accent);
        background: rgba(240, 225, 212, 0.32);
        color: var(--muted);
      }

      .work-list {
        margin-top: 18px;
        display: flex;
        flex-direction: column;
        gap: 12px;
      }

      .work-item {
        padding: 14px 16px;
        border: 1px solid rgba(120, 96, 72, 0.16);
        background: rgba(255,255,255,0.55);
      }

      .work-head {
        display: flex;
        justify-content: space-between;
        gap: 12px;
        align-items: baseline;
        font-family: Inter, ui-sans-serif, system-ui, sans-serif;
      }

      .work-title {
        font-size: 15px;
        font-weight: 700;
        color: var(--chapter);
      }

      .work-status {
        font-size: 11px;
        font-weight: 800;
        text-transform: uppercase;
        letter-spacing: 0.06em;
      }

      .work-status.is-done { color: var(--done); }
      .work-status.is-wip { color: var(--wip); }
      .work-status.is-tbd { color: var(--tbd); }
      .work-status.is-blocked { color: var(--blocked); }

      .work-copy {
        margin-top: 6px;
        font-size: 14px;
        color: var(--muted);
      }

      .child-work {
        margin-top: 10px;
        padding-left: 14px;
        border-left: 2px solid rgba(120, 96, 72, 0.16);
      }

      .reference-list,
      .index-list {
        display: flex;
        flex-direction: column;
        gap: 14px;
        margin-top: 18px;
      }

      .reference-item,
      .index-item {
        padding-bottom: 14px;
        border-bottom: 1px solid rgba(120, 96, 72, 0.12);
      }

      .reference-path,
      .index-path {
        margin-top: 4px;
        color: var(--muted);
        font-size: 13px;
        font-family: Inter, ui-sans-serif, system-ui, sans-serif;
      }

      .reference-uri {
        display: block;
        margin-top: 8px;
        color: var(--accent);
        font-size: 13px;
        font-family: Inter, ui-sans-serif, system-ui, sans-serif;
        word-break: break-word;
      }

      .footer-note {
        margin-top: 34px;
        padding-top: 18px;
        border-top: 1px solid var(--rule);
        font-family: Inter, ui-sans-serif, system-ui, sans-serif;
        font-size: 12px;
        color: var(--muted);
      }

      @page {
        size: var(--page-width) var(--page-height);
        margin: var(--page-margin-top) var(--page-margin-right) var(--page-margin-bottom) var(--page-margin-left);
      }

      @media (max-width: 900px) {
        .book-shell {
          display: block;
          max-width: none;
        }

        .book-sidebar {
          position: static;
          height: auto;
          border-right: none;
          border-bottom: 1px solid rgba(120, 96, 72, 0.12);
        }

        .book-main {
          padding: 16px 10px 30px;
        }

        .book {
          width: 100%;
          margin: 0;
        }

        .page {
          padding: 32px 24px;
        }

        .title-page {
          min-height: auto;
        }

        .front-grid,
        .back-grid,
        .section-grid {
          grid-template-columns: 1fr;
        }
      }

      @media print {
        body {
          background: #fff;
        }

        .book-sidebar {
          display: none;
        }

        .book-main {
          padding: 0;
        }

        .book {
          width: 100%;
          max-width: none;
          margin: 0;
          box-shadow: none;
          border: none;
        }

        .page {
          padding: 0;
        }
      }
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

function renderBookContentsHtml(tocTree: BookTocNode[]): string {
  return tocTree.map((node) => renderBookContentsNode(node)).join("");
}

function renderBookContentsNode(node: BookTocNode): string {
  const childrenMarkup = node.children.length > 0
    ? `<div class="toc-children">${node.children.map((child) => renderBookContentsNode(child)).join("")}</div>`
    : "";

  return `
    <div class="toc-group">
      <div class="toc-product_area">
        <a href="#${node.id}" class="inline-link">${escapeHtml(node.title)}</a>
      </div>
      ${childrenMarkup}
    </div>
  `;
}

function renderBookProductAreaHtml(productAreaTree: ProductTree["product_areas"][number], chapterNumber: number, allWorkItems: WorkItem[], references: ProductReference[]): string {
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

function renderBookWorkItemList(nodes: WorkItemNode[]): string {
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

function renderBookReferencesHtml(references: ProductReference[]): string {
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

function filterBookReferences(
  product: Product,
  tree: ProductTree | undefined,
  references: ProductReference[],
): ProductReference[] {
  const scopeLabels = buildReferenceScopeLabels(product, tree);
  return references.filter((reference) => scopeLabels.has(getReferenceScopeKey(reference)));
}

function buildReferenceAtlas(
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

function collectNodeIndex(nodes: HierarchyTreeNode[]): IndexEntry[] {
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

function getTreeNodeSectionId(node: HierarchyTreeNode) {
  return node.node_type === "product_area"
    ? `product-area-${node.id}`
    : `capability-${node.capability_id ?? node.id}`;
}

function buildBookTocItems(
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

function buildBookTocTree(items: ProductOverviewTocItem[]): BookTocNode[] {
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

function renderWebSidebar(productName: string, tocTree: BookTocNode[]) {
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
