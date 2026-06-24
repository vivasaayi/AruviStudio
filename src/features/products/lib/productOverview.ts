import { countHierarchyNodes, countLeafNodes, getProductDirectWorkItems } from "../../../lib/hierarchyTree";
import type { Product, ProductReference, ProductTree, WorkItem } from "../../../lib/types";
import { filterReferencesForProductBook, filterReferencesForScope } from "./productReferences";
import { PRODUCT_OVERVIEW_BOOK_STYLES, PRODUCT_OVERVIEW_DOCUMENT_STYLES } from "./productOverviewStyles";
import {
  PRODUCT_DELIVERY_ID,
  PRODUCT_OVERVIEW_TOP_ID,
  buildProductOverviewToc,
  buildScopedWorkItemTree,
  buildWorkItemMetrics,
  sortWorkItems,
} from "./productOverviewModel";
import {
  escapeHtml,
  groupTocItems,
  renderBookContentsHtml,
  renderBookProductAreaHtml,
  renderBookWorkItemList,
  renderLegendRow,
  renderMetricHtml,
  renderProductAreaHtml,
  renderReferenceListHtml,
  renderTocGroupHtml,
  renderWorkItemTreeHtml,
  toHtmlParagraph,
} from "./productOverviewRenderHelpers";

export {
  PRODUCT_DELIVERY_ID,
  PRODUCT_OVERVIEW_TOP_ID,
  buildProductOverviewToc,
  buildScopedWorkItemTree,
  buildWorkItemMetrics,
  countCapabilities,
  countCapabilityTree,
  getCapabilitySectionId,
  getProductAreaSectionId,
  getWorkItemPresentation,
  sortWorkItems,
} from "./productOverviewModel";
export type { ProductOverviewTocItem, WorkItemMetrics, WorkItemNode, WorkItemPresentation } from "./productOverviewModel";

