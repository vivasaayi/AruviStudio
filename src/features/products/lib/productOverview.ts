import type { Capability, CapabilityTree, Module, ModuleTree, Product, ProductTree, WorkItem } from "../../../lib/types";

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

export function countCapabilities(modules: ModuleTree[]) {
  return modules.reduce((total, moduleTree) => total + moduleTree.features.reduce((sum, capabilityTree) => sum + countCapabilityTree(capabilityTree), 0), 0);
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

export function getModuleSectionId(module: Module) {
  return `module-${module.id}`;
}

export function getCapabilitySectionId(capability: Capability) {
  return `capability-${capability.id}`;
}

export function buildProductOverviewToc(tree: ProductTree | undefined, hasProductLevelWorkItems: boolean): ProductOverviewTocItem[] {
  const items: ProductOverviewTocItem[] = [{ id: PRODUCT_OVERVIEW_TOP_ID, title: "Overview", level: 0 }];

  if (hasProductLevelWorkItems) {
    items.push({ id: PRODUCT_DELIVERY_ID, title: "Product Delivery", level: 0 });
  }

  (tree?.modules ?? []).forEach((moduleTree, index) => {
    items.push({
      id: getModuleSectionId(moduleTree.module),
      title: `${index + 1}. ${moduleTree.module.name}`,
      level: 0,
    });
    appendCapabilityToc(items, moduleTree.features, `${index + 1}`, 1);
  });

  return items;
}

export function buildProductOverviewHtml({
  product,
  tree,
  workItems = [],
}: {
  product: Product;
  tree?: ProductTree;
  workItems?: WorkItem[];
}) {
  const allWorkItems = sortWorkItems(workItems);
  const metrics = buildWorkItemMetrics(allWorkItems);
  const moduleCount = tree?.modules.length ?? 0;
  const capabilityCount = countCapabilities(tree?.modules ?? []);
  const productLevelWorkItems = buildScopedWorkItemTree(allWorkItems.filter((workItem) => !workItem.module_id && !workItem.capability_id));
  const tocItems = buildProductOverviewToc(tree, productLevelWorkItems.length > 0);
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
      }

      .sidebar {
        padding: 28px 22px;
        border-right: 1px solid var(--border);
        background: rgba(255, 255, 255, 0.74);
        backdrop-filter: blur(16px);
      }

      .sidebar-panel {
        position: sticky;
        top: 22px;
        display: flex;
        flex-direction: column;
        gap: 22px;
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
            <p>Generated product documentation with modules, capabilities, and delivery work in one readable view.</p>
          </div>
          <div class="toc">
            <div class="toc-title">On this page</div>
            ${tocItems.map((item) => `<a href="#${item.id}" data-level="${item.level}">${escapeHtml(item.title)}</a>`).join("")}
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
        <section class="hero" id="${PRODUCT_OVERVIEW_TOP_ID}">
          <div class="eyebrow">Product Overview</div>
          <div class="hero-header">
            <div>
              <h2>${escapeHtml(product.name)}</h2>
              <p>${toHtmlParagraph(product.description || "Add a product description in Aruvi Studio so exported docs read like durable product documentation.")}</p>
            </div>
            <div class="hero-chip">${escapeHtml(product.status)}</div>
          </div>
          <div class="progress-panel">
            <div class="progress-label">
              <span>${metrics.done} of ${metrics.total} work items complete</span>
              <strong>${metrics.completion}% complete</strong>
            </div>
            <div class="progress-track"><span style="width: ${metrics.completion}%"></span></div>
          </div>
          <div class="metric-grid">
            ${renderMetricHtml("Modules", moduleCount)}
            ${renderMetricHtml("Capabilities", capabilityCount)}
            ${renderMetricHtml("Done", metrics.done)}
            ${renderMetricHtml("WIP", metrics.wip)}
            ${renderMetricHtml("TBD", metrics.tbd)}
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
        </div>

        ${productLevelWorkItems.length > 0 ? `
          <section class="section" id="${PRODUCT_DELIVERY_ID}">
            <div class="section-header">
              <div class="eyebrow">Product</div>
              <h3>Product Delivery</h3>
              <p>Cross-cutting work attached directly to the product rather than a single module or capability.</p>
            </div>
            <div class="section-body">
              ${renderWorkItemTreeHtml(productLevelWorkItems)}
            </div>
          </section>
        ` : ""}

        ${(tree?.modules ?? []).length > 0
          ? (tree?.modules ?? []).map((moduleTree, index) => renderModuleHtml(moduleTree, index + 1, allWorkItems)).join("")
          : `
            <section class="section">
              <div class="section-header">
                <div class="eyebrow">Product</div>
                <h3>No Modules Yet</h3>
                <p>Create the first module in Aruvi Studio to turn the product into a navigable system map.</p>
              </div>
            </section>
          `}

        <div class="export-note">Generated by Aruvi Studio on ${escapeHtml(generatedAt)}.</div>
      </main>
    </div>
  </body>
</html>`;
}

function appendCapabilityToc(items: ProductOverviewTocItem[], capabilities: CapabilityTree[], prefix: string, level: number) {
  capabilities.forEach((capabilityTree, index) => {
    const numbering = `${prefix}.${index + 1}`;
    items.push({
      id: getCapabilitySectionId(capabilityTree.capability),
      title: `${numbering}. ${capabilityTree.capability.name}`,
      level,
    });
    appendCapabilityToc(items, capabilityTree.children, numbering, level + 1);
  });
}

function collectCapabilityIds(capabilities: CapabilityTree[]): Set<string> {
  const ids = new Set<string>();
  capabilities.forEach((capabilityTree) => {
    ids.add(capabilityTree.capability.id);
    collectCapabilityIds(capabilityTree.children).forEach((id) => ids.add(id));
  });
  return ids;
}

function getModuleScopedWorkItems(moduleTree: ModuleTree, allWorkItems: WorkItem[]) {
  const capabilityIds = collectCapabilityIds(moduleTree.features);
  return allWorkItems.filter(
    (workItem) => workItem.module_id === moduleTree.module.id || (workItem.capability_id ? capabilityIds.has(workItem.capability_id) : false),
  );
}

function getCapabilityScopedWorkItems(capabilityTree: CapabilityTree, allWorkItems: WorkItem[]) {
  const capabilityIds = collectCapabilityIds([capabilityTree]);
  return allWorkItems.filter((workItem) => workItem.capability_id ? capabilityIds.has(workItem.capability_id) : false);
}

function renderModuleHtml(moduleTree: ModuleTree, chapterNumber: number, allWorkItems: WorkItem[]): string {
  const moduleScopedItems = getModuleScopedWorkItems(moduleTree, allWorkItems);
  const moduleMetrics = buildWorkItemMetrics(moduleScopedItems);
  const directModuleWorkItems = buildScopedWorkItemTree(
    allWorkItems.filter((workItem) => workItem.module_id === moduleTree.module.id && !workItem.capability_id),
  );

  return `
    <section class="chapter" id="${getModuleSectionId(moduleTree.module)}">
      <details open>
        <summary>
          <div class="chapter-header">
            <div>
              <div class="chapter-kicker">Module ${chapterNumber}</div>
              <h3>${escapeHtml(moduleTree.module.name)}</h3>
              <p>${toHtmlParagraph(moduleTree.module.description || moduleTree.module.purpose || "Document this module so the product architecture remains readable.")}</p>
            </div>
            <div class="count-row">
              <span class="count-pill">${moduleTree.features.length} ${moduleTree.features.length === 1 ? "capability" : "capabilities"}</span>
              ${renderMetricSummaryPills(moduleMetrics)}
            </div>
          </div>
        </summary>
        <div class="chapter-body">
          ${moduleTree.module.purpose ? `
            <div class="note-card" style="margin-bottom: 16px;">
              <h4>Purpose</h4>
              <p>${toHtmlParagraph(moduleTree.module.purpose)}</p>
            </div>
          ` : ""}
          ${directModuleWorkItems.length > 0 ? `
            <div class="section-header" style="margin-bottom: 12px;">
              <div class="eyebrow">Direct Work</div>
              <h3 style="font-size: 20px;">Module Delivery</h3>
            </div>
            ${renderWorkItemTreeHtml(directModuleWorkItems)}
          ` : ""}
          ${moduleTree.features.length > 0
            ? moduleTree.features.map((capabilityTree, index) => renderCapabilityHtml(capabilityTree, `${chapterNumber}.${index + 1}`, allWorkItems)).join("")
            : `<p class="muted-line">No capabilities defined for this module yet.</p>`}
        </div>
      </details>
    </section>
  `;
}

function renderCapabilityHtml(capabilityTree: CapabilityTree, numbering: string, allWorkItems: WorkItem[]): string {
  const capabilityType = capabilityTree.capability.level === 0 ? "Capability" : "Outcome";
  const scopedItems = getCapabilityScopedWorkItems(capabilityTree, allWorkItems);
  const directWorkItems = buildScopedWorkItemTree(
    allWorkItems.filter((workItem) => workItem.capability_id === capabilityTree.capability.id),
  );
  const metrics = buildWorkItemMetrics(scopedItems);

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
          ${capabilityTree.capability.acceptance_criteria || capabilityTree.capability.technical_notes ? `
            <div class="info-grid">
              ${capabilityTree.capability.acceptance_criteria ? `
                <div class="note-card">
                  <h4>Acceptance Criteria</h4>
                  <p>${toHtmlParagraph(capabilityTree.capability.acceptance_criteria)}</p>
                </div>
              ` : ""}
              ${capabilityTree.capability.technical_notes ? `
                <div class="note-card">
                  <h4>Technical Notes</h4>
                  <p>${toHtmlParagraph(capabilityTree.capability.technical_notes)}</p>
                </div>
              ` : ""}
            </div>
          ` : `<p class="muted-line">No acceptance criteria or technical notes recorded yet.</p>`}

          ${directWorkItems.length > 0 ? `
            <div class="section-header" style="margin: 18px 0 12px;">
              <div class="eyebrow">Delivery</div>
              <h3 style="font-size: 20px;">Work Items</h3>
            </div>
            ${renderWorkItemTreeHtml(directWorkItems)}
          ` : `<p class="muted-line" style="margin-top: 16px;">No work items attached to this ${capabilityType.toLowerCase()} yet.</p>`}

          ${capabilityTree.children.length > 0
            ? capabilityTree.children.map((child, index) => renderCapabilityHtml(child, `${numbering}.${index + 1}`, allWorkItems)).join("")
            : ""}
        </div>
      </details>
    </section>
  `;
}

function renderWorkItemTreeHtml(nodes: WorkItemNode[]): string {
  return `<div class="work-item-list">${nodes.map((node) => renderWorkItemNodeHtml(node)).join("")}</div>`;
}

function renderWorkItemNodeHtml(node: WorkItemNode): string {
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

function renderMetricHtml(label: string, value: number) {
  return `
    <div class="metric">
      <div class="metric-label">${escapeHtml(label)}</div>
      <div class="metric-value">${value}</div>
    </div>
  `;
}

function renderMetricSummaryPills(metrics: WorkItemMetrics) {
  if (metrics.total === 0) {
    return `<span class="count-pill">No work items</span>`;
  }

  return [
    metrics.done > 0 ? `<span class="status-pill is-done">${metrics.done} done</span>` : "",
    metrics.wip > 0 ? `<span class="status-pill is-wip">${metrics.wip} WIP</span>` : "",
    metrics.tbd > 0 ? `<span class="status-pill is-tbd">${metrics.tbd} TBD</span>` : "",
    metrics.blocked > 0 ? `<span class="status-pill is-blocked">${metrics.blocked} blocked</span>` : "",
  ].filter(Boolean).join("");
}

function renderLegendRow(label: string, color: string) {
  return `<div class="legend-row"><span class="legend-dot" style="background: ${color};"></span>${escapeHtml(label)}</div>`;
}

function summarizeText(text: string, maxLength = 220) {
  const normalized = text.replace(/\s+/g, " ").trim();
  if (normalized.length <= maxLength) {
    return normalized;
  }
  return `${normalized.slice(0, maxLength - 1).trimEnd()}…`;
}

function toHtmlParagraph(text: string) {
  return escapeHtml(text).replace(/\n/g, "<br />");
}

function escapeHtml(value: string) {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll("\"", "&quot;")
    .replaceAll("'", "&#39;");
}
