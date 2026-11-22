"use strict";
var AruviPlannerPacket = (() => {
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

  // src/features/planner/lib/plannerPacketBackend.ts
  var plannerPacketBackend_exports = {};
  __export(plannerPacketBackend_exports, {
    renderJson: () => renderJson
  });

  // src/features/planner/lib/plannerActionSummary.ts
  function summarizeAction(action) {
    if (!action || typeof action !== "object") {
      return {
        symbol: "?",
        tone: "warn",
        title: "Unknown planner action",
        detail: "The planner returned an empty or invalid action payload."
      };
    }
    const raw = action;
    const actionType = typeof action.type === "string" ? String(action.type) : "unknown_action";
    const target = raw.target;
    const name = typeof raw.name === "string" ? raw.name : void 0;
    const title = typeof raw.title === "string" ? raw.title : void 0;
    const description = typeof raw.description === "string" ? raw.description : void 0;
    const vision = typeof raw.vision === "string" ? raw.vision : void 0;
    const fields = raw.fields ?? void 0;
    switch (actionType) {
      case "create_product_area":
        return { symbol: "+", tone: "add", title: `Create product area ${name ?? target?.productAreaName ?? "unnamed product area"}`, detail: target?.productName ? `Product: ${target.productName}` : "Attach to selected product." };
      case "create_capability":
        return { symbol: "+", tone: "add", title: `Create capability ${name ?? target?.capabilityName ?? "unnamed capability"}`, detail: [target?.productName, target?.productAreaName].filter(Boolean).join(" / ") || "Attach to selected scope." };
      case "apply_capability_template":
        return {
          symbol: "+",
          tone: "add",
          title: `Apply template ${String(raw.templateKind ?? "chapter")} to ${name ?? "unnamed topic"}`,
          detail: [target?.productName, target?.productAreaName, target?.capabilityName].filter(Boolean).join(" / ") || description || "Create a product-design scaffold."
        };
      case "convert_capability_kind":
        return {
          symbol: "~",
          tone: "update",
          title: `Convert capability ${target?.capabilityName ?? ""}`.trim(),
          detail: `nodeKind=${String(raw.nodeKind ?? "unknown")} childStrategy=${String(raw.childStrategy ?? "reject")}`
        };
      case "create_work_item":
        return { symbol: "+", tone: "add", title: `Create story/task ${title ?? target?.workItemTitle ?? "untitled work item"}`, detail: [target?.productName, target?.productAreaName, target?.capabilityName].filter(Boolean).join(" / ") || description || "New delivery work proposal." };
      case "update_product":
        return { symbol: "~", tone: "update", title: `Update product ${target?.productName ?? ""}`.trim(), detail: JSON.stringify(fields, null, 2) };
      case "update_product_area":
        return { symbol: "~", tone: "update", title: `Update product area ${target?.productAreaName ?? ""}`.trim(), detail: JSON.stringify(fields, null, 2) };
      case "update_capability":
        return { symbol: "~", tone: "update", title: `Update capability ${target?.capabilityName ?? ""}`.trim(), detail: JSON.stringify(fields, null, 2) };
      case "update_work_item":
        return { symbol: "~", tone: "update", title: `Update story/task ${target?.workItemTitle ?? ""}`.trim(), detail: JSON.stringify(fields, null, 2) };
      case "approve_work_item":
      case "approve_work_item_plan":
      case "approve_work_item_test_review":
      case "start_workflow":
      case "workflow_action":
      case "reject_work_item":
      case "reject_work_item_plan":
      case "archive_product":
      case "delete_product_area":
      case "delete_capability":
      case "delete_work_item":
        return { symbol: "!", tone: "warn", title: actionType.replace(/_/g, " "), detail: JSON.stringify(action, null, 2) };
      case "report_status":
        return { symbol: "i", tone: "update", title: "Status report", detail: target?.productName || target?.workItemTitle || "Current scope" };
      case "report_tree":
        return { symbol: "i", tone: "update", title: "Tree report", detail: target?.productName || "All products" };
      default:
        return {
          symbol: "?",
          tone: "warn",
          title: actionType.replace(/_/g, " "),
          detail: JSON.stringify(action, null, 2)
        };
    }
  }

  // src/features/planner/lib/plannerReviewPacket.ts
  function buildDesignReviewPacketHtml(input) {
    const actionSummaries = (input.plan?.actions ?? []).map((action) => summarizeAction(action));
    const rootNames = input.draftTreeNodes.map((node) => node.label);
    const featureActions = actionSummaries.filter(
      (summary) => /create|update|apply|convert/i.test(summary.title)
    );
    const workActions = actionSummaries.filter(
      (summary) => /work item|task/i.test(summary.title)
    );
    const riskItems = [
      ...input.validation.issues.filter((issue) => issue.tone === "warn").map((issue) => `${issue.title}: ${issue.detail}`),
      ...input.plan?.clarification_question ? [`Open question: ${input.plan.clarification_question}`] : []
    ];
    const changeSetJson = JSON.stringify(
      {
        generatedAt: input.generatedAt,
        title: input.title,
        selectedNode: input.selectedNode?.label ?? null,
        draftTree: input.draftTreeNodes,
        actions: input.plan?.actions ?? [],
        validation: input.validation
      },
      null,
      2
    );
    return `<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>${escapeHtml(input.title)} - Design Review Packet</title>
    <script src="https://cdn.jsdelivr.net/npm/mermaid@10/dist/mermaid.min.js"></script>
    <script>window.addEventListener("DOMContentLoaded", function () { if (window.mermaid) mermaid.initialize({ startOnLoad: true, theme: "base" }); });</script>
    <style>
      :root {
        color-scheme: light;
        --bg: #f8fafc;
        --panel: #ffffff;
        --ink: #111827;
        --muted: #64748b;
        --border: #d8dee8;
        --accent: #2563eb;
        --accent-soft: #eff6ff;
        --ok: #15803d;
        --warn: #a16207;
        --danger: #b91c1c;
      }
      * { box-sizing: border-box; }
      body { margin: 0; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; background: var(--bg); color: var(--ink); line-height: 1.55; }
      .shell { max-width: 1180px; margin: 0 auto; padding: 28px; }
      .hero { background: linear-gradient(145deg, #eff6ff, #ffffff 64%); border: 1px solid #bfdbfe; border-radius: 16px; padding: 24px; margin-bottom: 18px; }
      .eyebrow { color: var(--accent); font-size: 12px; font-weight: 800; letter-spacing: .12em; text-transform: uppercase; }
      h1 { margin: 8px 0 10px; font-size: 34px; line-height: 1.05; }
      h2 { margin: 0 0 12px; font-size: 20px; }
      h3 { margin: 16px 0 8px; font-size: 15px; }
      p { margin: 0 0 10px; }
      .meta { color: var(--muted); font-size: 13px; }
      .grid { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 10px; margin-top: 16px; }
      .metric { background: var(--panel); border: 1px solid var(--border); border-radius: 12px; padding: 12px; }
      .metric-label { color: var(--muted); font-size: 11px; font-weight: 800; letter-spacing: .08em; text-transform: uppercase; }
      .metric-value { font-size: 24px; font-weight: 900; margin-top: 4px; }
      .section { background: var(--panel); border: 1px solid var(--border); border-radius: 14px; padding: 18px; margin: 14px 0; }
      .toc { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 8px; margin-top: 16px; }
      .toc a { color: #1e3a8a; background: #f8fafc; border: 1px solid var(--border); border-radius: 10px; padding: 8px 10px; text-decoration: none; font-weight: 700; font-size: 13px; }
      ul { margin: 8px 0 0 20px; padding: 0; }
      li { margin: 5px 0; }
      .badge { display: inline-flex; border-radius: 999px; padding: 4px 8px; background: var(--accent-soft); color: #1d4ed8; font-size: 12px; font-weight: 800; margin: 2px 4px 2px 0; }
      .diff { border-top: 1px solid #e2e8f0; padding: 10px 0; display: grid; grid-template-columns: 32px minmax(0, 1fr); gap: 10px; }
      .symbol { font-weight: 900; color: var(--ok); }
      .warn { color: var(--warn); }
      .danger { color: var(--danger); }
      .diagram { background: #ffffff; border: 1px solid var(--border); border-radius: 12px; padding: 12px; overflow: auto; }
      pre { white-space: pre-wrap; word-break: break-word; background: #0f172a; color: #e2e8f0; border-radius: 12px; padding: 14px; overflow: auto; }
      .approval { border-color: #bfdbfe; background: #eff6ff; }
      @media print { body { background: white; } .shell { max-width: none; padding: 0; } .section, .hero { break-inside: avoid; } }
    </style>
  </head>
  <body>
    <main class="shell">
      <section class="hero">
        <div class="eyebrow">Design Review Packet</div>
        <h1>${escapeHtml(input.title)}</h1>
        <p class="meta">Generated ${escapeHtml(input.generatedAt)}${input.activeProductName ? ` \xB7 Active product: ${escapeHtml(input.activeProductName)}` : ""}</p>
        <div class="grid">
          <div class="metric"><div class="metric-label">Current Products</div><div class="metric-value">${input.currentProducts.length}</div></div>
          <div class="metric"><div class="metric-label">Draft Roots</div><div class="metric-value">${input.draftTreeNodes.length}</div></div>
          <div class="metric"><div class="metric-label">Proposed Changes</div><div class="metric-value">${input.plan?.actions.length ?? 0}</div></div>
          <div class="metric"><div class="metric-label">Readiness</div><div class="metric-value">${input.validation.score}</div></div>
        </div>
        <nav class="toc">
          ${[
      "Executive Summary",
      "Current State",
      "Proposed Architecture",
      "Change Diff",
      "Feature Specification",
      "UX / Design Proposal",
      "Implementation Plan",
      "Risk Review",
      "Work Breakdown",
      "Approval Section"
    ].map((label, index) => `<a href="#section-${index + 1}">${index + 1}. ${label}</a>`).join("")}
        </nav>
      </section>

      ${packetSection(1, "Executive Summary", [
      paragraph(input.latestAssistantText || input.plan?.assistant_response || "This packet captures the proposed product design before the catalog is changed."),
      list([
        rootNames.length > 0 ? `Primary proposed structure: ${rootNames.join(", ")}.` : "No staged structure is currently available.",
        `The packet includes ${input.plan?.actions.length ?? 0} proposed catalog operation(s).`,
        "No catalog changes should be applied until this packet is reviewed and approved."
      ])
    ])}

      ${packetSection(2, "Current State", [
      list([
        `${input.currentProducts.length} product(s) exist in the current workspace.`,
        `${input.currentProductTrees.reduce((total, tree) => total + tree.roots.length, 0)} root section(s) are loaded across current products.`,
        input.currentWorkItemsHasMore ? `The planner context is capped at the first ${input.currentWorkItems.length} story/task item(s); use Work Items for full paged delivery browsing.` : `${input.currentWorkItems.length} story/task item(s) are currently visible to the planner.`,
        input.activeProductName ? `Current active product context: ${input.activeProductName}.` : "No active product context was selected."
      ])
    ])}

      ${packetSection(3, "Proposed Architecture", [
      `<div class="diagram"><pre class="mermaid">${escapeHtml(buildMermaidDiagram(input.draftTreeNodes))}</pre></div>`,
      paragraph("The architecture diagram is generated from the staged design tree. Use it to inspect hierarchy, ownership, and major boundaries before applying changes.")
    ])}

      ${packetSection(4, "Change Diff", [
      actionSummaries.length > 0 ? actionSummaries.map((summary) => `<div class="diff"><div class="symbol">${escapeHtml(summary.symbol)}</div><div><strong>${escapeHtml(summary.title)}</strong>${summary.detail ? `<div class="meta">${escapeHtml(summary.detail)}</div>` : ""}</div></div>`).join("") : paragraph("No structured change actions are currently available.")
    ])}

      ${packetSection(5, "Feature Specification", [
      list([
        ...featureActions.map((summary) => summary.title),
        featureActions.length === 0 ? "No explicit feature additions or modifications were found in the latest plan." : ""
      ].filter((value) => Boolean(value))),
      paragraph("Each feature should be reviewed for user value, acceptance criteria, edge cases, and whether it belongs in this product boundary.")
    ])}

      ${packetSection(6, "UX / Design Proposal", [
      list([
        "Identify the primary user workflows affected by this design.",
        "Review first-screen information hierarchy, navigation, empty states, loading states, error states, success states, and conflict states.",
        "Check whether the proposed screens are operationally useful, not just visually complete.",
        "Confirm accessibility expectations: keyboard access, readable contrast, clear focus states, and screen-reader labels."
      ])
    ])}

      ${packetSection(7, "Implementation Plan", [
      list([
        "Phase 1: Apply the approved product area/capability/feature structure.",
        "Phase 2: Generate or refine stories and tasks with acceptance criteria and dependencies.",
        "Phase 3: Build UI/data/API changes behind reviewable stories and tasks.",
        "Phase 4: Validate with focused tests, walkthroughs, and user review before release."
      ])
    ])}

      ${packetSection(8, "Risk Review", [
      riskItems.length > 0 ? list(riskItems) : list([
        "No blocking structural risks were detected by the current validation pass.",
        "Review assumptions manually before applying the design."
      ])
    ])}

      ${packetSection(9, "Work Breakdown", [
      workActions.length > 0 ? list(workActions.map((summary) => summary.title)) : list([
        "Convert approved features into implementation-ready stories and tasks.",
        "Add priorities, dependencies, complexity, and acceptance criteria before delivery."
      ])
    ])}

      ${packetSection(10, "Approval Section", [
      `<div class="section approval">`,
      `<h3>Recommended approval options</h3>`,
      list([
        "Approve all proposed changes and apply them to the product catalog.",
        "Approve selected changes only.",
        "Request revision and regenerate this packet.",
        "Export/share this packet for stakeholder review.",
        "Archive this packet without applying changes."
      ]),
      `<h3>Structured change set</h3>`,
      `<pre>${escapeHtml(changeSetJson)}</pre>`,
      `</div>`
    ])}
    </main>
  </body>
</html>`;
  }
  function packetSection(index, title, body) {
    return `<section id="section-${index}" class="section"><h2>${index}. ${escapeHtml(title)}</h2>${body.join("\n")}</section>`;
  }
  function paragraph(value) {
    return `<p>${escapeHtml(value)}</p>`;
  }
  function list(items) {
    return `<ul>${items.map((item) => `<li>${escapeHtml(item)}</li>`).join("")}</ul>`;
  }
  function buildMermaidDiagram(nodes) {
    if (nodes.length === 0) {
      return "flowchart TD\n  Empty[No staged design yet]";
    }
    const lines = ["flowchart TD"];
    const seen = /* @__PURE__ */ new Set();
    const walk = (node, parentId) => {
      const nodeId = mermaidNodeId(node.id);
      if (!seen.has(nodeId)) {
        lines.push(`  ${nodeId}["${escapeMermaidLabel(node.label)}"]`);
        seen.add(nodeId);
      }
      if (parentId) {
        lines.push(`  ${parentId} --> ${nodeId}`);
      }
      node.children.forEach((child) => walk(child, nodeId));
    };
    nodes.forEach((node) => walk(node));
    return lines.join("\n");
  }
  function mermaidNodeId(value) {
    const cleaned = value.replace(/[^a-zA-Z0-9_]/g, "_");
    return /^[a-zA-Z_]/.test(cleaned) ? cleaned : `node_${cleaned}`;
  }
  function escapeMermaidLabel(value) {
    return value.replace(/"/g, "'").replace(/\n/g, " ");
  }
  function escapeHtml(value) {
    return String(value ?? "").replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;").replace(/'/g, "&#39;");
  }
  function slugifyPacketName(value) {
    return value.toLowerCase().trim().replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "") || "design-review-packet";
  }

  // src/features/planner/lib/plannerPacketBackend.ts
  function renderJson(requestJson) {
    const input = JSON.parse(requestJson);
    return JSON.stringify({
      filename: `${slugifyPacketName(input.title)}-design-review-packet.html`,
      html: buildDesignReviewPacketHtml(input).replace(
        '<main class="shell">',
        input.snapshotKind === "proposal" ? '<main class="shell"><p role="note">This packet uses a caller-supplied proposal snapshot. The proposal and any recorded draft are from the selected message; catalog context was gathered at export time and may have changed.</p>' : '<main class="shell"><p role="note">This packet uses a caller-supplied snapshot, which may be stale and has not been checked against the current catalog.</p>'
      )
    });
  }
  return __toCommonJS(plannerPacketBackend_exports);
})();
