import { countHierarchyNodes, countLeafNodes, getProductDirectWorkItems } from "../../../lib/hierarchyTree";
import type { Product, ProductReference, ProductTree, WorkItem } from "../../../lib/types";
import { filterReferencesForProductBook, filterReferencesForScope } from "./productReferences";
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
      :root {
        color-scheme: light;
        --bg: #f4f7fb;
        --panel: #ffffff;
        --panel-alt: #eef3fb;
        --text: #142033;
        --muted: #5d6c80;
        --border: #d8e1ee;
        --border-strong: #c2d0e3;
        --accent: #2e6ae6;
        --accent-soft: #e9f1ff;
        --shadow: 0 14px 34px rgba(30, 47, 83, 0.08);
        --done: #2e8b57;
        --wip: #b07a08;
        --tbd: #2f70cc;
        --blocked: #be4049;
      }

      * { box-sizing: border-box; }

      html { scroll-behavior: smooth; }

      body {
        margin: 0;
        background: var(--bg);
        color: var(--text);
        font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
      }

      a { color: var(--accent); text-decoration: none; }
      a:hover { text-decoration: underline; }

      .page {
        max-width: 1480px;
        margin: 0 auto;
        display: grid;
        grid-template-columns: 280px minmax(0, 1fr);
        min-height: 100vh;
        transition: grid-template-columns 180ms ease;
      }

      body.sidebar-collapsed .page {
        grid-template-columns: minmax(0, 1fr);
      }

      .sidebar {
        padding: 28px 22px;
        border-right: 1px solid var(--border);
        background: rgba(255, 255, 255, 0.74);
        backdrop-filter: blur(16px);
        transition: opacity 180ms ease, transform 180ms ease;
        min-height: 100vh;
      }

      body.sidebar-collapsed .sidebar {
        display: none;
      }

      .sidebar-panel {
        position: sticky;
        top: 22px;
        display: flex;
        flex-direction: column;
        gap: 22px;
        height: calc(100vh - 44px);
        min-height: 0;
      }

      .brand {
        display: flex;
        flex-direction: column;
        gap: 6px;
      }

      .eyebrow {
        font-size: 11px;
        font-weight: 800;
        letter-spacing: 0.14em;
        text-transform: uppercase;
        color: var(--accent);
      }

      .brand h1 {
        margin: 0;
        font-size: 22px;
        line-height: 1.15;
      }

      .brand p {
        margin: 0;
        font-size: 13px;
        line-height: 1.55;
        color: var(--muted);
      }

      .toc {
        display: flex;
        flex-direction: column;
        gap: 8px;
        min-height: 0;
        flex: 1;
      }

      .toc-scroll {
        overflow-y: auto;
        padding-right: 4px;
        display: flex;
        flex-direction: column;
        gap: 6px;
        flex: 1;
        min-height: 0;
      }

      .toc-title {
        font-size: 12px;
        font-weight: 800;
        letter-spacing: 0.08em;
        text-transform: uppercase;
        color: var(--muted);
      }

      .toc a {
        display: block;
        padding: 8px 10px;
        border-radius: 10px;
        font-size: 13px;
        line-height: 1.4;
        color: #31445e;
      }

      .toc a:hover {
        background: var(--accent-soft);
        text-decoration: none;
      }

      .toc-group {
        border-radius: 12px;
        border: 1px solid var(--border);
        background: rgba(255, 255, 255, 0.78);
        overflow: hidden;
      }

      .toc-group summary {
        list-style: none;
        cursor: pointer;
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 8px;
        padding: 8px 10px;
        color: #31445e;
        font-size: 13px;
        font-weight: 700;
      }

      .toc-group summary::-webkit-details-marker {
        display: none;
      }

      .toc-group-children {
        display: flex;
        flex-direction: column;
        gap: 3px;
        padding: 0 6px 8px;
      }

      .toc a[data-level="1"] { padding-left: 22px; font-size: 12px; }
      .toc a[data-level="2"] { padding-left: 34px; font-size: 12px; }
      .toc a[data-level="3"] { padding-left: 46px; font-size: 12px; }

      .legend {
        display: grid;
        gap: 8px;
      }

      .legend-row {
        display: flex;
        align-items: center;
        gap: 10px;
        font-size: 12px;
        color: var(--muted);
      }

      .legend-dot {
        width: 10px;
        height: 10px;
        border-radius: 999px;
      }

      .content {
        padding: 28px 34px 80px;
      }

      .topbar {
        margin-bottom: 16px;
        display: flex;
        justify-content: flex-end;
      }

      .sidebar-toggle {
        appearance: none;
        border: 1px solid var(--border-strong);
        background: rgba(255, 255, 255, 0.88);
        color: var(--text);
        border-radius: 999px;
        padding: 8px 12px;
        font-size: 12px;
        font-weight: 700;
        cursor: pointer;
        box-shadow: var(--shadow);
      }

      .sidebar-toggle:hover {
        background: #ffffff;
      }

      .hero {
        margin-bottom: 24px;
        padding: 28px;
        border-radius: 24px;
        background:
          radial-gradient(circle at top right, rgba(135, 174, 255, 0.32), transparent 34%),
          linear-gradient(140deg, #173056 0%, #132946 52%, #0f2138 100%);
        color: #f7fbff;
        box-shadow: 0 24px 48px rgba(17, 35, 63, 0.22);
      }

      .hero-header {
        display: flex;
        justify-content: space-between;
        align-items: flex-start;
        gap: 18px;
      }

      .hero-header h2 {
        margin: 0;
        font-size: 34px;
        line-height: 1.05;
      }

      .hero-header p {
        max-width: 760px;
        margin: 10px 0 0;
        font-size: 15px;
        line-height: 1.65;
        color: rgba(232, 242, 255, 0.88);
      }

      .hero-chip {
        display: inline-flex;
        align-items: center;
        gap: 6px;
        border-radius: 999px;
        padding: 8px 12px;
        background: rgba(255, 255, 255, 0.12);
        color: #f6fbff;
        font-size: 12px;
        font-weight: 700;
      }

      .progress-panel {
        margin-top: 22px;
        padding: 14px 16px;
        border-radius: 16px;
        background: rgba(255, 255, 255, 0.08);
        border: 1px solid rgba(255, 255, 255, 0.12);
      }

      .progress-label {
        display: flex;
        justify-content: space-between;
        gap: 12px;
        font-size: 12px;
        color: rgba(232, 242, 255, 0.84);
      }

      .progress-track {
        margin-top: 10px;
        width: 100%;
        height: 10px;
        border-radius: 999px;
        overflow: hidden;
        background: rgba(255, 255, 255, 0.14);
      }

      .progress-track span {
        display: block;
        height: 100%;
        border-radius: inherit;
        background: linear-gradient(90deg, #69d391 0%, #8ef1b9 100%);
      }

      .metric-grid {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(132px, 1fr));
        gap: 12px;
        margin-top: 18px;
      }

      .metric {
        padding: 14px;
        border-radius: 16px;
        background: rgba(255, 255, 255, 0.09);
        border: 1px solid rgba(255, 255, 255, 0.12);
      }

      .metric-label {
        font-size: 11px;
        font-weight: 800;
        text-transform: uppercase;
        letter-spacing: 0.08em;
        color: rgba(240, 247, 255, 0.7);
      }

      .metric-value {
        margin-top: 6px;
        font-size: 28px;
        font-weight: 800;
      }

      .summary-grid {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
        gap: 16px;
        margin-top: 20px;
      }

      .summary-card {
        padding: 18px;
        border-radius: 18px;
        background: var(--panel);
        border: 1px solid var(--border);
        box-shadow: var(--shadow);
      }

      .summary-card h3,
      .section-header h3,
      .chapter-header h3,
      .capability-title {
        margin: 0;
      }

      .section-header {
        display: flex;
        flex-direction: column;
        gap: 6px;
        margin-bottom: 14px;
      }

      .section-header p {
        margin: 0;
        color: var(--muted);
        line-height: 1.55;
      }

      .summary-card p,
      .section-body p {
        margin: 0;
        line-height: 1.7;
        color: var(--muted);
      }

      .summary-card ul {
        margin: 10px 0 0;
        padding-left: 18px;
        color: var(--muted);
      }

      .summary-card li { margin-top: 8px; }

      .tag-row,
      .meta-row,
      .count-row {
        display: flex;
        flex-wrap: wrap;
        gap: 8px;
      }

      .tag,
      .meta-pill,
      .count-pill,
      .status-pill {
        display: inline-flex;
        align-items: center;
        border-radius: 999px;
        padding: 6px 10px;
        font-size: 12px;
      }

      .tag,
      .meta-pill {
        background: var(--accent-soft);
        color: #2550a8;
      }

      .count-pill {
        background: #f0f4fa;
        color: #39506b;
        border: 1px solid var(--border);
      }

      .status-pill {
        font-weight: 700;
        border: 1px solid currentColor;
        background: rgba(255, 255, 255, 0.65);
      }

      .status-pill.is-done { color: var(--done); }
      .status-pill.is-wip { color: var(--wip); }
      .status-pill.is-tbd { color: var(--tbd); }
      .status-pill.is-blocked { color: var(--blocked); }

      .section,
      .chapter,
      .capability {
        margin-top: 18px;
      }

      .section,
      .chapter-body,
      .capability-body {
        padding: 20px;
        border-radius: 20px;
        background: var(--panel);
        border: 1px solid var(--border);
        box-shadow: var(--shadow);
      }

      .chapter details,
      .capability details {
        border-radius: 20px;
        background: var(--panel);
        border: 1px solid var(--border);
        box-shadow: var(--shadow);
      }

      .chapter summary,
      .capability summary {
        cursor: pointer;
        list-style: none;
      }

      .chapter summary::-webkit-details-marker,
      .capability summary::-webkit-details-marker {
        display: none;
      }

      .chapter-header,
      .capability-header {
        display: flex;
        justify-content: space-between;
        align-items: flex-start;
        gap: 16px;
        padding: 20px;
      }

      .chapter-header p,
      .capability-header p {
        margin: 8px 0 0;
        color: var(--muted);
        line-height: 1.6;
      }

      .chapter-kicker {
        font-size: 11px;
        font-weight: 800;
        letter-spacing: 0.14em;
        text-transform: uppercase;
        color: var(--accent);
      }

      .chapter-body,
      .capability-body {
        margin-top: 1px;
        border-top-left-radius: 0;
        border-top-right-radius: 0;
      }

      .info-grid {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
        gap: 14px;
      }

      .note-card {
        padding: 14px 15px;
        border-radius: 16px;
        background: var(--panel-alt);
        border: 1px solid var(--border);
      }

      .note-card h4 {
        margin: 0 0 8px;
        font-size: 12px;
        font-weight: 800;
        letter-spacing: 0.08em;
        text-transform: uppercase;
        color: var(--muted);
      }

      .note-card p {
        margin: 0;
        color: var(--muted);
        line-height: 1.65;
      }

      .reference-grid {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
        gap: 14px;
        margin-top: 14px;
      }

      .reference-card a {
        display: block;
        margin-top: 8px;
        color: var(--accent);
        word-break: break-word;
      }

      .muted-line {
        margin: 0;
        color: var(--muted);
        line-height: 1.6;
      }

      .work-item-list {
        display: flex;
        flex-direction: column;
        gap: 12px;
      }

      .work-item {
        padding: 14px 15px;
        border-radius: 16px;
        border: 1px solid var(--border-strong);
        border-left: 5px solid var(--accent);
        background: #fff;
      }

      .work-item.is-done { border-left-color: var(--done); }
      .work-item.is-wip { border-left-color: var(--wip); }
      .work-item.is-tbd { border-left-color: var(--tbd); }
      .work-item.is-blocked { border-left-color: var(--blocked); }

      .work-item-header {
        display: flex;
        justify-content: space-between;
        align-items: flex-start;
        gap: 12px;
      }

      .work-item-header h5 {
        margin: 0;
        font-size: 15px;
      }

      .work-item p {
        margin: 8px 0 0;
        color: var(--muted);
        line-height: 1.6;
      }

      .work-item-children {
        margin-top: 12px;
        margin-left: 14px;
        padding-left: 14px;
        border-left: 2px solid var(--border);
      }

      .export-note {
        margin-top: 24px;
        font-size: 12px;
        color: var(--muted);
      }

      @media (max-width: 980px) {
        .page { grid-template-columns: 1fr; }
        .sidebar {
          border-right: none;
          border-bottom: 1px solid var(--border);
        }
        .sidebar-panel {
          position: relative;
          top: 0;
        }
        .content {
          padding: 20px;
        }
        .hero,
        .section,
        .chapter-body,
        .capability-body {
          padding: 18px;
        }
        .chapter-header,
        .capability-header,
        .hero-header {
          flex-direction: column;
        }
      }
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
      }

      * { box-sizing: border-box; }

      html { scroll-behavior: smooth; }

      body {
        margin: 0;
        background:
          radial-gradient(circle at top, rgba(255,255,255,0.7), transparent 36%),
          linear-gradient(180deg, #efe8db 0%, var(--paper) 100%);
        color: var(--ink);
        font-family: "Iowan Old Style", "Palatino Linotype", "Book Antiqua", Palatino, Georgia, serif;
      }

      .book {
        width: min(920px, calc(100vw - 40px));
        margin: 40px auto 80px;
        background: rgba(255,255,255,0.72);
        border: 1px solid rgba(120, 96, 72, 0.14);
        box-shadow: 0 24px 60px rgba(64, 46, 31, 0.12);
      }

      .page {
        padding: 58px 72px;
        border-top: 1px solid rgba(120, 96, 72, 0.08);
      }

      .page:first-child {
        border-top: none;
      }

      .title-page {
        min-height: 86vh;
        display: flex;
        flex-direction: column;
        justify-content: center;
      }

      .kicker {
        font-family: Inter, ui-sans-serif, system-ui, sans-serif;
        text-transform: uppercase;
        letter-spacing: 0.18em;
        font-size: 12px;
        font-weight: 800;
        color: var(--accent);
      }

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

      .toc-title,
      .section-kicker,
      .chapter-kicker {
        font-family: Inter, ui-sans-serif, system-ui, sans-serif;
        text-transform: uppercase;
        letter-spacing: 0.14em;
        font-size: 11px;
        font-weight: 800;
        color: var(--accent);
      }

      .toc-title { margin-bottom: 14px; }

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

      .toc-product_area {
        display: flex;
        justify-content: space-between;
        gap: 18px;
        align-items: baseline;
        font-family: Inter, ui-sans-serif, system-ui, sans-serif;
        font-size: 18px;
        font-weight: 700;
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
      }

      .toc-capability {
        display: flex;
        justify-content: space-between;
        gap: 18px;
        align-items: baseline;
        font-size: 14px;
        line-height: 1.45;
        color: var(--muted);
      }

      .section-block {
        margin-top: 18px;
        padding-top: 18px;
        border-top: 1px solid var(--rule);
      }

      .lead,
      .body-copy,
      .note-copy,
      .work-copy {
        line-height: 1.85;
        color: var(--ink);
      }

      .lead {
        font-size: 19px;
        color: var(--muted);
      }

      .body-copy {
        font-size: 17px;
      }

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

      .goal-list li {
        margin-top: 10px;
        line-height: 1.7;
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

      .chapter-stats {
        margin-top: 18px;
        display: flex;
        flex-wrap: wrap;
        gap: 10px;
        font-family: Inter, ui-sans-serif, system-ui, sans-serif;
      }

      .stat-pill {
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

      .capability-meta {
        margin-top: 12px;
        display: flex;
        flex-wrap: wrap;
        gap: 8px;
        font-family: Inter, ui-sans-serif, system-ui, sans-serif;
      }

      .meta-chip {
        display: inline-flex;
        align-items: center;
        border-radius: 999px;
        padding: 5px 10px;
        font-size: 12px;
        background: rgba(240, 225, 212, 0.72);
        color: var(--chapter);
      }

      .note-block {
        margin-top: 18px;
        padding-left: 16px;
        border-left: 3px solid var(--rule);
      }

      .note-label {
        font-family: Inter, ui-sans-serif, system-ui, sans-serif;
        text-transform: uppercase;
        letter-spacing: 0.08em;
        font-size: 11px;
        font-weight: 800;
        color: var(--accent);
      }

      .note-copy {
        margin-top: 8px;
        font-size: 15px;
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

      .page-break {
        break-before: page;
        page-break-before: always;
      }

      .footer-note {
        margin-top: 34px;
        padding-top: 18px;
        border-top: 1px solid var(--rule);
        font-family: Inter, ui-sans-serif, system-ui, sans-serif;
        font-size: 12px;
        color: var(--muted);
      }

      @media (max-width: 900px) {
        .book {
          width: calc(100vw - 20px);
          margin: 10px auto 30px;
        }

        .page {
          padding: 32px 24px;
        }

        .title-page {
          min-height: auto;
        }

        .section-grid {
          grid-template-columns: 1fr;
        }
      }

      @media print {
        body {
          background: #fff;
        }

        .book {
          width: 100%;
          margin: 0;
          box-shadow: none;
          border: none;
        }

        .page {
          padding: 0.7in;
        }
      }
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