export function buildProductOverviewHtml({
  product,
  tree,
  workItems = [],
  references = [],
}: {
  product: Product;
  tree?: ProductTree;
  workItems?: WorkItem[];
  references?: ProductReference[];
}) {
  const allWorkItems = sortWorkItems(workItems);
  const metrics = buildWorkItemMetrics(allWorkItems);
  const rootSectionCount = tree?.roots.length ?? 0;
  const totalNodeCount = tree ? countHierarchyNodes(tree.roots) : 0;
  const leafNodeCount = tree ? countLeafNodes(tree.roots) : 0;
  const activeWorkItemCount = allWorkItems.filter((workItem) => workItem.status !== "done" && workItem.status !== "cancelled").length;
  const productLevelWorkItems = buildScopedWorkItemTree(getProductDirectWorkItems(allWorkItems));
  const bookReferences = filterReferencesForProductBook(product.id, tree, references);
  const productReferences = filterReferencesForScope(bookReferences, { scopeType: "product", scopeId: product.id });
  const tocItems = buildProductOverviewToc(tree, productLevelWorkItems.length > 0);
  const tocGroups = groupTocItems(tocItems);
  const generatedAt = new Date().toLocaleString();

  return `<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>${escapeHtml(product.name)} - Product Overview</title>
    <style>
      ${PRODUCT_OVERVIEW_DOCUMENT_STYLES}
    </style>
  </head>
  <body>
    <div class="page">
      <aside class="sidebar">
        <div class="sidebar-panel">
          <div class="brand">
            <div class="eyebrow">Aruvi Studio</div>
            <h1>${escapeHtml(product.name)}</h1>
            <p>Generated product documentation with product areas, capabilities, features, stories, and tasks in one readable view.</p>
          </div>
          <div class="toc">
            <div class="toc-title">On this page</div>
            <div class="toc-scroll">
              ${tocGroups.map((group) => renderTocGroupHtml(group)).join("")}
            </div>
          </div>
          <div class="legend">
            <div class="toc-title">Work status</div>
            ${renderLegendRow("Done", "var(--done)")}
            ${renderLegendRow("WIP", "var(--wip)")}
            ${renderLegendRow("TBD", "var(--tbd)")}
            ${renderLegendRow("Blocked", "var(--blocked)")}
          </div>
        </div>
      </aside>
      <main class="content">
        <div class="topbar">
          <button type="button" class="sidebar-toggle" id="sidebar-toggle">Hide Sidebar</button>
        </div>
        <section class="hero" id="${PRODUCT_OVERVIEW_TOP_ID}">
          <div class="eyebrow">Product Book</div>
          <div class="hero-header">
            <div>
              <h2>${escapeHtml(product.name)}</h2>
              <p>${toHtmlParagraph(product.description || "Add a product description in Aruvi Studio to anchor the book before coding starts.")}</p>
            </div>
            <div class="hero-chip">${escapeHtml(product.status)}</div>
          </div>
          ${renderReferenceListHtml(productReferences)}
          <div class="progress-panel">
            <div class="progress-label">
              <span>${metrics.done} of ${metrics.total} stories complete</span>
              <strong>${metrics.completion}% complete</strong>
            </div>
            <div class="progress-track"><span style="width: ${metrics.completion}%"></span></div>
          </div>
          <div class="metric-grid">
            ${renderMetricHtml("Product Areas", rootSectionCount)}
            ${renderMetricHtml("Total Nodes", totalNodeCount)}
            ${renderMetricHtml("Leaf Nodes", leafNodeCount)}
            ${renderMetricHtml("Active Stories", activeWorkItemCount)}
            ${renderMetricHtml("Done", metrics.done)}
            ${renderMetricHtml("Blocked", metrics.blocked)}
          </div>
        </section>

        <div class="summary-grid">
          <section class="summary-card">
            <div class="section-header">
              <div class="eyebrow">Direction</div>
              <h3>Vision</h3>
            </div>
            <p>${toHtmlParagraph(product.vision || "No product vision recorded yet.")}</p>
          </section>
          <section class="summary-card">
            <div class="section-header">
              <div class="eyebrow">Intent</div>
              <h3>Goals</h3>
            </div>
            ${product.goals.length > 0 ? `<ul>${product.goals.map((goal) => `<li>${escapeHtml(goal)}</li>`).join("")}</ul>` : `<p>No goals recorded yet.</p>`}
          </section>
          <section class="summary-card">
            <div class="section-header">
              <div class="eyebrow">Metadata</div>
              <h3>Tags</h3>
            </div>
            ${product.tags.length > 0 ? `<div class="tag-row">${product.tags.map((tag) => `<span class="tag">${escapeHtml(tag)}</span>`).join("")}</div>` : `<p>No product tags recorded yet.</p>`}
          </section>
          <section class="summary-card">
            <div class="section-header">
              <div class="eyebrow">Evidence</div>
              <h3>References</h3>
            </div>
            <p>${bookReferences.length} scoped ${bookReferences.length === 1 ? "reference" : "references"} attached to this product book.</p>
          </section>
        </div>

        ${productLevelWorkItems.length > 0 ? `
          <section class="section" id="${PRODUCT_DELIVERY_ID}">
            <div class="section-header">
              <div class="eyebrow">Product</div>
              <h3>Product Delivery</h3>
              <p>Cross-cutting work attached directly to the book rather than a single chapter or feature.</p>
            </div>
            <div class="section-body">
              ${renderWorkItemTreeHtml(productLevelWorkItems)}
            </div>
          </section>
        ` : ""}

        ${(tree?.product_areas ?? []).length > 0
          ? (tree?.product_areas ?? []).map((productAreaTree, index) => renderProductAreaHtml(productAreaTree, index + 1, allWorkItems, bookReferences)).join("")
          : `
            <section class="section">
              <div class="section-header">
                <div class="eyebrow">Product</div>
                <h3>No Product Areas Yet</h3>
                <p>Create the first product area in Aruvi Studio to turn the product into a navigable system map.</p>
              </div>
            </section>
          `}

        <div class="export-note">Generated by Aruvi Studio on ${escapeHtml(generatedAt)}.</div>
      </main>
    </div>
    <script>
      (function () {
        const button = document.getElementById("sidebar-toggle");
        if (!button) return;

        const syncLabel = function () {
          button.textContent = document.body.classList.contains("sidebar-collapsed")
            ? "Show Sidebar"
            : "Hide Sidebar";
        };

        button.addEventListener("click", function () {
          document.body.classList.toggle("sidebar-collapsed");
          syncLabel();
        });

        syncLabel();
      })();
    </script>
  </body>
</html>`;
}
export function buildProductOverviewBookHtml({
  product,
  tree,
  workItems = [],
}: {
  product: Product;
  tree?: ProductTree;
  workItems?: WorkItem[];
}) {
  const allWorkItems = sortWorkItems(workItems);
  const rootSectionCount = tree?.roots.length ?? 0;
  const totalNodeCount = tree ? countHierarchyNodes(tree.roots) : 0;
  const metrics = buildWorkItemMetrics(allWorkItems);
  const productLevelWorkItems = buildScopedWorkItemTree(
    getProductDirectWorkItems(allWorkItems),
  );
  const generatedAt = new Date().toLocaleString();

  return `<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>${escapeHtml(product.name)} - Book</title>
    <style>
      ${PRODUCT_OVERVIEW_BOOK_STYLES}
    </style>
  </head>
  <body>
    <article class="book">
      <section class="page title-page" id="${PRODUCT_OVERVIEW_TOP_ID}">
        <div class="kicker">Aruvi Studio Book</div>
        <h1>${escapeHtml(product.name)}</h1>
        <div class="deck">${toHtmlParagraph(product.description || "A durable product narrative generated from Aruvi Studio.")}</div>
        <div class="book-meta">
          <div class="meta-item"><strong>Product Areas</strong>${rootSectionCount}</div>
          <div class="meta-item"><strong>Total Nodes</strong>${totalNodeCount}</div>
          <div class="meta-item"><strong>Delivery</strong>${metrics.done} done, ${metrics.wip} active, ${metrics.tbd} planned</div>
          <div class="meta-item"><strong>Generated</strong>${escapeHtml(generatedAt)}</div>
        </div>
      </section>

      <section class="page">
        <div class="toc-title">Contents</div>
        <div class="lead">This edition keeps the product tree readable as a narrative: direction first, then product areas, capabilities, and features, with delivery stories shown only as concise implementation notes.</div>
        <div class="section-grid">
          <div class="panel">
            <h3>Vision</h3>
            <div class="body-copy">${toHtmlParagraph(product.vision || "No product vision recorded yet.")}</div>
          </div>
          <div class="panel">
            <h3>Goals</h3>
            ${product.goals.length > 0 ? `<ol class="goal-list">${product.goals.map((goal) => `<li>${escapeHtml(goal)}</li>`).join("")}</ol>` : `<div class="body-copy">No goals recorded yet.</div>`}
          </div>
        </div>
        <div class="toc-list">
          ${renderBookContentsHtml(tree, productLevelWorkItems.length > 0)}
        </div>
      </section>

      ${productLevelWorkItems.length > 0 ? `
        <section class="page">
          <div class="chapter-kicker">Prelude</div>
          <h2 class="chapter-title">Product Delivery Themes</h2>
          <div class="chapter-intro">Cross-cutting work attached directly to the product, shown here as implementation themes rather than detailed execution records.</div>
          ${renderBookWorkItemList(productLevelWorkItems)}
        </section>
      ` : ""}

      ${(tree?.product_areas ?? []).length > 0
        ? (tree?.product_areas ?? []).map((productAreaTree, index) => renderBookProductAreaHtml(productAreaTree, index + 1, allWorkItems)).join("")
        : `
          <section class="page">
            <div class="chapter-kicker">Catalog</div>
            <h2 class="chapter-title">No Product Areas Yet</h2>
            <div class="chapter-intro">Create the first product area in Aruvi Studio to turn this product into a readable book.</div>
          </section>
        `}
    </article>
  </body>
</html>`;
}
