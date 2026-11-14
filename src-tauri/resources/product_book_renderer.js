"use strict";
var AruviProductBook = (() => {
  var __defProp = Object.defineProperty;
  var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
  var __getOwnPropNames = Object.getOwnPropertyNames;
  var __hasOwnProp = Object.prototype.hasOwnProperty;
  var __export = (target, all) => {
    for (var name in all)
      __defProp(target, name, { get: all[name], enumerable: true });
  };
  var __copyProps = (to, from, except, desc) => {
    if (from && typeof from === "object" || typeof from === "function") {
      for (let key of __getOwnPropNames(from))
        if (!__hasOwnProp.call(to, key) && key !== except)
          __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
    }
    return to;
  };
  var __toCommonJS = (mod) => __copyProps(__defProp({}, "__esModule", { value: true }), mod);

  // src/features/products/lib/productBookBackend.ts
  var productBookBackend_exports = {};
  __export(productBookBackend_exports, {
    renderJson: () => renderJson
  });

  // src/lib/hierarchyTree.ts
  function getWorkItemOwnerKey(workItem) {
    if (workItem.source_node_id && workItem.source_node_type) {
      return `${workItem.source_node_type}:${workItem.source_node_id}`;
    }
    if (workItem.capability_id) {
      return `capability:${workItem.capability_id}`;
    }
    if (workItem.product_area_id) {
      return `product_area:${workItem.product_area_id}`;
    }
    return "product";
  }
  function isDirectProductWorkItem(workItem) {
    return getWorkItemOwnerKey(workItem) === "product";
  }
  function flattenHierarchyNodes(roots) {
    const nodes = [];
    const visit = (node) => {
      nodes.push(node);
      node.children.forEach(visit);
    };
    roots.forEach(visit);
    return nodes;
  }
  function countHierarchyNodes(roots) {
    return flattenHierarchyNodes(roots).length;
  }
  function countLeafNodes(roots) {
    return flattenHierarchyNodes(roots).filter((node) => node.children.length === 0).length;
  }
  function getHierarchyNodeSectionId(node) {
    if (!node) {
      return "product-overview-top";
    }
    if (node.node_type === "product_area") {
      return `product-area-${node.id}`;
    }
    return `${node.node_type}-${node.id}`;
  }
  function getProductDirectWorkItems(workItems) {
    return workItems.filter(isDirectProductWorkItem);
  }

  // src/features/products/lib/productReferences.ts
  function getProductAreaReferenceScope(productAreaId) {
    return { scopeType: "product_area", scopeId: productAreaId };
  }
  function getCapabilityReferenceScope(capability) {
    return {
      scopeType: capability.node_kind === "feature" ? "feature" : "capability",
      scopeId: capability.id
    };
  }
  function filterReferencesForScope(references, scope) {
    return references.filter(
      (reference) => reference.scope_type === scope.scopeType && reference.scope_id === scope.scopeId
    );
  }
  function filterReferencesForProductBook(productId, tree, references) {
    const scopeKeys = /* @__PURE__ */ new Set([`product:${productId}`]);
    const visit = (node) => {
      if (node.node_type === "product_area") {
        scopeKeys.add(`product_area:${node.id}`);
      } else if (node.capability_id) {
        scopeKeys.add(`${node.node_kind === "feature" ? "feature" : "capability"}:${node.capability_id}`);
      }
      node.children.forEach(visit);
    };
    (tree?.roots ?? []).forEach(visit);
    return references.filter((reference) => scopeKeys.has(`${reference.scope_type}:${reference.scope_id}`));
  }
  function getReferenceKindLabel(kind) {
    return kind.split("_").map((part) => part.charAt(0).toUpperCase() + part.slice(1)).join(" ");
  }

  // src/features/products/lib/productOverviewStyles.ts
  var PRODUCT_OVERVIEW_DOCUMENT_STYLES = ':root{color-scheme:light;--bg:#f4f7fb;--panel:#ffffff;--panel-alt:#eef3fb;--text:#142033;--muted:#5d6c80;--border:#d8e1ee;--border-strong:#c2d0e3;--accent:#2e6ae6;--accent-soft:#e9f1ff;--shadow:0 14px 34px rgba(30,47,83,0.08);--done:#2e8b57;--wip:#b07a08;--tbd:#2f70cc;--blocked:#be4049}*{box-sizing:border-box}html{scroll-behavior:smooth}body{margin:0;background:var(--bg);color:var(--text);font-family:Inter,ui-sans-serif,system-ui,-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif}a{color:var(--accent);text-decoration:none}a:hover{text-decoration:underline}.page{max-width:1480px;margin:0 auto;display:grid;grid-template-columns:280px minmax(0,1fr);min-height:100vh;transition:grid-template-columns 180ms ease}body.sidebar-collapsed .page{grid-template-columns:minmax(0,1fr)}.sidebar{padding:28px 22px;border-right:1px solid var(--border);background:rgba(255,255,255,0.74);backdrop-filter:blur(16px);transition:opacity 180ms ease,transform 180ms ease;min-height:100vh}body.sidebar-collapsed .sidebar{display:none}.sidebar-panel{position:sticky;top:22px;display:flex;flex-direction:column;gap:22px;height:calc(100vh - 44px);min-height:0}.brand{display:flex;flex-direction:column;gap:6px}.eyebrow{font-size:11px;font-weight:800;letter-spacing:0.14em;text-transform:uppercase;color:var(--accent)}.brand h1{margin:0;font-size:22px;line-height:1.15}.brand p{margin:0;font-size:13px;line-height:1.55;color:var(--muted)}.toc{display:flex;flex-direction:column;gap:8px;min-height:0;flex:1}.toc-scroll{overflow-y:auto;padding-right:4px;display:flex;flex-direction:column;gap:6px;flex:1;min-height:0}.toc-title{font-size:12px;font-weight:800;letter-spacing:0.08em;text-transform:uppercase;color:var(--muted)}.toc a{display:block;padding:8px 10px;border-radius:10px;font-size:13px;line-height:1.4;color:#31445e}.toc a:hover{background:var(--accent-soft);text-decoration:none}.toc-group{border-radius:12px;border:1px solid var(--border);background:rgba(255,255,255,0.78);overflow:hidden}.toc-group summary{list-style:none;cursor:pointer;display:flex;align-items:center;justify-content:space-between;gap:8px;padding:8px 10px;color:#31445e;font-size:13px;font-weight:700}.toc-group summary::-webkit-details-marker{display:none}.toc-group-children{display:flex;flex-direction:column;gap:3px;padding:0 6px 8px}.toc a[data-level="1"]{padding-left:22px;font-size:12px}.toc a[data-level="2"]{padding-left:34px;font-size:12px}.toc a[data-level="3"]{padding-left:46px;font-size:12px}.legend{display:grid;gap:8px}.legend-row{display:flex;align-items:center;gap:10px;font-size:12px;color:var(--muted)}.legend-dot{width:10px;height:10px;border-radius:999px}.content{padding:28px 34px 80px}.topbar{margin-bottom:16px;display:flex;justify-content:flex-end}.sidebar-toggle{appearance:none;border:1px solid var(--border-strong);background:rgba(255,255,255,0.88);color:var(--text);border-radius:999px;padding:8px 12px;font-size:12px;font-weight:700;cursor:pointer;box-shadow:var(--shadow)}.sidebar-toggle:hover{background:#ffffff}.hero{margin-bottom:24px;padding:28px;border-radius:24px;background:radial-gradient(circle at top right,rgba(135,174,255,0.32),transparent 34%),linear-gradient(140deg,#173056 0%,#132946 52%,#0f2138 100%);color:#f7fbff;box-shadow:0 24px 48px rgba(17,35,63,0.22)}.hero-header{display:flex;justify-content:space-between;align-items:flex-start;gap:18px}.hero-header h2{margin:0;font-size:34px;line-height:1.05}.hero-header p{max-width:760px;margin:10px 0 0;font-size:15px;line-height:1.65;color:rgba(232,242,255,0.88)}.hero-chip{display:inline-flex;align-items:center;gap:6px;border-radius:999px;padding:8px 12px;background:rgba(255,255,255,0.12);color:#f6fbff;font-size:12px;font-weight:700}.progress-panel{margin-top:22px;padding:14px 16px;border-radius:16px;background:rgba(255,255,255,0.08);border:1px solid rgba(255,255,255,0.12)}.progress-label{display:flex;justify-content:space-between;gap:12px;font-size:12px;color:rgba(232,242,255,0.84)}.progress-track{margin-top:10px;width:100%;height:10px;border-radius:999px;overflow:hidden;background:rgba(255,255,255,0.14)}.progress-track span{display:block;height:100%;border-radius:inherit;background:linear-gradient(90deg,#69d391 0%,#8ef1b9 100%)}.metric-grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(132px,1fr));gap:12px;margin-top:18px}.metric{padding:14px;border-radius:16px;background:rgba(255,255,255,0.09);border:1px solid rgba(255,255,255,0.12)}.metric-label{font-size:11px;font-weight:800;text-transform:uppercase;letter-spacing:0.08em;color:rgba(240,247,255,0.7)}.metric-value{margin-top:6px;font-size:28px;font-weight:800}.summary-grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(260px,1fr));gap:16px;margin-top:20px}.summary-card{padding:18px;border-radius:18px;background:var(--panel);border:1px solid var(--border);box-shadow:var(--shadow)}.summary-card h3,.section-header h3,.chapter-header h3,.capability-title{margin:0}.section-header{display:flex;flex-direction:column;gap:6px;margin-bottom:14px}.section-header p{margin:0;color:var(--muted);line-height:1.55}.summary-card p,.section-body p{margin:0;line-height:1.7;color:var(--muted)}.summary-card ul{margin:10px 0 0;padding-left:18px;color:var(--muted)}.summary-card li{margin-top:8px}.tag-row,.meta-row,.count-row{display:flex;flex-wrap:wrap;gap:8px}.tag,.meta-pill,.count-pill,.status-pill{display:inline-flex;align-items:center;border-radius:999px;padding:6px 10px;font-size:12px}.tag,.meta-pill{background:var(--accent-soft);color:#2550a8}.count-pill{background:#f0f4fa;color:#39506b;border:1px solid var(--border)}.status-pill{font-weight:700;border:1px solid currentColor;background:rgba(255,255,255,0.65)}.status-pill.is-done{color:var(--done)}.status-pill.is-wip{color:var(--wip)}.status-pill.is-tbd{color:var(--tbd)}.status-pill.is-blocked{color:var(--blocked)}.section,.chapter,.capability{margin-top:18px}.section,.chapter-body,.capability-body{padding:20px;border-radius:20px;background:var(--panel);border:1px solid var(--border);box-shadow:var(--shadow)}.chapter details,.capability details{border-radius:20px;background:var(--panel);border:1px solid var(--border);box-shadow:var(--shadow)}.chapter summary,.capability summary{cursor:pointer;list-style:none}.chapter summary::-webkit-details-marker,.capability summary::-webkit-details-marker{display:none}.chapter-header,.capability-header{display:flex;justify-content:space-between;align-items:flex-start;gap:16px;padding:20px}.chapter-header p,.capability-header p{margin:8px 0 0;color:var(--muted);line-height:1.6}.chapter-kicker{font-size:11px;font-weight:800;letter-spacing:0.14em;text-transform:uppercase;color:var(--accent)}.chapter-body,.capability-body{margin-top:1px;border-top-left-radius:0;border-top-right-radius:0}.info-grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(240px,1fr));gap:14px}.note-card{padding:14px 15px;border-radius:16px;background:var(--panel-alt);border:1px solid var(--border)}.note-card h4{margin:0 0 8px;font-size:12px;font-weight:800;letter-spacing:0.08em;text-transform:uppercase;color:var(--muted)}.note-card p{margin:0;color:var(--muted);line-height:1.65}.reference-grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(240px,1fr));gap:14px;margin-top:14px}.reference-card a{display:block;margin-top:8px;color:var(--accent);word-break:break-word}.muted-line{margin:0;color:var(--muted);line-height:1.6}.work-item-list{display:flex;flex-direction:column;gap:12px}.work-item{padding:14px 15px;border-radius:16px;border:1px solid var(--border-strong);border-left:5px solid var(--accent);background:#fff}.work-item.is-done{border-left-color:var(--done)}.work-item.is-wip{border-left-color:var(--wip)}.work-item.is-tbd{border-left-color:var(--tbd)}.work-item.is-blocked{border-left-color:var(--blocked)}.work-item-header{display:flex;justify-content:space-between;align-items:flex-start;gap:12px}.work-item-header h5{margin:0;font-size:15px}.work-item p{margin:8px 0 0;color:var(--muted);line-height:1.6}.work-item-children{margin-top:12px;margin-left:14px;padding-left:14px;border-left:2px solid var(--border)}.export-note{margin-top:24px;font-size:12px;color:var(--muted)}@media (max-width:980px){.page{grid-template-columns:1fr}.sidebar{border-right:none;border-bottom:1px solid var(--border)}.sidebar-panel{position:relative;top:0}.content{padding:20px}.hero,.section,.chapter-body,.capability-body{padding:18px}.chapter-header,.capability-header,.hero-header{flex-direction:column}}';

  // src/features/products/lib/productOverviewModel.ts
  var PRODUCT_OVERVIEW_TOP_ID = "product-overview-top";
  var PRODUCT_DELIVERY_ID = "product-delivery";
  function sortWorkItems(workItems) {
    return [...workItems].sort(
      (left, right) => left.sort_order - right.sort_order || left.title.localeCompare(right.title) || left.created_at.localeCompare(right.created_at)
    );
  }
  function buildScopedWorkItemTree(workItems) {
    if (workItems.length === 0) {
      return [];
    }
    const sortedItems = sortWorkItems(workItems);
    const itemIds = new Set(sortedItems.map((workItem) => workItem.id));
    const childrenByParent = /* @__PURE__ */ new Map();
    sortedItems.forEach((workItem) => {
      if (workItem.parent_work_item_id && itemIds.has(workItem.parent_work_item_id)) {
        const siblings = childrenByParent.get(workItem.parent_work_item_id) ?? [];
        siblings.push(workItem);
        childrenByParent.set(workItem.parent_work_item_id, siblings);
      }
    });
    const roots = sortedItems.filter(
      (workItem) => !workItem.parent_work_item_id || !itemIds.has(workItem.parent_work_item_id)
    );
    const materialize = (workItem) => ({
      workItem,
      children: (childrenByParent.get(workItem.id) ?? []).map(materialize)
    });
    return roots.map(materialize);
  }
  function buildWorkItemMetrics(workItems) {
    const totals = workItems.reduce(
      (accumulator, workItem) => {
        const bucket = getWorkItemPresentation(workItem.status).bucket;
        if (bucket === "done") accumulator.done += 1;
        else if (bucket === "wip") accumulator.wip += 1;
        else if (bucket === "blocked") accumulator.blocked += 1;
        else accumulator.tbd += 1;
        return accumulator;
      },
      { done: 0, wip: 0, tbd: 0, blocked: 0 }
    );
    const total = totals.done + totals.wip + totals.tbd + totals.blocked;
    const completion = total === 0 ? 0 : Math.round(totals.done / total * 100);
    return { ...totals, total, completion };
  }
  function getWorkItemPresentation(status) {
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
          badgeColor: "#a8f4d0"
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
          badgeColor: "#ffe8a8"
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
          badgeColor: "#ffbfc3"
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
          badgeColor: "#b9d9ff"
        };
    }
  }
  function getProductAreaSectionId(product_area) {
    return `product-area-${product_area.id}`;
  }
  function getCapabilitySectionId(capability) {
    return `capability-${capability.id}`;
  }
  function buildProductOverviewToc(tree, hasProductLevelWorkItems) {
    const items = [{ id: PRODUCT_OVERVIEW_TOP_ID, title: "Overview", level: 0 }];
    if (hasProductLevelWorkItems) {
      items.push({ id: PRODUCT_DELIVERY_ID, title: "Product Delivery", level: 0 });
    }
    appendHierarchyToc(items, tree?.roots ?? [], "", 0);
    return items;
  }
  function appendHierarchyToc(items, nodes, prefix, level) {
    nodes.forEach((node, index) => {
      const numbering = prefix ? `${prefix}.${index + 1}` : `${index + 1}`;
      items.push({
        id: getHierarchyNodeSectionId(node),
        title: `${numbering}. ${node.name}`,
        level
      });
      appendHierarchyToc(items, node.children, numbering, level + 1);
    });
  }

  // src/lib/hierarchyLabels.ts
  var NODE_KIND_LABELS = {
    product_area: {
      singular: "Product Area",
      plural: "Product Areas",
      singularLower: "product area",
      pluralLower: "product areas"
    },
    capability: {
      singular: "Capability",
      plural: "Capabilities",
      singularLower: "capability",
      pluralLower: "capabilities"
    },
    feature: {
      singular: "Feature",
      plural: "Features",
      singularLower: "feature",
      pluralLower: "features"
    }
  };
  function getHierarchyNodeKindLabel(nodeKind, options = {}) {
    const labels = nodeKind ? NODE_KIND_LABELS[nodeKind] : void 0;
    if (!labels) {
      if (options.lowercase) {
        return options.plural ? "nodes" : "node";
      }
      return options.plural ? "Nodes" : "Node";
    }
    if (options.lowercase) {
      return options.plural ? labels.pluralLower : labels.singularLower;
    }
    return options.plural ? labels.plural : labels.singular;
  }

  // src/features/products/lib/productOverviewRenderHelpers.ts
  function groupTocItems(items) {
    const groups = [];
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
  function collectCapabilityIds(capabilities) {
    const ids = /* @__PURE__ */ new Set();
    capabilities.forEach((capabilityTree) => {
      ids.add(capabilityTree.capability.id);
      collectCapabilityIds(capabilityTree.children).forEach((id) => ids.add(id));
    });
    return ids;
  }
  function getProductAreaScopedWorkItems(productAreaTree, allWorkItems) {
    const capabilityIds = collectCapabilityIds(productAreaTree.features);
    return allWorkItems.filter(
      (workItem) => workItem.product_area_id === productAreaTree.product_area.id || (workItem.capability_id ? capabilityIds.has(workItem.capability_id) : false)
    );
  }
  function getCapabilityScopedWorkItems(capabilityTree, allWorkItems) {
    const capabilityIds = collectCapabilityIds([capabilityTree]);
    return allWorkItems.filter((workItem) => workItem.capability_id ? capabilityIds.has(workItem.capability_id) : false);
  }
  function renderProductAreaHtml(productAreaTree, chapterNumber, allWorkItems, references) {
    const productAreaScopedItems = getProductAreaScopedWorkItems(productAreaTree, allWorkItems);
    const productAreaMetrics = buildWorkItemMetrics(productAreaScopedItems);
    const directProductAreaWorkItems = buildScopedWorkItemTree(
      allWorkItems.filter((workItem) => workItem.product_area_id === productAreaTree.product_area.id && !workItem.capability_id)
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
          ${productAreaTree.features.length > 0 ? productAreaTree.features.map((capabilityTree, index) => renderCapabilityHtml(capabilityTree, `${chapterNumber}.${index + 1}`, allWorkItems, references)).join("") : `<p class="muted-line">No capabilities defined for this product area yet.</p>`}
        </div>
      </details>
    </section>
  `;
  }
  function renderCapabilityHtml(capabilityTree, numbering, allWorkItems, references) {
    const capabilityType = getHierarchyNodeKindLabel(capabilityTree.capability.node_kind);
    const scopedItems = getCapabilityScopedWorkItems(capabilityTree, allWorkItems);
    const directWorkItems = buildScopedWorkItemTree(
      allWorkItems.filter((workItem) => workItem.capability_id === capabilityTree.capability.id)
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

          ${capabilityTree.children.length > 0 ? capabilityTree.children.map((child, index) => renderCapabilityHtml(child, `${numbering}.${index + 1}`, allWorkItems, references)).join("") : ""}
        </div>
      </details>
    </section>
  `;
  }
  function renderNoteCardHtml(label, value) {
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
  function renderReferenceListHtml(references) {
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
  function renderWorkItemTreeHtml(nodes) {
    return `<div class="work-item-list">${nodes.map((node) => renderWorkItemNodeHtml(node)).join("")}</div>`;
  }
  function renderWorkItemNodeHtml(node) {
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
  function renderMetricHtml(label, value) {
    return `
    <div class="metric">
      <div class="metric-label">${escapeHtml(label)}</div>
      <div class="metric-value">${value}</div>
    </div>
  `;
  }
  function renderTocGroupHtml(group) {
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
  function renderMetricSummaryPills(metrics) {
    if (metrics.total === 0) {
      return `<span class="count-pill">No stories</span>`;
    }
    return [
      metrics.done > 0 ? `<span class="status-pill is-done">${metrics.done} done</span>` : "",
      metrics.wip > 0 ? `<span class="status-pill is-wip">${metrics.wip} WIP</span>` : "",
      metrics.tbd > 0 ? `<span class="status-pill is-tbd">${metrics.tbd} TBD</span>` : "",
      metrics.blocked > 0 ? `<span class="status-pill is-blocked">${metrics.blocked} blocked</span>` : ""
    ].filter(Boolean).join("");
  }
  function renderLegendRow(label, color) {
    return `<div class="legend-row"><span class="legend-dot" style="background: ${color};"></span>${escapeHtml(label)}</div>`;
  }
  function summarizeText(text, maxLength = 220) {
    const normalized = text.replace(/\s+/g, " ").trim();
    if (normalized.length <= maxLength) {
      return normalized;
    }
    return `${normalized.slice(0, maxLength - 1).trimEnd()}\u2026`;
  }
  function toHtmlParagraph(text) {
    return escapeHtml(text).replace(/\n/g, "<br />");
  }
  function escapeHtml(value) {
    return value.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;").replaceAll('"', "&quot;").replaceAll("'", "&#39;");
  }

  // src/features/products/lib/productOverview.ts
  function buildProductOverviewHtml({
    product,
    tree,
    workItems = [],
    references = []
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
    const generatedAt = (/* @__PURE__ */ new Date()).toLocaleString();
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

        ${(tree?.product_areas ?? []).length > 0 ? (tree?.product_areas ?? []).map((productAreaTree, index) => renderProductAreaHtml(productAreaTree, index + 1, allWorkItems, bookReferences)).join("") : `
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

  // src/features/products/lib/bookExportTextRendering.ts
  function renderRichTextHtml(text) {
    const normalized = text.replace(/\r\n/g, "\n").trim();
    if (!normalized) {
      return "";
    }
    const lines = normalized.split("\n");
    const blocks = [];
    let index = 0;
    while (index < lines.length) {
      const line = lines[index];
      const trimmed = line.trim();
      if (!trimmed) {
        index += 1;
        continue;
      }
      if (trimmed.startsWith("```")) {
        const language = trimmed.slice(3).trim();
        index += 1;
        const codeLines = [];
        while (index < lines.length && !lines[index].trim().startsWith("```")) {
          codeLines.push(lines[index]);
          index += 1;
        }
        if (index < lines.length) {
          index += 1;
        }
        blocks.push(`
        <figure class="book-code">
          <pre><code>${escapeHtml2(codeLines.join("\n"))}</code></pre>
          ${language ? `<figcaption>${escapeHtml2(language)} snippet</figcaption>` : ""}
        </figure>
      `);
        continue;
      }
      if (isFigureLine(trimmed)) {
        blocks.push(renderFigureBlock(trimmed));
        index += 1;
        continue;
      }
      if (isTableLine(trimmed)) {
        const tableLines = [];
        while (index < lines.length && isTableLine(lines[index].trim())) {
          tableLines.push(lines[index].trim());
          index += 1;
        }
        blocks.push(renderTableBlock(tableLines));
        continue;
      }
      if (/^[-*]\s+/.test(trimmed)) {
        const items = [];
        while (index < lines.length && /^[-*]\s+/.test(lines[index].trim())) {
          items.push(lines[index].trim().replace(/^[-*]\s+/, ""));
          index += 1;
        }
        blocks.push(`<ul class="book-list">${items.map((item) => `<li>${renderInlineRichText(item)}</li>`).join("")}</ul>`);
        continue;
      }
      if (/^\d+\.\s+/.test(trimmed)) {
        const items = [];
        while (index < lines.length && /^\d+\.\s+/.test(lines[index].trim())) {
          items.push(lines[index].trim().replace(/^\d+\.\s+/, ""));
          index += 1;
        }
        blocks.push(`<ol class="book-list">${items.map((item) => `<li>${renderInlineRichText(item)}</li>`).join("")}</ol>`);
        continue;
      }
      if (/^>\s?/.test(trimmed)) {
        const quoteLines = [];
        while (index < lines.length && /^>\s?/.test(lines[index].trim())) {
          quoteLines.push(lines[index].trim().replace(/^>\s?/, ""));
          index += 1;
        }
        blocks.push(`<blockquote class="book-quote">${quoteLines.map((item) => `<p class="book-paragraph">${renderInlineRichText(item)}</p>`).join("")}</blockquote>`);
        continue;
      }
      const paragraphLines = [];
      while (index < lines.length && lines[index].trim() && !lines[index].trim().startsWith("```") && !isFigureLine(lines[index].trim()) && !isTableLine(lines[index].trim()) && !/^[-*]\s+/.test(lines[index].trim()) && !/^\d+\.\s+/.test(lines[index].trim()) && !/^>\s?/.test(lines[index].trim())) {
        paragraphLines.push(lines[index].trim());
        index += 1;
      }
      blocks.push(`<p class="book-paragraph">${renderInlineRichText(paragraphLines.join(" "))}</p>`);
    }
    return blocks.join("");
  }
  function renderInlineRichText(text) {
    return escapeHtml2(text).replace(/`([^`]+)`/g, (_match, code) => `<code class="book-inline-code">${escapeHtml2(code)}</code>`);
  }
  function renderFigureBlock(line) {
    const match = line.match(/^!\[(.*?)\]\((\S+?)(?:\s+"(.*)")?\)$/);
    if (!match) {
      return `<p class="book-paragraph">${renderInlineRichText(line)}</p>`;
    }
    const [, alt, src, title] = match;
    const caption = title || alt || "Figure";
    return `
    <figure class="book-figure">
      <img src="${escapeHtml2(src)}" alt="${escapeHtml2(alt || caption)}" />
      <figcaption>${escapeHtml2(caption)}</figcaption>
    </figure>
  `;
  }
  function renderTableBlock(lines) {
    if (lines.length < 2) {
      return `<p class="book-paragraph">${renderInlineRichText(lines.join(" "))}</p>`;
    }
    const rows = lines.map((line) => line.trim().replace(/^\||\|$/g, "").split("|").map((cell) => cell.trim())).filter((row) => row.length > 0);
    if (rows.length < 2) {
      return `<p class="book-paragraph">${renderInlineRichText(lines.join(" "))}</p>`;
    }
    const [header, ...body] = rows;
    const bodyRows = body.filter((row, index) => !(index === 0 && row.every((cell) => /^:?-{2,}:?$/.test(cell))));
    return `
    <figure class="book-table-wrap">
      <table class="book-table">
        <thead>
          <tr>${header.map((cell) => `<th>${renderInlineRichText(cell)}</th>`).join("")}</tr>
        </thead>
        <tbody>
          ${bodyRows.map((row) => `<tr>${row.map((cell) => `<td>${renderInlineRichText(cell)}</td>`).join("")}</tr>`).join("")}
        </tbody>
      </table>
      <figcaption>Technical table</figcaption>
    </figure>
  `;
  }
  function isFigureLine(line) {
    return /^!\[.*?\]\(\S+?(?:\s+".*?")?\)$/.test(line);
  }
  function isTableLine(line) {
    return line.startsWith("|") && line.endsWith("|");
  }
  function summarizeText2(text, maxLength = 220) {
    const normalized = text.replace(/\s+/g, " ").trim();
    if (normalized.length <= maxLength) {
      return normalized;
    }
    return `${normalized.slice(0, maxLength - 1).trimEnd()}\u2026`;
  }
  function escapeHtml2(value) {
    return value.replace(/[\u0000-\u0008\u000B\u000C\u000E-\u001F\uFFFE\uFFFF]/g, "\uFFFD").replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;").replaceAll('"', "&quot;").replaceAll("'", "&#39;");
  }

  // src/features/products/lib/bookExportContentStyles.ts
  function renderBookExportContentStyles({ renderMode }) {
    return `
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
  `;
  }

  // src/features/products/lib/bookExportLayoutStyles.ts
  function renderBookExportLayoutStyles({
    trimPreset,
    renderMode
  }) {
    return `      :root {
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
        background: ${renderMode === "web" ? "radial-gradient(circle at top, rgba(255,255,255,0.7), transparent 36%), linear-gradient(180deg, #efe8db 0%, var(--paper) 100%)" : "#ffffff"};
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

      .book-sidebar-nav,
      .book-sidebar-node {
        display: flex;
        flex-direction: column;
      }

      .book-sidebar-nav { gap: 8px; }
      .book-sidebar-node { gap: 8px; }

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

      .page:first-child { border-top: none; }`;
  }

  // src/features/products/lib/bookExportPageStyles.ts
  function renderBookExportPageStyles() {
    return `
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
  `;
  }

  // src/features/products/lib/bookExportStyles.ts
  function renderBookExportStyles({ trimPreset, renderMode }) {
    return `${renderBookExportLayoutStyles({ trimPreset, renderMode })}
      ${renderBookExportContentStyles({ renderMode })}
      ${renderBookExportPageStyles()}`;
  }

  // src/features/products/lib/bookExportOptions.ts
  var BOOK_EXPORT_TRIM_PRESETS = [
    {
      id: "trade-6x9",
      label: "Trade Paperback (6\xD79)",
      description: "KDP-friendly trade paperback trim with balanced margins.",
      pageWidth: "6in",
      pageHeight: "9in",
      contentWidth: "5.8in",
      marginTop: "0.75in",
      marginRight: "0.72in",
      marginBottom: "0.85in",
      marginLeft: "0.82in"
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
      marginLeft: "20mm"
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
      marginLeft: "0.95in"
    }
  ];
  var DEFAULT_TRIM_PRESET_ID = "trade-6x9";
  var BOOK_PUBLISHING_DETAILS_ID = "book-publishing-details";
  var BOOK_CONTENTS_ID = "book-contents";
  var BOOK_REFERENCE_ATLAS_ID = "book-reference-atlas";
  var BOOK_NODE_INDEX_ID = "book-node-index";
  var BOOK_EXPORT_NOTES_ID = "book-export-notes";
  function getBookExportTrimPreset(preset) {
    if (preset && typeof preset !== "string") {
      return preset;
    }
    return BOOK_EXPORT_TRIM_PRESETS.find((candidate) => candidate.id === (preset ?? DEFAULT_TRIM_PRESET_ID)) ?? BOOK_EXPORT_TRIM_PRESETS[0];
  }

  // src/features/products/lib/bookExportToc.ts
  function renderBookContentsHtml2(tocTree) {
    return tocTree.map((node) => renderBookContentsNode(node)).join("");
  }
  function renderBookContentsNode(node) {
    return `
    <div class="toc-group">
      <div class="toc-${node.level === 0 ? "product_area" : "capability"}">
        <a class="inline-link" href="#${node.id}">${escapeHtml2(node.title)}</a>
        <span>${node.children.length > 0 ? `${node.children.length} sections` : ""}</span>
      </div>
      ${node.children.length > 0 ? `<div class="toc-children">${node.children.map((child) => renderBookContentsNode(child)).join("")}</div>` : ""}
    </div>
  `;
  }
  function collectNodeIndex(nodes) {
    const entries = [];
    const visit = (node) => {
      entries.push({
        id: getTreeNodeSectionId(node),
        title: node.name,
        pathLabel: node.path.join(" / "),
        kindLabel: getHierarchyNodeKindLabel(node.node_kind)
      });
      node.children.forEach(visit);
    };
    nodes.forEach(visit);
    return entries.sort((left, right) => left.title.localeCompare(right.title));
  }
  function buildBookTocItems(tree, hasProductLevelWorkItems, includeBackMatter) {
    const items = [
      { id: PRODUCT_OVERVIEW_TOP_ID, title: "Title Page", level: 0 },
      { id: BOOK_PUBLISHING_DETAILS_ID, title: "Publishing Details", level: 0 },
      { id: BOOK_CONTENTS_ID, title: "Contents", level: 0 }
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
  function appendBookHierarchyToc(items, nodes, prefix, level) {
    nodes.forEach((node, index) => {
      const numbering = prefix ? `${prefix}.${index + 1}` : `${index + 1}`;
      items.push({
        id: getTreeNodeSectionId(node),
        title: `${numbering}. ${node.name}`,
        level
      });
      appendBookHierarchyToc(items, node.children, numbering, level + 1);
    });
  }
  function getTreeNodeSectionId(node) {
    return node.node_type === "product_area" ? `product-area-${node.id}` : `capability-${node.capability_id ?? node.id}`;
  }
  function buildBookTocTree(items) {
    const roots = [];
    const stack = [];
    items.forEach((item) => {
      const node = { ...item, children: [] };
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
  function renderWebSidebar(productName, tocTree) {
    return `
    <aside class="book-sidebar">
      <div class="book-sidebar-inner">
        <div class="book-sidebar-kicker">Book Navigation</div>
        <div class="book-sidebar-title">${escapeHtml2(productName)}</div>
        <div class="book-sidebar-note">Jump through front matter, chapters, and back matter from a persistent outline rather than a centered single-column preview.</div>
        <nav class="book-sidebar-nav">
          ${tocTree.map((node) => renderWebSidebarNode(node)).join("")}
        </nav>
      </div>
    </aside>
  `;
  }
  function renderWebSidebarNode(node) {
    return `
    <div class="book-sidebar-node">
      <a class="book-sidebar-link" href="#${node.id}">${escapeHtml2(node.title)}</a>
      ${node.children.length > 0 ? `<div class="book-sidebar-children">${node.children.map((child) => renderWebSidebarNode(child)).join("")}</div>` : ""}
    </div>
  `;
  }

  // src/features/products/lib/bookExportReferences.ts
  function renderReferenceUri(uri, renderMode) {
    if (!uri) return "";
    const escaped = escapeHtml2(uri);
    const safeExternal = /^(?:https?:\/\/|mailto:|tel:)/i.test(uri) && !/[\u0000-\u0020\u007f]/.test(uri);
    const safeRelative = !/^(?:[a-z][a-z0-9+.-]*:|\/\/|\\)/i.test(uri) && !/[\u0000-\u0020\u007f]/.test(uri);
    return safeExternal || renderMode !== "epub" && safeRelative ? `<a class="reference-uri" href="${escaped}">${escaped}</a>` : `<span class="reference-uri">${escaped}</span>`;
  }
  function renderBookReferencesHtml(references, renderMode) {
    if (references.length === 0) {
      return "";
    }
    return `
    <div class="reference-list">
      ${references.map((reference) => `
        <div class="reference-item">
          <div class="meta-label">${escapeHtml2(getReferenceKindLabel(reference.reference_kind))}</div>
          <h3 style="margin-top: 6px;">${escapeHtml2(reference.title)}</h3>
          ${reference.content ? `<div class="note-copy">${renderRichTextHtml(reference.content)}</div>` : ""}
          ${renderReferenceUri(reference.uri, renderMode)}
        </div>
      `).join("")}
    </div>
  `;
  }
  function filterBookReferences(product, tree, references) {
    const scopeLabels = buildReferenceScopeLabels(product, tree);
    return references.filter((reference) => scopeLabels.has(getReferenceScopeKey(reference)));
  }
  function buildReferenceAtlas(product, tree, references) {
    const scopeLabels = buildReferenceScopeLabels(product, tree);
    return references.map((reference) => ({
      id: reference.id,
      kindLabel: getReferenceKindLabel(reference.reference_kind),
      pathLabel: scopeLabels.get(getReferenceScopeKey(reference)) ?? `${reference.scope_type} / ${reference.scope_id}`,
      title: reference.title,
      summary: reference.content,
      uri: reference.uri
    }));
  }
  function buildReferenceScopeLabels(product, tree) {
    const scopeLabels = /* @__PURE__ */ new Map();
    scopeLabels.set(`product:${product.id}`, product.name);
    const visit = (node) => {
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
  function getReferenceScopeKey(reference) {
    return `${reference.scope_type}:${reference.scope_id}`;
  }

  // src/features/products/lib/bookExportChapters.ts
  function renderBookProductAreaHtml2(productAreaTree, chapterNumber, allWorkItems, references, renderMode) {
    const productAreaScopedItems = allWorkItems.filter((workItem) => workItem.product_area_id === productAreaTree.product_area.id);
    const metrics = buildWorkItemMetrics(productAreaScopedItems);
    const directProductAreaWorkItems = buildScopedWorkItemTree(
      allWorkItems.filter((workItem) => workItem.product_area_id === productAreaTree.product_area.id && !workItem.capability_id)
    );
    const rootKindLabel = getHierarchyNodeKindLabel(productAreaTree.product_area.node_kind);
    const childCountLabel = productAreaTree.features.length === 1 ? "child node" : "child nodes";
    const productAreaReferences = filterReferencesForScope(references, getProductAreaReferenceScope(productAreaTree.product_area.id));
    return `
    <section class="page" id="${getProductAreaSectionId(productAreaTree.product_area)}">
      <div class="chapter-kicker">${escapeHtml2(rootKindLabel)} ${chapterNumber}</div>
      <h2 class="chapter-title">${escapeHtml2(productAreaTree.product_area.name)}</h2>
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
      ${renderBookReferencesHtml(productAreaReferences, renderMode)}
      ${directProductAreaWorkItems.length > 0 ? `
        <div class="section-block">
          <div class="section-kicker">Direct Delivery Notes</div>
          ${renderBookWorkItemList2(directProductAreaWorkItems)}
        </div>
      ` : ""}
      ${productAreaTree.features.length > 0 ? productAreaTree.features.map((capabilityTree, index) => renderBookCapabilityHtml(capabilityTree, `${chapterNumber}.${index + 1}`, allWorkItems, references, renderMode)).join("") : `<div class="section-block"><div class="body-copy">No child nodes are defined for this product area yet.</div></div>`}
      <div class="footer-note">End of chapter ${chapterNumber}.</div>
    </section>
  `;
  }
  function renderBookCapabilityHtml(capabilityTree, numbering, allWorkItems, references, renderMode) {
    const capabilityType = getHierarchyNodeKindLabel(capabilityTree.capability.node_kind);
    const directWorkItems = buildScopedWorkItemTree(
      allWorkItems.filter((workItem) => workItem.capability_id === capabilityTree.capability.id)
    );
    const capabilityReferences = filterReferencesForScope(references, getCapabilityReferenceScope(capabilityTree.capability));
    return `
    <section class="capability" id="${getCapabilitySectionId(capabilityTree.capability)}">
      <div class="section-kicker">${escapeHtml2(capabilityType)} ${escapeHtml2(numbering)}</div>
      <h3 class="capability-title">${escapeHtml2(capabilityTree.capability.name)}</h3>
      <div class="body-copy" style="margin-top: 10px;">${renderRichTextHtml(capabilityTree.capability.description || `This ${capabilityType.toLowerCase()} needs a fuller narrative in Aruvi Studio.`)}</div>
      <div class="capability-meta">
        <span class="meta-chip">${escapeHtml2(capabilityTree.capability.status.replace(/_/g, " "))}</span>
        <span class="meta-chip">${escapeHtml2(capabilityTree.capability.priority)} priority</span>
        <span class="meta-chip">${escapeHtml2(capabilityTree.capability.risk)} risk</span>
      </div>
      ${renderNoteBlock("Acceptance", capabilityTree.capability.acceptance_criteria)}
      ${renderNoteBlock("Explanation", capabilityTree.capability.explanation)}
      ${renderNoteBlock("Examples", capabilityTree.capability.examples)}
      ${renderNoteBlock("Technical Notes", capabilityTree.capability.technical_notes)}
      ${renderNoteBlock("Implementation Notes", capabilityTree.capability.implementation_notes)}
      ${renderNoteBlock("Test Guidance", capabilityTree.capability.test_guidance)}
      ${renderBookReferencesHtml(capabilityReferences, renderMode)}
      ${directWorkItems.length > 0 ? `
        <div class="section-block">
          <div class="section-kicker">Delivery Notes</div>
          ${renderBookWorkItemList2(directWorkItems)}
        </div>
      ` : ""}
      ${capabilityTree.children.length > 0 ? capabilityTree.children.map((child, index) => renderBookCapabilityHtml(child, `${numbering}.${index + 1}`, allWorkItems, references, renderMode)).join("") : ""}
    </section>
  `;
  }
  function renderBookWorkItemList2(nodes) {
    return `<div class="work-list">${nodes.map((node) => renderBookWorkItem(node)).join("")}</div>`;
  }
  function renderBookWorkItem(node) {
    const presentation = getWorkItemPresentation(node.workItem.status);
    const excerpt = summarizeText2(
      node.workItem.description || node.workItem.problem_statement || node.workItem.acceptance_criteria || "No delivery notes captured yet.",
      280
    );
    return `
    <div class="work-item">
      <div class="work-head">
        <div class="work-title">${escapeHtml2(node.workItem.title)}</div>
        <div class="work-status ${presentation.toneClass}">${presentation.label}</div>
      </div>
      <div class="work-copy">${escapeHtml2(excerpt)}</div>
      ${node.children.length > 0 ? `<div class="child-work">${renderBookWorkItemList2(node.children)}</div>` : ""}
    </div>
  `;
  }
  function renderNoteBlock(label, text) {
    if (!text.trim()) {
      return "";
    }
    return `
    <div class="note-block">
      <div class="note-label">${escapeHtml2(label)}</div>
      <div class="note-copy">${renderRichTextHtml(text)}</div>
    </div>
  `;
  }

  // src/features/products/lib/bookExport.ts
  function buildProductOverviewBookBundle({
    product,
    tree,
    workItems = [],
    references = []
  }, options = {}) {
    const trimPreset = getBookExportTrimPreset(options.trimPreset);
    const renderMode = options.renderMode ?? "web";
    const includeFrontMatter = options.includeFrontMatter ?? true;
    const includeBackMatter = options.includeBackMatter ?? true;
    const allWorkItems = sortWorkItems(workItems);
    const metrics = buildWorkItemMetrics(allWorkItems);
    const rootSectionCount = tree?.roots.length ?? 0;
    const totalNodeCount = tree ? countHierarchyNodes(tree.roots) : 0;
    const productLevelWorkItems = buildScopedWorkItemTree(getProductDirectWorkItems(allWorkItems));
    const generatedAt = (/* @__PURE__ */ new Date()).toLocaleString();
    const tocItems = buildBookTocItems(
      tree,
      productLevelWorkItems.length > 0,
      includeBackMatter
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
    <title>${escapeHtml2(product.name)} - Book</title>
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
        renderMode
      }) : ""}
          ${productLevelWorkItems.length > 0 ? `
            <section class="page" id="${PRODUCT_DELIVERY_ID}">
              <div class="chapter-kicker">Prelude</div>
              <h2 class="chapter-title">Product Delivery Themes</h2>
              <div class="chapter-intro">Cross-cutting work attached directly to the book, presented as implementation themes rather than ticket-level execution details.</div>
              ${renderBookWorkItemList2(productLevelWorkItems)}
            </section>
          ` : ""}
          ${(tree?.product_areas ?? []).length > 0 ? (tree?.product_areas ?? []).map((productAreaTree, index) => renderBookProductAreaHtml2(productAreaTree, index + 1, allWorkItems, bookReferences, renderMode)).join("") : `
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
        renderMode
      }) : ""}
        </article>
      </main>
    </div>
  </body>
</html>`
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
    renderMode
  }) {
    const productReferences = filterReferencesForScope(references, { scopeType: "product", scopeId: product.id });
    return `
    <section class="page title-page" id="${PRODUCT_OVERVIEW_TOP_ID}">
      <div class="kicker">Aruvi Studio Book</div>
      <h1>${escapeHtml2(product.name)}</h1>
      <div class="deck">${renderRichTextHtml(product.description || "A durable product narrative generated from Aruvi Studio.")}</div>
      <div class="book-meta">
        <div class="meta-item"><strong>Product Areas</strong>${rootSectionCount}</div>
        <div class="meta-item"><strong>Total Nodes</strong>${totalNodeCount}</div>
        <div class="meta-item"><strong>References</strong>${references.length}</div>
        <div class="meta-item"><strong>Delivery</strong>${metrics.done} done, ${metrics.wip} active, ${metrics.tbd} planned</div>
        <div class="meta-item"><strong>Generated</strong>${escapeHtml2(generatedAt)}</div>
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
          <div class="index-copy">Prepared on ${escapeHtml2(generatedAt)} from the current live product structure and attached delivery stories.</div>
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
      ${renderBookReferencesHtml(productReferences, renderMode)}
      <div class="toc-list">
        ${renderBookContentsHtml2(tocTree)}
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
    renderMode
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
              <div class="meta-label">${escapeHtml2(entry.kindLabel)}</div>
              <h3 style="margin-top: 6px;">${escapeHtml2(entry.title)}</h3>
              <div class="reference-path">${escapeHtml2(entry.pathLabel)}</div>
              <div class="note-copy">${renderRichTextHtml(entry.summary || "No summary recorded yet.")}</div>
              ${renderReferenceUri(entry.uri, renderMode)}
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
            <a href="#${entry.id}" class="inline-link"><strong>${escapeHtml2(entry.title)}</strong></a>
            <div class="index-path">${escapeHtml2(entry.kindLabel)} \xB7 ${escapeHtml2(entry.pathLabel)}</div>
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
          <div class="index-copy">${escapeHtml2(trimPreset.label)}. ${escapeHtml2(trimPreset.description)}</div>
        </div>
        <div class="panel">
          <h3>Delivery Snapshot</h3>
          <div class="index-copy">${metrics.done} done, ${metrics.wip} active, ${metrics.tbd} planned, ${metrics.blocked} blocked.</div>
        </div>
        <div class="panel">
          <h3>Catalog Snapshot</h3>
          <div class="index-copy">${escapeHtml2(product.name)} exported with ${totalNodeCount} semantic nodes on ${escapeHtml2(generatedAt)}.</div>
        </div>
      </div>
    </section>
  `;
  }

  // src/features/products/lib/productBookBackend.ts
  function renderJson(requestJson) {
    const request = JSON.parse(requestJson);
    const { tree, work_items: workItems, references } = request.source;
    const input = { product: tree.product, tree, workItems, references };
    if (request.format === "docs") return JSON.stringify({ html: buildProductOverviewHtml(input) });
    if (request.format === "book") return JSON.stringify(buildProductOverviewBookBundle(input, {
      renderMode: request.renderMode,
      trimPreset: request.trimPreset
    }));
    throw new Error("Unsupported product book format");
  }
  return __toCommonJS(productBookBackend_exports);
})();
